use std::process::exit;

use anyhow::Result;
use aws_config::SdkConfig;
use aws_sdk_eks::{
  config::{self, retry::RetryConfig},
  types::Cluster,
  Client,
};
use tracing::error;

/// Get the EKS client
pub async fn get_client(config: SdkConfig, retries: u32) -> Result<Client> {
  let client = Client::from_conf(
    // Start with the shared environment configuration
    config::Builder::from(&config)
      // Set max attempts
      .retry_config(RetryConfig::standard().with_max_attempts(retries))
      .build(),
  );
  Ok(client)
}

/// Describe the cluster to get its full details
pub async fn get_cluster(client: &Client, name: &str) -> Result<Cluster> {
  let request = client.describe_cluster().name(name);
  let response = match request.send().await {
    Ok(response) => response,
    Err(_) => {
      error!("Unable to describe cluster {name}");
      exit(1)
    }
  };

  match response.cluster {
    Some(cluster) => Ok(cluster),
    None => exit(1),
  }
}
