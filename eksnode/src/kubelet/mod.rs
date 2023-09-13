mod args;
mod config;
mod credential;
mod kubeconfig;

use anyhow::Result;
pub use args::{Args, ExtraArgs, ARGS_PATH, EXTRA_ARGS_PATH};
pub use config::KubeletConfiguration;
pub use credential::CredentialProviderConfig;
pub use kubeconfig::KubeConfig;
use semver::Version;
use tracing::debug;

use crate::utils;

pub fn get_kubelet_version() -> Result<Version> {
  let cmd_output = utils::cmd_exec("kubelet", vec!["--version"])?;
  debug!("kubelet version: {cmd_output}");

  utils::get_semver(&cmd_output)
}
