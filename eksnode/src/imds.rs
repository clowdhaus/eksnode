use std::net::{Ipv4Addr, Ipv6Addr};

use anyhow::Result;
use aws_config::{imds::client::Client, provider_config::ProviderConfig};
use http::Uri;
use ipnet::Ipv4Net;
use serde::{Deserialize, Serialize};
use tokio::time::Duration;

/// Get the IMDS client
async fn get_client() -> Result<Client> {
  let config = ProviderConfig::with_default_region().await;
  let mut client = Client::builder()
    .configure(&config)
    .max_attempts(5)
    .token_ttl(Duration::from_secs(900))
    .connect_timeout(Duration::from_secs(5))
    .read_timeout(Duration::from_secs(5));

  if let Ok(endpoint) = std::env::var("IMDS_ENDPOINT") {
    client = client.endpoint(endpoint.parse::<Uri>()?);
  }

  Ok(client.build().await?)
}

/// EC2 Instance metadata
///
/// https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/instancedata-data-categories.html
#[derive(Debug, Serialize, Deserialize)]
pub struct InstanceMetadata {
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
  pub ipv6_addresses: Vec<Ipv6Addr>,
  /// The instance type of the instance.
  pub instance_type: String,
  /// The ID of the instance.
  pub instance_id: String,
}

/// Get data from the IMDS endpoint
///
/// Collects the relevant metadata from IMDS used in joining node to cluster
pub async fn get_imds_data() -> Result<InstanceMetadata> {
  let client = get_client().await?;
  let region = client.get("/latest/meta-data/placement/region").await?;
  let domain = client.get("/latest/meta-data/services/domain").await?;
  let mac_address: String = client.get("/latest/meta-data/mac").await?;
  let vpc_ipv4_cidr_blocks = client
    .get(&format!(
      "/latest/meta-data/network/interfaces/macs/{mac_address}/vpc-ipv4-cidr-blocks"
    ))
    .await
    .expect("Failed to get VPC IPv4 CIDR blocks")
    .split('\n')
    .map(|s| s.parse::<Ipv4Net>().expect("Failed to parse VPC IPv4 CIDR block"))
    .collect();
  let local_ipv4 = match client.get("/latest/meta-data/local-ipv4").await {
    Ok(s) => Some(s.parse::<Ipv4Addr>().expect("Failed to parse local IPv4 address")),
    Err(_) => None,
  };
  let ipv6s_uri = format!("/latest/meta-data/network/interfaces/macs/{mac_address}/ipv6s");
  let ipv6_addresses = client
    .get(&ipv6s_uri)
    .await
    .expect("Failed to get IPv6 addresses")
    .split('\n')
    .map(|s| s.parse::<Ipv6Addr>().expect("Failed to parse IPv6 address"))
    .collect();
  let instance_type = client.get("/latest/meta-data/instance-type").await?;
  let instance_id = client.get("/latest/meta-data/instance-id").await?;

  let metadata = InstanceMetadata {
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
  let client = get_client().await?;
  let instance_type = client.get("/latest/meta-data/instance-type").await?;

  Ok(instance_type)
}