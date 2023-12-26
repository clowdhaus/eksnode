#[cfg(target_os = "linux")]
use std::os::linux::fs::MetadataExt;
// For development on macOS system
#[cfg(target_os = "macos")]
use std::os::macos::fs::MetadataExt;
use std::{fs, os::unix::fs::PermissionsExt};

use anyhow::{anyhow, Result};
use clap::Args;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::Assets;

#[derive(Debug, Serialize, Deserialize)]
struct Metadata<'a> {
  path: &'a str,
  // Mode in octal format which shows both permissions
  // as well as additional details such as file type
  mode: &'a str,
  // User ID
  uid: u32,
  // Group ID
  gid: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct Validate<'a> {
  #[serde(borrow)]
  files: Vec<Metadata<'a>>,
}

#[derive(Args, Debug)]
pub struct ValidateNodeInput {}

impl ValidateNodeInput {
  pub async fn validate(&self) -> Result<()> {
    let file = Assets::get("validate.yaml").unwrap();
    let contents = std::str::from_utf8(file.data.as_ref())?;
    let validation: Validate = serde_yaml::from_str(contents)?;

    validate(validation.files.iter()).await
  }
}

/// Iterate over the array of files and validate their properties
/// against the expected values
async fn validate<'a, I>(files: I) -> Result<()>
where
  I: Iterator<Item = &'a Metadata<'a>>,
{
  let mut pass = true;
  files
    .map(|f| {
      match fs::metadata(f.path) {
        Ok(meta) => {
          let mode = meta.permissions().mode();
          let uid = meta.st_uid();
          let gid = meta.st_gid();

          if mode != u32::from_str_radix(f.mode, 8)? {
            error!("{} has incorrect mode: {mode:0}", f.path);
            pass = false;
          }

          if uid != f.uid {
            error!("{} has incorrect uid: {uid}", f.path);
            pass = false;
          }

          if gid != f.gid {
            error!("{} has incorrect gid: {gid}", f.path);
            pass = false;
          }
        }
        Err(e) => {
          error!("{}: {}", f.path, e);
          pass = false;
        }
      };

      Ok(())
    })
    .collect::<Result<Vec<()>>>()?;

  match pass {
    true => {
      info!("Validation succeeded");
      Ok(())
    }
    false => Err(anyhow!("Validation failed")),
  }
}

#[cfg(target_os = "linux")]
#[cfg(test)]
mod tests {
  use std::os::unix::fs::chown;

  use tempfile::tempdir;
  use tokio::{fs::OpenOptions, io::AsyncWriteExt};

  use super::*;

  #[tokio::test]
  async fn it_validates() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("foo");
    let mut file = OpenOptions::new()
      .write(true)
      .create(true)
      .mode(0o644)
      .open(&path)
      .await
      .unwrap();

    file.write_all(b"hello world").await.unwrap();
    file.flush().await.unwrap();

    // chown(&dir, Some(1000), Some(1000)).unwrap();
    chown(&path, Some(1000), Some(1000)).unwrap();

    let files = [
      Metadata {
        path: path.to_str().unwrap(),
        mode: "100644",
        uid: 1000,
        gid: 1000,
      },
      // TODO - figure out why this is failing
      // Metadata {
      //   path: dir.path().to_str().unwrap(),
      //   mode: "40755",
      //   id: 1000,
      // },
    ];

    let result = validate(files.iter());
    assert!(result.await.is_ok());
  }
}
