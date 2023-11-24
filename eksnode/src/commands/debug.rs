use anyhow::Result;
use clap::Args;
use serde::{Deserialize, Serialize};

#[derive(Args, Debug, Default, Serialize, Deserialize)]
pub struct DebugInput {
  /// Collect various log files and package into a zip archive
  #[arg(long)]
  pub create_log_archive: bool,
}

impl DebugInput {
  pub async fn debug(&self) -> Result<()> {
    Ok(())
  }
}
