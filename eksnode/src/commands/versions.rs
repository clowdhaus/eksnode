use anyhow::Result;
use clap::Args;
use serde::{Deserialize, Serialize};
use tabled::{Table, Tabled};

use crate::utils;

const RPM_SEPARATOR: char = '|';

/// Package details containing the name and version of the package
///
/// Release is optional as it is not always available; typically
/// its only valid for RPM/Linux packages
#[derive(Debug, Serialize, Deserialize, Tabled)]
pub struct Package {
  name: String,
  version: String,
}

pub trait PackageRepository {
  fn versions(&self) -> Result<Vec<Package>>;
}

/// Get the versions of the packages
///
/// Trait wrapper to support testing
fn get_versions<T: PackageRepository>(pkg: T) -> Result<Vec<Package>> {
  pkg.versions()
}

/// Input arguments for `get-versions` command
#[derive(Args, Debug, Default, Serialize, Deserialize)]
pub struct GetVersionsInput {
  /// Output versions in JSON format
  #[arg(long, default_value = "true")]
  pub output_json: bool,

  /// Output versions in Markdown table format
  #[arg(long)]
  pub output_markdown: bool,
}

struct Rpm {}

impl GetVersionsInput {
  pub async fn get_versions(&self) -> Result<()> {
    let rpm = Rpm {};
    let rpm_versions = get_versions(rpm)?;

    match self.output_markdown {
      true => {
        let table = Table::new(&rpm_versions).to_string();
        println!("{}", table);
      }
      false => {}
    }

    match self.output_json {
      true => {
        let versions = Versions { linux: rpm_versions };
        println!("{}", serde_json::to_string_pretty(&versions)?);
      }
      false => {}
    }

    Ok(())
  }
}

/// Resulting output from version collection
#[derive(Debug, Default, Serialize, Deserialize)]
struct Versions {
  linux: Vec<Package>,
}

impl PackageRepository for Rpm {
  fn versions(&self) -> Result<Vec<Package>> {
    let cmd = utils::cmd_exec(
      "rpm",
      vec![
        "--query",
        "--all",
        "--queryformat",
        ["%{NAME}", "%{VERSION}", "%{RELEASE}\n"]
          .join(&RPM_SEPARATOR.to_string())
          .as_str(),
      ],
    )?;

    let pkgs = cmd
      .stdout
      .lines()
      .map(|line| {
        let mut parts = line.split(RPM_SEPARATOR);
        Package {
          name: parts.next().unwrap_or_default().to_string(),
          version: parts
            .map(|release| release.to_string())
            .collect::<Vec<String>>()
            .join("-"),
        }
      })
      .collect::<Vec<Package>>();

    Ok(pkgs)
  }
}

#[cfg(test)]
mod tests {

  use super::*;

  #[test]
  fn test_get_versions() {
    struct MockRpm {}

    impl PackageRepository for MockRpm {
      fn versions(&self) -> Result<Vec<Package>> {
        let pkgs = vec![
          Package {
            name: "package1".to_string(),
            version: "1.0.0".to_string(),
          },
          Package {
            name: "package2".to_string(),
            version: "2.0.0".to_string(),
          },
        ];
        Ok(pkgs)
      }
    }
    let rpm = MockRpm {};
    let rpm_versions = get_versions(rpm).unwrap();

    assert_eq!(rpm_versions.first().unwrap().name, "package1");
    assert_eq!(rpm_versions.first().unwrap().version, "1.0.0");
  }
}
