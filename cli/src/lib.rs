pub mod cli;
pub mod eks;
pub mod imds;
pub mod utils;

use std::env;

use anyhow::Result;
use aws_config::{meta::region::RegionProviderChain, SdkConfig};
use aws_types::region::Region;
pub use cli::{Cli, Commands};

/// Get the configuration to authn/authz with AWS that will be used across AWS clients
pub async fn get_sdk_config(region: &Option<String>) -> Result<SdkConfig> {
  let aws_region = match region {
    Some(region) => Some(Region::new(region.to_owned())),
    None => env::var("AWS_REGION").ok().map(Region::new),
  };

  let region_provider = RegionProviderChain::first_try(aws_region).or_default_provider();

  Ok(aws_config::from_env().region(region_provider).load().await)
}
