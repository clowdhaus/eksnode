use anyhow::Result;
use aws_config::BehaviorVersion;
use aws_sdk_ecr::{
  config::{self, retry::RetryConfig, timeout::TimeoutConfig},
  Client,
};
use tokio::time::Duration;
use tracing::error;

/// Get the ECR client
pub async fn get_client() -> Result<Client> {
  let sdk_config = aws_config::load_defaults(BehaviorVersion::latest()).await;
  let timeout_config = TimeoutConfig::builder()
    .operation_attempt_timeout(Duration::from_secs(5))
    .build();

  let config = config::Builder::from(&sdk_config)
    .retry_config(RetryConfig::adaptive().with_max_attempts(3))
    .timeout_config(timeout_config)
    .build();

  Ok(Client::from_conf(config))
}

pub async fn get_authorization_token(client: &Client) -> Result<String> {
  let resp = client.get_authorization_token().send().await?;
  let token = resp
    .authorization_data
    .expect("Failed to get ECR authorization data")
    .pop()
    .unwrap()
    .authorization_token
    .expect("Failed to get ECR authorization token");

  Ok(token)
}

/// Get the ECR URI for the given region and domain
///
/// More details about the mappings in this file can be found here
/// https://docs.aws.amazon.com/eks/latest/userguide/add-ons-images.html
/// ECR endpoints https://docs.aws.amazon.com/general/latest/gr/ecr.html
pub fn get_ecr_uri(region: &str, enable_fips: bool) -> Result<String> {
  let acct_id = match region {
    "af-south-1" => "877085696533",
    "ap-east-1" => "800184023465",
    "ap-south-2" => "900889452093",
    "ap-southeast-4" => "491585149902",
    "ap-southeast-3" => "296578399912",
    "cn-north-1" => "918309763551",
    "cn-northwest-1" => "961992271922",
    "eu-central-2" => "900612956339",
    "eu-south-1" => "590381155156",
    "eu-south-2" => "455263428931",
    "me-south-1" => "558608220178",
    "me-central-1" => "759879836304",
    "us-gov-east-1" => "151742754352",
    "us-gov-west-1" => "013241004608",
    "us-iso-west-1" => "608367168043",
    "us-iso-east-1" => "725322719131",
    "us-isob-east-1" => "187977181151",
    "il-central-1" => "066635153087",
    _ => "602401143452",
  };

  let domain = match region {
    "cn-north-1" | "cn-northwest-1" => "amazonaws.com.cn",
    _ => "amazonaws.com",
  };

  if enable_fips && !region.starts_with("us-") {
    error!("FIPS endpoints are only supported in US regions");
  }

  let uri = match enable_fips {
    true => format!("{acct_id}.dkr.ecr-fips.{region}.{domain}"),
    false => format!("{acct_id}.dkr.ecr.{region}.{domain}"),
  };

  Ok(uri)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn it_gets_ecr_uri_apeast1() {
    let result = get_ecr_uri("cn-north-1", false).unwrap();
    assert_eq!(result, "918309763551.dkr.ecr.cn-north-1.amazonaws.com.cn");
  }

  #[test]
  fn it_gets_ecr_uri_default() {
    let result = get_ecr_uri("us-east-1", false).unwrap();
    assert_eq!(result, "602401143452.dkr.ecr.us-east-1.amazonaws.com");
  }

  #[test]
  fn it_gets_ecr_uri_fips() {
    let result = get_ecr_uri("us-east-1", true).unwrap();
    assert_eq!(result, "602401143452.dkr.ecr-fips.us-east-1.amazonaws.com");
  }
}
