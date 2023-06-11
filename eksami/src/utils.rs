use anyhow::Result;
use regex::Regex;
use semver::Version;

/// Extract the semantic version from the version string provided
pub fn get_semver(ver: &str) -> Result<Version> {
  let re = Regex::new(r"v?(\d+\.\d+\.\d+)(-.*)?")?;
  return match re.captures(ver) {
    Some(cap) => match cap.get(1) {
      Some(cap) => {
        let version = Version::parse(cap.as_str()).unwrap();
        Ok(version)
      }
      None => Err(anyhow::anyhow!("Unable to parse version")),
    },
    None => Err(anyhow::anyhow!("Unable to parse version")),
  };
}

pub fn get_kubelet_version() -> Result<Version> {
  todo!();
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn it_gets_semver_bare() {
    let expected = Version::parse("1.20.4").unwrap();
    let result = get_semver("1.20.4").unwrap();
    assert_eq!(result, expected);
  }

  #[test]
  fn it_gets_semver_leading() {
    let expected = Version::parse("1.20.4").unwrap();
    let result = get_semver("v1.20.4").unwrap();
    assert_eq!(result, expected);
  }

  #[test]
  fn it_gets_semver_trailing() {
    let expected = Version::parse("1.20.4").unwrap();
    let result = get_semver("v1.20.4-this.something_else").unwrap();
    assert_eq!(result, expected);
  }
}
