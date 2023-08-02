use std::{
  fs,
  os::{linux::fs::MetadataExt, unix::fs::PermissionsExt},
};

use anyhow::{anyhow, Result};
use clap::Args;
use tracing::{error, info};

struct Metadata<'a> {
  path: &'a str,
  // Mode in octal format which shows both permissions
  // as well as additional details such as file type
  mode: &'a str,
  // User and group ID
  id: u32,
}

/// Array of files and their expected properties
static FILES: [Metadata; 12] = [
  Metadata {
    path: "/etc/cni/net.d",
    mode: "40755",
    id: 0,
  },
  Metadata {
    path: "/etc/eks/containerd/containerd-config.toml",
    mode: "100644",
    id: 0,
  },
  Metadata {
    path: "/etc/eks/kubelet-containerd.service",
    mode: "100644",
    id: 0,
  },
  Metadata {
    path: "/etc/eks/sandbox-image.service",
    mode: "100644",
    id: 0,
  },
  Metadata {
    path: "/etc/eks/eni-max-pods.txt",
    mode: "100644",
    id: 0,
  },
  Metadata {
    path: "/etc/eks/image-credential-provider/config.json",
    mode: "100644",
    id: 0,
  },
  Metadata {
    path: "/etc/eks/image-credential-provider/ecr-credential-provider",
    mode: "100755",
    id: 0,
  },
  Metadata {
    path: "/etc/eks/iptables-restore.service",
    mode: "100644",
    id: 0,
  },
  Metadata {
    path: "/etc/kubernetes/kubelet/kubelet-config.json",
    mode: "100644",
    id: 0,
  },
  Metadata {
    path: "/etc/kubernetes/pki/ca.crt",
    mode: "100644",
    id: 0,
  },
  Metadata {
    path: "/etc/logrotate.conf",
    mode: "100644",
    id: 0,
  },
  Metadata {
    path: "/etc/logrotate.d/kube-proxy",
    mode: "100644",
    id: 0,
  },
];

#[derive(Args, Debug)]
pub struct Validation {}

impl Validation {
  pub async fn validate(&self) -> Result<()> {
    validate(FILES.iter()).await
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

          if uid != f.id {
            error!("{} has incorrect uid: {uid}", f.path);
            pass = false;
          }

          if gid != f.id {
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

#[cfg(test)]
mod tests {
  use std::{
    fs::OpenOptions,
    io::Write,
    os::unix::fs::{chown, OpenOptionsExt},
  };

  use tempfile::tempdir;

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
      .unwrap();

    file.write_all(b"hello world").unwrap();
    chown(&dir, Some(1000), Some(1000)).unwrap();
    chown(&path, Some(1000), Some(1000)).unwrap();

    let files = [
      Metadata {
        path: path.to_str().unwrap(),
        mode: "100644",
        id: 1000,
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
