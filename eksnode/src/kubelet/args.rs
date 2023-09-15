use std::path::Path;

use anyhow::Result;

use crate::utils;

pub const ARGS_PATH: &str = "/etc/systemd/system/kubelet.service.d/10-kubelet-args.conf";
pub const EXTRA_ARGS_PATH: &str = "/etc/systemd/system/kubelet.service.d/30-kubelet-extra-args.conf";

#[derive(Debug, Default)]
pub struct Args {
  pub node_ip: String,
  pub pod_infra_container_image: String,
  pub hostname_override: Option<String>,
  pub cloud_provider: String,
  pub container_runtime: Option<String>,
}

impl Args {
  pub fn write<P: AsRef<Path>>(&self, path: P, chown: bool) -> Result<()> {
    let end = " \\\n";
    let mut args = format!("--v=2{end}");

    args.push_str(&format!("\t--node-ip={}{end}", self.node_ip));
    args.push_str(&format!(
      "\t--pod-infra-container-image={}{end}",
      self.pod_infra_container_image
    ));
    if let Some(hostname_override) = &self.hostname_override {
      args.push_str(&format!("\t--hostname-override={}{end}", hostname_override));
    }
    args.push_str(&format!("\t--cloud-provider={}{end}", self.cloud_provider));
    if let Some(container_runtime) = &self.container_runtime {
      args.push_str(&format!("\t--container-runtime={}{end}", container_runtime));
    }

    // To ensure file content integrity
    if path.as_ref().is_file() {
      std::fs::remove_file(&path)?;
    }

    let args = args.strip_suffix(end).unwrap();
    let content = format!("[Service]\nEnvironment='KUBELET_ARGS={args}'\n",);
    utils::write_file(content.as_bytes(), path, Some(0o644), chown)
  }
}

#[derive(Debug, Default)]
pub struct ExtraArgs {
  pub args: Option<String>,
}

impl ExtraArgs {
  pub fn new(args: Option<String>) -> Self {
    Self { args }
  }

  pub fn write<P: AsRef<Path>>(&self, path: P, chown: bool) -> Result<()> {
    let args = match self.args {
      Some(ref args) => args,
      None => "",
    };

    // To ensure file content integrity
    if path.as_ref().is_file() {
      std::fs::remove_file(&path)?;
    }

    let contents = format!("[Service]\nEnvironment='KUBELET_EXTRA_ARGS={args}'\n");
    utils::write_file(contents.as_bytes(), path, Some(0o644), chown)
  }
}

#[cfg(test)]
mod tests {
  use std::io::{Read, Seek, SeekFrom};

  use tempfile::NamedTempFile;

  use super::*;

  #[test]
  fn it_creates_args() {
    let args = Args {
      node_ip: "10.0.0.1".to_string(),
      pod_infra_container_image: "k8s.gcr.io/pause:3.1".to_string(),
      hostname_override: None,
      cloud_provider: "external".to_string(),
      container_runtime: Some("remote".to_string()),
    };

    // Write to file
    let mut file = NamedTempFile::new().unwrap();
    args.write(file.path(), false).unwrap();
    file.seek(SeekFrom::Start(0)).unwrap();

    // Read back contents written to file
    let mut buf = String::new();
    file.read_to_string(&mut buf).unwrap();
    insta::assert_debug_snapshot!(buf);
  }

  #[test]
  fn it_creates_empty_extrargs() {
    let args = ExtraArgs::new(None);

    // Write to file
    let mut file = NamedTempFile::new().unwrap();
    args.write(file.path(), false).unwrap();
    file.seek(SeekFrom::Start(0)).unwrap();

    // Read back contents written to file
    let mut buf = String::new();
    file.read_to_string(&mut buf).unwrap();
    insta::assert_debug_snapshot!(buf);
  }

  #[test]
  fn it_creates_extrargs() {
    let args = ExtraArgs::new(Some("--max-pods=true".to_string()));

    // Write to file
    let mut file = NamedTempFile::new().unwrap();
    args.write(file.path(), false).unwrap();
    file.seek(SeekFrom::Start(0)).unwrap();

    // Read back contents written to file
    let mut buf = String::new();
    file.read_to_string(&mut buf).unwrap();
    insta::assert_debug_snapshot!(buf);
  }
}
