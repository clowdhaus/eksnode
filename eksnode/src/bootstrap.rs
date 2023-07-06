use std::net::IpAddr;

use anyhow::Result;
use clap::{Args, ValueEnum};
use ipnet::IpNet;
use serde::{Deserialize, Serialize};

use crate::eks;

#[derive(Args, Debug, Serialize, Deserialize)]
pub struct Bootstrap {
  /// The EKS cluster API Server endpoint
  ///
  /// Only valid when used with --b64-cluster-ca. Bypasses calling "aws eks describe-cluster"
  #[arg(long)]
  pub apiserver_endpoint: Option<String>,

  /// The base64 encoded cluster CA content
  ///
  /// Only valid when used with --apiserver-endpoint. Bypasses calling "aws eks describe-cluster"
  #[arg(long)]
  pub b64_cluster_ca: Option<String>,

  /// The name of the EKS cluster
  #[arg(long)]
  pub cluster_name: String,

  /// File containing the containerd configuration to be used in place of AMI defaults
  #[arg(long)]
  pub containerd_config_file: Option<String>,

  /// Overrides the IP address to use for DNS queries within the cluster
  ///
  /// Defaults to 10.100.0.10 or 172.20.0.10 based on the IP address of the primary interface
  #[arg(long)]
  pub dns_cluster_ip: Option<IpAddr>,

  /// Execute the bootstrap process without making any changes to the system
  ///
  /// Useful for debugging - will display changes that are intended to be made during bootstrapping
  #[arg(long)]
  pub dry_run: bool,

  /// Specifies cluster is a local cluster on Outpost
  #[arg(long)]
  pub is_local_cluster: bool,

  /// Specify ip family of the cluster
  #[arg(long, value_enum, default_value_t)]
  pub ip_family: IpvFamily,

  /// Extra arguments to add to the kubelet
  ///
  /// Useful for adding labels or taints
  #[arg(long)]
  pub kubelet_extra_args: Option<String>,

  /// Setup instance storage NVMe disks in raid0 or mount the individual disks for use by pods
  #[arg(long, value_enum)]
  pub local_disks: Option<LocalDisks>,

  /// The pause container image <registry>:<tag/version>
  #[arg(long)]
  pub pause_container_image: Option<String>,

  /// IPv4 or IPv6 CIDR range of the cluster
  #[arg(long)]
  pub service_cidr: Option<IpNet>,

  /// Sets --max-pods for the kubelet when true (default: true)
  #[arg(long, default_value = "true")]
  pub use_max_pods: bool,
}

#[derive(Clone, Debug, ValueEnum, Serialize, Deserialize)]
pub enum IpvFamily {
  Ipv4,
  Ipv6,
}

impl Default for IpvFamily {
  fn default() -> Self {
    Self::Ipv4
  }
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
  pub async fn exec(&self) -> Result<()> {
    let config = crate::get_sdk_config(None).await?;
    let imds_data = crate::imds::get_imds_data().await?;
    println!("{:#?}", imds_data);

    let cluster_boostrap = eks::collect_or_get_cluster_bootstrap(config, self, &imds_data.vpc_ipv4_cidr_blocks).await?;
    println!("{:#?}", cluster_boostrap);

    Ok(())
  }
}
