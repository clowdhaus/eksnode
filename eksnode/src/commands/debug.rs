use anyhow::Result;
use clap::Args;
use serde::{Deserialize, Serialize};

#[derive(Args, Debug, Default, Serialize, Deserialize)]
pub struct Debug {
  /// Collect various log files and package into a zip archive
  #[arg(long)]
  pub create_log_archive: bool,
}

impl Debug {
  pub async fn debug(&self) -> Result<()> {
    Ok(())
  }
}
