use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use anyhow::{bail, Result};
use aws_config::SdkConfig;
use aws_sdk_eks::{
  config::{self, retry::RetryConfig},
  types::Cluster,
  Client,
};
use ipnet::{IpNet, Ipv4Net};
use tracing::{debug, info};

use crate::cli::{Bootstrap, IpvFamily};

/// Get the EKS client
async fn get_client(config: SdkConfig, retries: u32) -> Result<Client> {
  let client = Client::from_conf(
    // Start with the shared environment configuration
    config::Builder::from(&config)
      // Set max attempts
      .retry_config(RetryConfig::standard().with_max_attempts(retries))
      .build(),
  );
  Ok(client)
}

/// Describe the cluster to extract the relevant details for bootstrapping
async fn describe_cluster(client: &Client, name: &str) -> Result<Cluster> {
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

/// Derive the IP address to use for DNS queries within the cluster
///
/// When --ip-family ipv4 (default):
/// - If --service-cidr is supplied, return x.x.x.10 address from the CIDR
/// - If --service-cidr is not supplied, return x.x.x.10 address from the instance metadata
///   - Querying IMDS vpc-ipv4-cidr-blocks, if 10.x.x.x/x net is found, use 10.100.0.10
///     otherwise 172.20.0.10 is used
///
/// When --ip-family ipv6:
/// --service-cidr is required, return :::a address from the CIDR
fn derive_dns_cluster_ip(
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
        let ten_net = Ipv4Net::new(Ipv4Addr::new(10, 0, 0, 0), 8).unwrap();
        for cidr in vpc_ipv4_cidr_blocks {
          if ten_net.contains(cidr) {
            result = Some(Ipv4Addr::new(10, 100, 0, 10));
            break;
          }
        }
        if result.is_none() {
          result = Some(Ipv4Addr::new(172, 20, 0, 10));
        }
        Ok(IpAddr::V4(result.unwrap()))
      }
      IpvFamily::Ipv6 => bail!("--ip-family ipv6 requires --service-cidr to be supplied"),
    },
  }
}

/// The EKS cluster bootstrap details
///
/// The details required to bootstrap a node to the cluster
#[derive(Debug)]
pub struct ClusterBootstrap {
  /// Name of the cluster
  pub name: String,
  /// Cluster API server endpoint
  pub endpoint: String,
  /// Base64 encoded certificate data
  pub b64_ca: String,
  /// Identifies if the control plane is deployed on Outpost
  pub is_local_cluster: bool,
  /// IP address to use for DNS queries within the cluster
  pub dns_cluster_ip: IpAddr,
}

/// Collect the cluster bootstrap details from input passed into boostrap Command
fn collect_cluster_bootstrap(cli_input: &Bootstrap, dns_cluster_ip: IpAddr) -> Result<Option<ClusterBootstrap>> {
  if let Some(endpoint) = cli_input.apiserver_endpoint.to_owned() {
    if let Some(b64_ca) = cli_input.b64_cluster_ca.to_owned() {
      return Ok(Some(ClusterBootstrap {
        name: cli_input.cluster_name.to_owned(),
        endpoint,
        b64_ca,
        is_local_cluster: cli_input.is_local_cluster,
        dns_cluster_ip,
      }));
    }
  }

  Ok(None)
}

/// Extract cluster bootstrap details from CLI input, or get directly from cluster
///
/// If all the necessary details required to bootstrap a node to the cluster are provided, then
/// we can save an API call. Otherwise, we need to describe the cluster to get the details.
pub async fn collect_or_get_cluster_bootstrap(
  config: SdkConfig,
  cli_input: &Bootstrap,
  vpc_ipv4_cidr_blocks: &[Ipv4Net],
) -> Result<ClusterBootstrap> {
  // DNS cluster IP is not related to cluster - if it cannot be derived, it should fail
  let dns_cluster_ip = match cli_input.dns_cluster_ip {
    Some(ip) => ip,
    None => derive_dns_cluster_ip(&cli_input.service_cidr, &cli_input.ip_family, vpc_ipv4_cidr_blocks)?,
  };
  info!("DNS cluster IP address: {}", dns_cluster_ip);

  let cluster_name = &cli_input.cluster_name.clone();

  match collect_cluster_bootstrap(cli_input, dns_cluster_ip)? {
    Some(bootstrap) => {
      debug!("Cluster bootstrap details collected from CLI input - no describe API call required");
      Ok(bootstrap)
    }
    None => {
      debug!("Cluster bootstrap details are insufficient - describing cluster to get details");
      let client = get_client(config, 3).await?;
      let describe = describe_cluster(&client, cluster_name).await?;

      Ok(ClusterBootstrap {
        name: describe.name.unwrap(),
        endpoint: describe.endpoint.unwrap(),
        b64_ca: describe.certificate_authority.unwrap().data.unwrap(),
        is_local_cluster: describe.outpost_config.is_some(),
        dns_cluster_ip,
      })
    }
  }
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
  #[case(None, &IpvFamily::Ipv4, &["10.1.0.0/24".parse::<Ipv4Net>().unwrap()], IpAddr::V4(Ipv4Addr::new(10, 100, 0, 10)))]
  #[case(None, &IpvFamily::Ipv4, &["192.168.8.0/24".parse::<Ipv4Net>().unwrap(), "10.100.0.0/16".parse::<Ipv4Net>().unwrap()], IpAddr::V4(Ipv4Addr::new(10, 100, 0, 10)))]
  #[case(None, &IpvFamily::Ipv4, &["192.168.8.0/24".parse::<Ipv4Net>().unwrap()], IpAddr::V4(Ipv4Addr::new(172, 20, 0, 10)))]
  #[case(None, &IpvFamily::Ipv4, &["172.16.123.0/24".parse::<Ipv4Net>().unwrap()], IpAddr::V4(Ipv4Addr::new(172, 20, 0, 10)))]
  // --service-cidr required when --ip-family is ipv4
  #[should_panic]
  #[case(None, &IpvFamily::Ipv6, &[], IpAddr::V6("fd00::a".parse::<Ipv6Addr>().unwrap()))]
  fn derive_dns_cluster_ip_test(
    #[case] service_cidr: Option<IpNet>,
    #[case] ip_family: &IpvFamily,
    #[case] vpc_ipv4_cidr_blocks: &[Ipv4Net],
    #[case] expected: IpAddr,
  ) {
    let result = derive_dns_cluster_ip(&service_cidr, ip_family, vpc_ipv4_cidr_blocks).unwrap();
    assert_eq!(expected, result);
  }
}
