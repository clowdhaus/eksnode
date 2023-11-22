pub mod cli;
pub mod commands;
pub mod containerd;
pub mod ec2;
pub mod ecr;
pub mod eks;
pub mod gpu;
pub mod kubelet;
pub mod resource;
pub mod utils;

use clap::ValueEnum;
pub use cli::{Cli, Commands};
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};

/// Embeds the contents of the `files/` directory into the binary
///
/// This struct contains the static data used within `eksnode`
#[derive(RustEmbed)]
#[folder = "files/"]
pub struct Assets;

#[derive(Copy, Clone, Debug, ValueEnum, Serialize, Deserialize)]
pub enum IpvFamily {
  Ipv4,
  Ipv6,
}

impl Default for IpvFamily {
  fn default() -> Self {
    Self::Ipv4
  }
}
