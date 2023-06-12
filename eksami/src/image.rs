use anyhow::Result;
use tracing::error;

/// Get the ECR URI for the given region and domain
///
/// More details about the mappings in this file can be found here
/// https://docs.aws.amazon.com/eks/latest/userguide/add-ons-images.html
pub fn get_ecr_uri(region: &str, domain: &str, enable_fips: bool) -> Result<String> {
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
    let result = get_ecr_uri("ap-east-1", "amazonaws.com", false).unwrap();
    assert_eq!(result, "800184023465.dkr.ecr.ap-east-1.amazonaws.com");
  }

  #[test]
  fn it_gets_ecr_uri_default() {
    let result = get_ecr_uri("us-east-1", "amazonaws.com", false).unwrap();
    assert_eq!(result, "602401143452.dkr.ecr.us-east-1.amazonaws.com");
  }

  #[test]
  fn it_gets_ecr_uri_fips() {
    let result = get_ecr_uri("us-east-1", "amazonaws.com", true).unwrap();
    assert_eq!(result, "602401143452.dkr.ecr-fips.us-east-1.amazonaws.com");
  }
}
