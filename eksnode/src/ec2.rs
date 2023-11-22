use std::{
  collections::HashMap,
  net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

use anyhow::{Context, Result};
use aws_config::{imds::client::Client as ImdsClient, provider_config::ProviderConfig, BehaviorVersion};
use aws_sdk_ec2::{
  config::{self, retry::RetryConfig},
  Client,
};
use http::Uri;
use ipnet::Ipv4Net;
use serde::{Deserialize, Serialize};
use tokio::time::Duration;
use tokio_retry::{
  strategy::{jitter, FibonacciBackoff},
  Retry,
};

use crate::Assets;

// Limit the timeout for fetching the private DNS name of the EC2 instance to 5 minutes.
const FETCH_PRIVATE_DNS_NAME_TIMEOUT: Duration = Duration::from_secs(300);
// Fibonacci backoff base duration when retrying requests
const FIBONACCI_BACKOFF_BASE_DURATION_MILLIS: u64 = 200;

/// Get the EC2 client
pub async fn get_client() -> Result<Client> {
  let sdk_config = aws_config::load_defaults(BehaviorVersion::v2023_11_09()).await;
  let client = Client::from_conf(
    // Start with the shared environment configuration
    config::Builder::from(&sdk_config)
      // Set max attempts
      .retry_config(RetryConfig::standard().with_max_attempts(3))
      .build(),
  );
  Ok(client)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Instance {
  /// The default number of vCPUs for the instance
  pub default_vcpus: i32,

  /// The (theoretical) maximum number of pods
  ///
  /// This is based off the maximum number of ENIs and the maximum number of IPv4 addresses per ENI
  pub eni_maximum_pods: i32,

  /// The hypervisor (nitro | xen | unknown)
  pub hypervisor: String,

  /// Indicates whether instance storage is supported
  pub instance_storage_supported: bool,

  /// The maximum number of IPv4 addresses per ENI
  pub ipv4_addresses_per_interface: i32,

  /// The maximum number of ENIs
  pub maximum_network_interfaces: i32,
}

pub fn get_instance(instance: &str) -> Result<Option<Instance>> {
  let file = Assets::get("ec2-instances.yaml").unwrap();
  let contents = std::str::from_utf8(file.data.as_ref())?;
  let instances: HashMap<String, Instance> = serde_yaml::from_str(contents)?;

  Ok(instances.get(instance).cloned())
}

/// Get the IMDS client
async fn get_imds_client() -> Result<ImdsClient> {
  let config = ProviderConfig::with_default_region().await;
  let mut client = ImdsClient::builder()
    .configure(&config)
    .max_attempts(5)
    .token_ttl(Duration::from_secs(90))
    .connect_timeout(Duration::from_secs(5))
    .read_timeout(Duration::from_secs(5));

  if let Ok(endpoint) = std::env::var("IMDS_ENDPOINT") {
    let endpoint = endpoint.parse::<Uri>()?;
    client = client.endpoint(endpoint.to_string()).map_err(|e| anyhow::anyhow!(e))?;
  }

  Ok(client.build())
}

pub async fn get_private_dns_name(instance_id: &str, client: &Client) -> Result<String> {
  tokio::time::timeout(
    FETCH_PRIVATE_DNS_NAME_TIMEOUT,
    Retry::spawn(
      FibonacciBackoff::from_millis(FIBONACCI_BACKOFF_BASE_DURATION_MILLIS).map(jitter),
      || async {
        client
          .describe_instances()
          .instance_ids(instance_id.to_owned())
          .send()
          .await
          .context(format!("Unable to describe instance {instance_id}"))?
          .reservations
          .and_then(|reservations| {
            reservations.first().and_then(|r| {
              r.instances.clone().and_then(|instances| {
                instances
                  .first()
                  .and_then(|i| i.private_dns_name().map(|s| s.to_string()))
              })
            })
          })
          .filter(|private_dns_name| !private_dns_name.is_empty())
          .context("Reservation.Instance.PrivateDNSName is empty")
      },
    ),
  )
  .await
  .context("Failed to get PrivateDnsName")?
}

/// EC2 Instance metadata
///
/// https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/instancedata-data-categories.html
#[derive(Debug, Serialize, Deserialize)]
pub struct InstanceMetadata {
  /// The availablity zone in which the instance is launched
  pub availability_zone: String,
  /// The AWS Region in which the instance is launched
  pub region: String,
  /// The domain for AWS resources for the Region
  pub domain: String,
  /// The instance's media access control (MAC) address.
  ///
  /// In cases where multiple network interfaces are present,
  /// this refers to the eth0 device (the device for which the device number is 0)
  pub mac_address: String,
  /// The IPv4 CIDR blocks for the VPC.
  pub vpc_ipv4_cidr_blocks: Vec<Ipv4Net>,
  /// The private IPv4 address of the instance.
  ///
  /// In cases where multiple network interfaces are present,
  /// this refers to the eth0 device (the device for which the device number is 0)
  pub local_ipv4: Option<Ipv4Addr>,
  /// The IPv6 addresses associated with the interface
  pub ipv6_addresses: Option<Vec<Ipv6Addr>>,
  /// The instance type of the instance.
  pub instance_type: String,
  /// The ID of the instance.
  pub instance_id: String,
}

impl InstanceMetadata {
  pub fn get_node_ip(&self, ip_family: &crate::IpvFamily) -> Result<String> {
    let node_ip = match ip_family {
      crate::IpvFamily::Ipv4 => IpAddr::V4(self.local_ipv4.expect("Failed to get node local IPv4 address")),
      crate::IpvFamily::Ipv6 => {
        let ips = self
          .ipv6_addresses
          .clone()
          .expect("No IPv6 addresses found for the instance");
        IpAddr::V6(ips.first().cloned().expect("Failed to get node IPv6 address"))
      }
    };

    Ok(node_ip.to_string())
  }
}

/// Get data from the IMDS endpoint
///
/// Collects the relevant metadata from IMDS used in joining node to cluster
pub async fn get_imds_data() -> Result<InstanceMetadata> {
  let client = get_imds_client().await?;
  let availability_zone = client
    .get("/latest/meta-data/placement/availability-zone")
    .await?
    .into();
  let region = client.get("/latest/meta-data/placement/region").await?.into();
  let domain = client.get("/latest/meta-data/services/domain").await?.into();
  let mac_address = client.get("/latest/meta-data/mac").await?.into();
  let vpc_ipv4_cidr_blocks = client
    .get(&format!(
      "/latest/meta-data/network/interfaces/macs/{mac_address}/vpc-ipv4-cidr-blocks"
    ))
    .await
    .expect("Failed to get VPC IPv4 CIDR blocks")
    .as_ref()
    .split('\n')
    .map(|s| s.parse::<Ipv4Net>().expect("Failed to parse VPC IPv4 CIDR block"))
    .collect();
  let local_ipv4 = match client.get("/latest/meta-data/local-ipv4").await {
    Ok(s) => Some(
      s.as_ref()
        .parse::<Ipv4Addr>()
        .expect("Failed to parse local IPv4 address"),
    ),
    Err(_) => None,
  };
  let ipv6s_uri = format!("/latest/meta-data/network/interfaces/macs/{mac_address}/ipv6s");
  let ipv6_addresses = match client.get(&ipv6s_uri).await {
    Ok(s) => Some(
      s.as_ref()
        .split('\n')
        .map(|s| s.parse::<Ipv6Addr>().expect("Failed to parse IPv6 address"))
        .collect(),
    ),
    Err(_) => None,
  };
  let instance_type = client.get("/latest/meta-data/instance-type").await?.into();
  let instance_id = client.get("/latest/meta-data/instance-id").await?.into();

  let metadata = InstanceMetadata {
    availability_zone,
    region,
    domain,
    mac_address,
    vpc_ipv4_cidr_blocks,
    local_ipv4,
    ipv6_addresses,
    instance_type,
    instance_id,
  };

  Ok(metadata)
}

/// Get the instance type from IMDS endpoint
pub async fn get_instance_type() -> Result<String> {
  let client = get_imds_client().await?;
  let instance_type = client.get("/latest/meta-data/instance-type").await?;

  Ok(instance_type.into())
}

/// Get the current region from IMDS endpoint
pub async fn get_region() -> Result<String> {
  let client = get_imds_client().await?;
  let region = client.get("/latest/meta-data/placement/region").await?;

  Ok(region.into())
}

/// Returns all regions for the current partition
pub async fn get_all_regions() -> Result<Vec<String>> {
  let client = get_client().await?;

  let regions = client.describe_regions().all_regions(true).send().await.map(|r| {
    r.regions
      .unwrap_or_default()
      .into_iter()
      .map(|r| r.region_name.unwrap_or_default())
      .collect::<Vec<String>>()
  })?;

  Ok(regions)
}
