pub mod cli;
pub mod commands;
pub mod containerd;
pub mod ec2;
pub mod ecr;
pub mod eks;
pub mod gpu;
pub mod kubelet;
pub mod resource;
pub mod utils;

use std::env;

use anyhow::Result;
use aws_config::{meta::region::RegionProviderChain, SdkConfig};
use aws_types::region::Region;
use clap::ValueEnum;
pub use cli::{Cli, Commands};
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};

/// Embeds the contents of the `files/` directory into the binary
///
/// This struct contains the static data used within `eksnode`
#[derive(RustEmbed)]
#[folder = "files/"]
pub struct Assets;

/// Get the configuration to authn/authz with AWS that will be used across AWS clients
pub async fn get_sdk_config(region: Option<String>) -> Result<SdkConfig> {
  let aws_region = match region {
    Some(region) => Some(Region::new(region)),
    None => env::var("AWS_DEFAULT_REGION").ok().map(Region::new),
  };

  let region_provider = RegionProviderChain::first_try(aws_region).or_default_provider();

  Ok(aws_config::from_env().region(region_provider).load().await)
}

#[derive(Copy, Clone, Debug, ValueEnum, Serialize, Deserialize)]
pub enum IpvFamily {
  Ipv4,
  Ipv6,
}

impl Default for IpvFamily {
  fn default() -> Self {
    Self::Ipv4
  }
}
