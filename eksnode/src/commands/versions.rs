use anyhow::Result;
use clap::Args;
use serde::{Deserialize, Serialize};

#[derive(Args, Debug, Default, Serialize, Deserialize)]
pub struct Versions {
  /// Output versions in JSON format
  #[arg(long, default_value = "true")]
  pub output_json: bool,

  /// Output versions in Markdown table format
  #[arg(long)]
  pub output_markdown: bool,
}

impl Versions {
  pub async fn get_versions(&self) -> Result<()> {
    Ok(())
  }
}
