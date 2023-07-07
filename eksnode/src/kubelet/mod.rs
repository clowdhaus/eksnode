mod config;
mod credential;
mod kubeconfig;

use anyhow::Result;
pub use config::KubeletConfiguration;
pub use credential::CredentialProviderConfig;
pub use kubeconfig::KubeConfig;
use semver::Version;

use crate::utils;

pub fn get_kubelet_version() -> Result<Version> {
  let cmd_output = utils::cmd_exec("kubelet", vec!["--version"])?;

  utils::get_semver(&cmd_output)
}
