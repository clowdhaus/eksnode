use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_verbosity_flag::Verbosity;
use serde::{Deserialize, Serialize};

/// Styles for CLI
fn get_styles() -> clap::builder::Styles {
  clap::builder::Styles::styled()
    .header(
      anstyle::Style::new()
        .bold()
        .underline()
        .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green))),
    )
    .literal(
      anstyle::Style::new()
        .bold()
        .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::BrightCyan))),
    )
    .usage(
      anstyle::Style::new()
        .bold()
        .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green))),
    )
    .placeholder(
      anstyle::Style::new()
        .bold()
        .underline()
        .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow))),
    )
}

#[derive(Debug, Parser)]
#[command(author, about, version)]
#[command(propagate_version = true)]
#[command(styles=get_styles())]
pub struct Cli {
  #[command(subcommand)]
  pub command: Commands,

  #[clap(flatten)]
  pub verbose: Verbosity,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
  /// Bootstraps an instance into an EKS cluster
  Bootstrap(Bootstrap),
}

#[derive(Args, Debug, Serialize, Deserialize)]
pub struct Bootstrap {
  /// The EKS cluster API Server endpoint
  ///
  /// Only valid when used with --b64-cluster-ca. Bypasses calling "aws eks describe-cluster"
  #[arg(long)]
  apiserver_endpoint: Option<String>,

  /// The base64 encoded cluster CA content
  ///
  /// Only valid when used with --apiserver-endpoint. Bypasses calling "aws eks describe-cluster"
  #[arg(long)]
  b64_cluster_ca: Option<String>,

  /// The ID of the EKS cluster
  #[arg(long)]
  cluster_id: String,

  /// File containing the containerd configuration to be used in place of AMI defaults
  #[arg(long)]
  containerd_config_file: Option<String>,

  /// Overrides the IP address to use for DNS queries within the cluster
  ///
  /// Defaults to 10.100.0.10 or 172.20.0.10 based on the IP address of the primary interface
  #[arg(long)]
  dns_cluster_ip: Option<String>,

  /// Execute the bootstrap process without making any changes to the system
  ///
  /// Useful for debugging - will display changes that are intended to be made during bootstrapping
  #[arg(long)]
  dry_run: bool,

  /// Enable support for worker nodes to communicate with the local control plane when running on a disconnected Outpost
  #[arg(long)]
  enable_local_outpost: bool,

  /// Specify ip family of the cluster
  #[arg(long)]
  ip_family: Option<String>,

  /// Extra arguments to add to the kubelet
  ///
  /// Useful for adding labels or taints
  #[arg(long)]
  kubelet_extra_args: Option<String>,

  /// Setup instance storage NVMe disks in raid0 or mount the individual disks for use by pods
  #[arg(long, value_enum)]
  local_disks: Option<LocalDisks>,

  /// Mount a bpffs at /sys/fs/bpf (default: true)
  #[arg(long, default_value = "true")]
  mount_bfs_fs: bool,

  /// The AWS account (number) to pull the pause container from
  #[arg(long)]
  pause_container_account: Option<String>,

  /// The tag of the pause container
  #[arg(long, default_value = "3.5")]
  pause_container_version: Option<String>,

  /// IPv6 CIDR range of the cluster
  #[arg(long)]
  service_ipv6_cidr: Option<String>,

  /// Sets --max-pods for the kubelet when true (default: true)
  #[arg(long, default_value = "true")]
  use_max_pods: bool,
}

#[derive(Clone, Debug, ValueEnum, Serialize, Deserialize)]
pub enum LocalDisks {
  /// Mount local disks individually
  Mount,
  /// Mount local disk in a raid0 configuration
  Raid0,
}

impl Default for LocalDisks {
  fn default() -> Self {
    Self::Raid0
  }
}

impl Bootstrap {
  pub async fn run(&self) -> Result<()> {
    // crate::imds::get_imds_data().await?;

    Ok(())
  }
}
