use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use anyhow::{bail, Result};
use aws_config::BehaviorVersion;
use aws_sdk_eks::{
  config::{self, retry::RetryConfig},
  Client,
};
use ipnet::{IpNet, Ipv4Net};
use tracing::{debug, info};

use crate::{commands::join::JoinClusterInput, IpvFamily};

/// Get the EKS client
async fn get_client() -> Result<Client> {
  let config = aws_config::load_defaults(BehaviorVersion::v2023_11_09()).await;
  let client = Client::from_conf(
    // Start with the shared environment configuration
    config::Builder::from(&config)
      // Set max attempts
      .retry_config(RetryConfig::standard().with_max_attempts(3))
      .build(),
  );
  Ok(client)
}

/// Describe the cluster to extract the relevant details to join the cluster
async fn describe_cluster(client: &Client, name: &str) -> Result<aws_sdk_eks::types::Cluster> {
  let request = client.describe_cluster().name(name);
  let response = request.send().await?;

  Ok(response.cluster.expect("Cluster not found"))
}

/// Given an IPv4 address, return its x.x.x.10 address
fn ipv4_dns_ip_address(addr: Ipv4Addr) -> Result<Ipv4Addr> {
  let mut octets = addr.octets();
  if let Some(last) = octets.last_mut() {
    *last = 10;
  }
  Ok(Ipv4Addr::from(octets))
}

/// Given an IPv6 address, return its :::a address
fn ipv6_dns_ip_address(addr: Ipv6Addr) -> Result<Ipv6Addr> {
  let mut segments = addr.segments();
  if let Some(last) = segments.last_mut() {
    *last = u16::from_str_radix("a", 16)?;
  }
  Ok(Ipv6Addr::from(segments))
}

/// Derive the IP address of the cluster DNS server
///
/// When --ip-family ipv4 (default):
/// - If --service-cidr is supplied, return x.x.x.10 address from the CIDR
/// - If --service-cidr is not supplied, return x.x.x.10 address from the instance metadata
///   - Querying IMDS vpc-ipv4-cidr-blocks, if 10.x.x.x/x net is found, use 10.100.0.10 otherwise 172.20.0.10 is used
///
/// When --ip-family ipv6:
/// --service-cidr is required, return :::a address from the CIDR
fn derive_cluster_dns_ip(
  service_cidr: &Option<IpNet>,
  ip_family: &IpvFamily,
  vpc_ipv4_cidr_blocks: &[Ipv4Net],
) -> Result<IpAddr> {
  match service_cidr {
    Some(cidr) => match cidr.network() {
      IpAddr::V4(addr) => {
        let result = ipv4_dns_ip_address(addr)?;
        println!("{result}");
        Ok(IpAddr::V4(result))
      }
      IpAddr::V6(addr) => {
        let result = ipv6_dns_ip_address(addr)?;
        println!("{result}");
        Ok(IpAddr::V6(result))
      }
    },

    None => match ip_family {
      IpvFamily::Ipv4 => {
        let mut result = None;
        for cidr in vpc_ipv4_cidr_blocks {
          if cidr.addr().octets().first().unwrap_or(&192).eq(&10) {
            result = Some(Ipv4Addr::new(172, 20, 0, 10));
            break;
          }
        }
        if result.is_none() {
          result = Some(Ipv4Addr::new(10, 100, 0, 10));
        }
        Ok(IpAddr::V4(result.unwrap()))
      }
      IpvFamily::Ipv6 => bail!("--ip-family ipv6 requires --service-cidr to be supplied"),
    },
  }
}

/// EKS cluster details required to join a node to the cluster
#[derive(Debug)]
pub struct Cluster {
  /// Name of the cluster
  pub name: String,
  /// Cluster API server endpoint
  pub endpoint: String,
  /// Base64 encoded certificate data
  pub b64_ca: String,
  /// Identifies if the control plane is deployed on Outpost
  pub is_local_cluster: bool,
  /// Cluster DNS IP address
  pub cluster_dns_ip: IpAddr,
}

/// Return the cluster details from the input collected
fn collect_cluster(node: &JoinClusterInput, cluster_dns_ip: IpAddr) -> Result<Option<Cluster>> {
  if let Some(endpoint) = node.apiserver_endpoint.to_owned() {
    if let Some(b64_ca) = node.b64_cluster_ca.to_owned() {
      return Ok(Some(Cluster {
        name: node.cluster_name.to_owned(),
        endpoint,
        b64_ca,
        is_local_cluster: node.is_local_cluster,
        cluster_dns_ip,
      }));
    }
  }

  Ok(None)
}

/// Extract cluster details from CLI input, or get directly from cluster
///
/// If all the necessary details required to join a node to the cluster are provided, then
/// we can save an API call. Otherwise, we need to describe the cluster to get the details.
pub async fn collect_or_get_cluster(node: &JoinClusterInput, vpc_ipv4_cidr_blocks: &[Ipv4Net]) -> Result<Cluster> {
  // DNS cluster IP is not related to cluster - if it cannot be derived, it should fail
  let cluster_dns_ip = match node.cluster_dns_ip {
    Some(ip) => ip,
    None => derive_cluster_dns_ip(&node.service_cidr, &node.ip_family, vpc_ipv4_cidr_blocks)?,
  };
  info!("DNS cluster IP address: {}", cluster_dns_ip);

  let cluster_name = &node.cluster_name.clone();

  match collect_cluster(node, cluster_dns_ip)? {
    Some(cluster) => {
      debug!("Cluster details collected from CLI input - no describe API call required");
      Ok(cluster)
    }
    None => {
      debug!("Insufficient cluster details - describing cluster to get details");

      let client = get_client().await?;
      let describe = describe_cluster(&client, cluster_name).await?;

      Ok(Cluster {
        name: describe.name.unwrap(),
        endpoint: describe.endpoint.unwrap(),
        b64_ca: describe.certificate_authority.unwrap().data.unwrap(),
        is_local_cluster: describe.outpost_config.is_some(),
        cluster_dns_ip,
      })
    }
  }
}

/// Addon version is relative to a given Kubernetes version
#[derive(Debug)]
pub struct AddonVersion {
  /// Latest supported version of the addon
  pub latest: String,
  /// Default version of the addon
  pub default: String,
}

/// Get the addon version details for the given addon and Kubernetes version
///
/// Returns the default version and latest version of the addon for the given Kubernetes version
pub async fn get_addon_versions(name: &str, kubernetes_version: &str) -> Result<AddonVersion> {
  let client = get_client().await?;

  // Get all of the addon versions supported for the given addon and Kubernetes version
  let describe = client
    .describe_addon_versions()
    .addon_name(name)
    .kubernetes_version(kubernetes_version)
    .send()
    .await?;

  // Since we are providing an addon name, we are only concerned with the first and only item
  let addon = describe
    .addons()
    .first()
    .expect("describe addons failed to return results");
  let latest_version = match addon.addon_versions().first() {
    Some(version) => version.addon_version().unwrap_or_default(),
    None => {
      bail!("Version not found for addon {name}");
    }
  };

  // The default version as specified by the EKS API for a given addon and Kubernetes version
  let default_version = addon
    .addon_versions()
    .iter()
    .filter(|v| v.compatibilities().iter().any(|c| c.default_version))
    .map(|v| v.addon_version().unwrap_or_default())
    .next()
    .unwrap_or_default();

  Ok(AddonVersion {
    latest: latest_version.to_owned(),
    default: default_version.to_owned(),
  })
}

#[cfg(test)]
mod tests {
  use ipnet::Ipv6Net;
  use rstest::*;

  use super::*;

  #[rstest]
  #[case(Ipv4Addr::new(10, 0, 0, 1), Ipv4Addr::new(10, 0, 0, 10))]
  #[case(Ipv4Addr::new(10, 100, 12, 192), Ipv4Addr::new(10, 100, 12, 10))]
  #[case(Ipv4Addr::new(192, 168, 12, 34), Ipv4Addr::new(192, 168, 12, 10))]
  #[case(Ipv4Addr::new(172, 16, 123, 133), Ipv4Addr::new(172, 16, 123, 10))]
  fn ipv4_dns_ip_address_test(#[case] addr: Ipv4Addr, #[case] expected: Ipv4Addr) {
    let result = ipv4_dns_ip_address(addr).unwrap();
    assert_eq!(expected, result);
  }

  #[rstest]
  #[case("fd00::".parse::<Ipv6Addr>().unwrap(), "fd00::a".parse::<Ipv6Addr>().unwrap())]
  #[case("fd00:1234:5678::".parse::<Ipv6Addr>().unwrap(), "fd00:1234:5678::a".parse::<Ipv6Addr>().unwrap())]
  #[case("2001:db8:8:4::2".parse::<Ipv6Addr>().unwrap(), "2001:db8:8:4::a".parse::<Ipv6Addr>().unwrap())]
  #[case("2001:db8:85a3:8d3:1319:8a2e:370:7348".parse::<Ipv6Addr>().unwrap(), "2001:db8:85a3:8d3:1319:8a2e:370:a".parse::<Ipv6Addr>().unwrap())]
  fn ipv6_dns_ip_address_test(#[case] addr: Ipv6Addr, #[case] expected: Ipv6Addr) {
    let result = ipv6_dns_ip_address(addr).unwrap();
    assert_eq!(expected, result);
  }

  #[rstest]
  // Service CIDR provided - IPv4
  #[case(Some(IpNet::V4("10.1.0.0/24".parse::<Ipv4Net>().unwrap())), &IpvFamily::Ipv4, &[], IpAddr::V4(Ipv4Addr::new(10, 1, 0, 10)))]
  #[case(Some(IpNet::V4("10.100.0.0/16".parse::<Ipv4Net>().unwrap())), &IpvFamily::Ipv4, &[], IpAddr::V4(Ipv4Addr::new(10, 100, 0, 10)))]
  #[case(Some(IpNet::V4("192.168.8.0/24".parse::<Ipv4Net>().unwrap())), &IpvFamily::Ipv4, &[], IpAddr::V4(Ipv4Addr::new(192, 168, 8, 10)))]
  #[case(Some(IpNet::V4("172.16.123.0/24".parse::<Ipv4Net>().unwrap())), &IpvFamily::Ipv4, &[], IpAddr::V4(Ipv4Addr::new(172, 16, 123, 10)))]
  // Service CIDR provided - IPv6
  #[case(Some(IpNet::V6("fd00::/18".parse::<Ipv6Net>().unwrap())), &IpvFamily::Ipv6, &[], IpAddr::V6("fd00::a".parse::<Ipv6Addr>().unwrap()))]
  #[case(Some(IpNet::V6("fd00:1234:5678::/62".parse::<Ipv6Net>().unwrap())), &IpvFamily::Ipv6, &[], IpAddr::V6("fd00:1234:5678::a".parse::<Ipv6Addr>().unwrap()))]
  #[case(Some(IpNet::V6("2001:db8:8:4::2/62".parse::<Ipv6Net>().unwrap())), &IpvFamily::Ipv6, &[], IpAddr::V6("2001:db8:8:4::a".parse::<Ipv6Addr>().unwrap()))]
  #[case(Some(IpNet::V6("2001:db8:85a3:8d3:1319:8a2e:370:7348/126".parse::<Ipv6Net>().unwrap())), &IpvFamily::Ipv6, &[], IpAddr::V6("2001:db8:85a3:8d3:1319:8a2e:370:a".parse::<Ipv6Addr>().unwrap()))]
  // Service CIDR NOT provided - IPv4
  #[case(None, &IpvFamily::Ipv4, &["10.1.0.0/24".parse::<Ipv4Net>().unwrap()], IpAddr::V4(Ipv4Addr::new(172, 20, 0, 10)))]
  #[case(None, &IpvFamily::Ipv4, &["192.168.8.0/24".parse::<Ipv4Net>().unwrap(), "10.100.0.0/16".parse::<Ipv4Net>().unwrap()], IpAddr::V4(Ipv4Addr::new(172, 20, 0, 10)))]
  #[case(None, &IpvFamily::Ipv4, &["192.168.8.0/24".parse::<Ipv4Net>().unwrap()], IpAddr::V4(Ipv4Addr::new(10, 100, 0, 10)))]
  #[case(None, &IpvFamily::Ipv4, &["172.16.123.0/24".parse::<Ipv4Net>().unwrap()],  IpAddr::V4(Ipv4Addr::new(10, 100, 0, 10)))]
  // --service-cidr required when --ip-family is ipv4
  #[should_panic]
  #[case(None, &IpvFamily::Ipv6, &[], IpAddr::V6("fd00::a".parse::<Ipv6Addr>().unwrap()))]
  fn derive_cluster_dns_ip_test(
    #[case] service_cidr: Option<IpNet>,
    #[case] ip_family: &IpvFamily,
    #[case] vpc_ipv4_cidr_blocks: &[Ipv4Net],
    #[case] expected: IpAddr,
  ) {
    let result = derive_cluster_dns_ip(&service_cidr, ip_family, vpc_ipv4_cidr_blocks).unwrap();
    assert_eq!(expected, result);
  }
}
