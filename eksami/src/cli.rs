use std::net::IpAddr;

use anyhow::{anyhow, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_verbosity_flag::Verbosity;
use ipnet::IpNet;
use serde::{Deserialize, Serialize};

use crate::{ec2, eks, resource};

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

  /// Calculate the maximum number of pods that can be scheduled on an instance
  ///
  /// Unlike `calculate_eni_max_pods` which calculates the theoretical limit based on ENIs,
  /// this function calculates the actual limit based on all of the preceding factors including
  /// the theoretical max pods limit.
  CalcMaxPods(MaxPods),
}

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

  /// Mount a bpf filesystem at /sys/fs/bpf (default: true)
  #[arg(long, default_value = "true")]
  pub mount_bpf_fs: bool,

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

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(group = clap::ArgGroup::new("instance-type").multiple(false).required(true))]
pub struct MaxPods {
  /// The instance type used to calculate max pods
  #[arg(short, long, group = "instance-type")]
  pub instance_type: Option<String>,

  /// The instance type is be queried from the instance metadata service
  #[arg(long, group = "instance-type")]
  pub instance_type_from_imds: bool,

  /// The version of the VPC-CNI (i.e. -  v1.12.6-eksbuild.2 or 1.12.6)
  #[arg(long)]
  pub cni_version: String,

  /// VPC-CNI custom networking is enabled
  #[arg(long)]
  pub cni_custom_networking_enabled: bool,

  /// VPC-CNI prefix-delegation is enabled
  #[arg(long)]
  pub cni_prefix_delegation_enabled: bool,

  /// The max number of ENIs used for prefix delegation
  ///
  /// Defaults to using all ENIs available to the instance
  #[arg(long)]
  pub cni_max_enis: Option<i32>,
}

impl MaxPods {
  pub async fn calc(&self) -> Result<()> {
    let instance_type = if self.instance_type_from_imds {
      crate::imds::get_instance_type().await?
    } else {
      self.instance_type.to_owned().unwrap()
    };
    let instance = match ec2::INSTANCES.get(&instance_type) {
      Some(instance) => instance,
      None => return Err(anyhow!("Instance type {} is not supported or invalid", &instance_type)),
    };

    let prefix_supported = resource::prefix_delegation_supported(&self.cni_version)?;

    // Take the min of either the number of ENIs passed by the CLI or the number of ENIs available to the instance
    let mut num_enis = match self.cni_max_enis {
      Some(enis) => std::cmp::min(instance.maximum_network_interfaces, enis),
      None => instance.maximum_network_interfaces,
    };

    if self.cni_custom_networking_enabled {
      // If custom networking is enabled, we need to reserve an ENI for the CNI
      num_enis -= 1;
    }

    let use_prefix_del = instance.hypervisor == "nitro" && prefix_supported && self.cni_prefix_delegation_enabled;
    let max_pods = resource::calculate_eni_max_pods(num_enis, instance.ipv4_addresses_per_interface, use_prefix_del);

    let result = match instance.default_vcpus > 30 {
      true => std::cmp::min(250, max_pods),
      _ => std::cmp::min(110, max_pods),
    };

    println!("{result}");

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use assert_cmd::prelude::*;
  use rstest::*;

  #[rstest]
  #[case("c6g.medium", "1.8.0", true, false, "5\n")]
  #[case("c6g.medium", "1.8.0", false, true, "8\n")]
  #[case("c6g.medium", "1.8.0", true, true, "5\n")]
  #[case("c6g.medium", "1.8.0", false, false, "8\n")]
  #[case("c6g.medium", "1.9.0", true, false, "5\n")]
  #[case("c6g.medium", "1.9.0", false, true, "98\n")]
  #[case("c6g.medium", "1.9.0", true, true, "50\n")]
  #[case("c6g.medium", "1.9.0", false, false, "8\n")]
  #[case("c5.large", "1.8.0", true, false, "20\n")]
  #[case("c5.large", "1.8.0", false, true, "29\n")]
  #[case("c5.large", "1.8.0", true, true, "20\n")]
  #[case("c5.large", "1.8.0", false, false, "29\n")]
  #[case("c5.large", "1.9.0", true, false, "20\n")]
  #[case("c5.large", "1.9.0", false, true, "110\n")]
  #[case("c5.large", "1.9.0", true, true, "110\n")]
  #[case("c5.large", "1.9.0", false, false, "29\n")]
  #[case("c5.xlarge", "1.8.0", true, false, "44\n")]
  #[case("c5.xlarge", "1.8.0", false, true, "58\n")]
  #[case("c5.xlarge", "1.8.0", true, true, "44\n")]
  #[case("c5.xlarge", "1.8.0", false, false, "58\n")]
  #[case("c5.xlarge", "1.9.0", true, false, "44\n")]
  #[case("c5.xlarge", "1.9.0", false, true, "110\n")]
  #[case("c5.xlarge", "1.9.0", true, true, "110\n")]
  #[case("c5.xlarge", "1.9.0", false, false, "58\n")]
  #[case("c5.4xlarge", "1.8.0", true, false, "110\n")]
  #[case("c5.4xlarge", "1.8.0", false, true, "110\n")]
  #[case("c5.4xlarge", "1.8.0", true, true, "110\n")]
  #[case("c5.4xlarge", "1.8.0", false, false, "110\n")]
  #[case("c5.4xlarge", "1.9.0", true, false, "110\n")]
  #[case("c5.4xlarge", "1.9.0", false, true, "110\n")]
  #[case("c5.4xlarge", "1.9.0", true, true, "110\n")]
  #[case("c5.4xlarge", "1.9.0", false, false, "110\n")]
  #[case("c6in.32xlarge", "1.8.0", true, false, "250\n")]
  #[case("c6in.32xlarge", "1.8.0", false, true, "250\n")]
  #[case("c6in.32xlarge", "1.8.0", true, true, "250\n")]
  #[case("c6in.32xlarge", "1.8.0", false, false, "250\n")]
  #[case("c6in.32xlarge", "1.9.0", true, false, "250\n")]
  #[case("c6in.32xlarge", "1.9.0", false, true, "250\n")]
  #[case("c6in.32xlarge", "1.9.0", true, true, "250\n")]
  #[case("c6in.32xlarge", "1.9.0", false, false, "250\n")]
  #[case("c5.18xlarge", "1.8.0", true, false, "250\n")]
  #[case("c5.18xlarge", "1.8.0", false, true, "250\n")]
  #[case("c5.18xlarge", "1.8.0", true, true, "250\n")]
  #[case("c5.18xlarge", "1.8.0", false, false, "250\n")]
  #[case("c5.18xlarge", "1.9.0", true, false, "250\n")]
  #[case("c5.18xlarge", "1.9.0", false, true, "250\n")]
  #[case("c5.18xlarge", "1.9.0", true, true, "250\n")]
  #[case("c5.18xlarge", "1.9.0", false, false, "250\n")]
  #[case("d3.2xlarge", "1.8.0", true, false, "14\n")]
  #[case("d3.2xlarge", "1.8.0", false, true, "18\n")]
  #[case("d3.2xlarge", "1.8.0", true, true, "14\n")]
  #[case("d3.2xlarge", "1.8.0", false, false, "18\n")]
  #[case("d3.2xlarge", "1.9.0", true, false, "14\n")]
  #[case("d3.2xlarge", "1.9.0", false, true, "110\n")]
  #[case("d3.2xlarge", "1.9.0", true, true, "110\n")]
  #[case("d3.2xlarge", "1.9.0", false, false, "18\n")]
  #[case("d3.4xlarge", "1.8.0", true, false, "29\n")]
  #[case("d3.4xlarge", "1.8.0", false, true, "38\n")]
  #[case("d3.4xlarge", "1.8.0", true, true, "29\n")]
  #[case("d3.4xlarge", "1.8.0", false, false, "38\n")]
  #[case("d3.4xlarge", "1.9.0", true, false, "29\n")]
  #[case("d3.4xlarge", "1.9.0", false, true, "110\n")]
  #[case("d3.4xlarge", "1.9.0", true, true, "110\n")]
  #[case("d3.4xlarge", "1.9.0", false, false, "38\n")]
  #[case("d3.8xlarge", "1.8.0", true, false, "40\n")]
  #[case("d3.8xlarge", "1.8.0", false, true, "59\n")]
  #[case("d3.8xlarge", "1.8.0", true, true, "40\n")]
  #[case("d3.8xlarge", "1.8.0", false, false, "59\n")]
  #[case("d3.8xlarge", "1.9.0", true, false, "40\n")]
  #[case("d3.8xlarge", "1.9.0", false, true, "250\n")]
  #[case("d3.8xlarge", "1.9.0", true, true, "250\n")]
  #[case("d3.8xlarge", "1.9.0", false, false, "59\n")]
  #[case("d3.xlarge", "1.8.0", true, false, "8\n")]
  #[case("d3.xlarge", "1.8.0", false, true, "10\n")]
  #[case("d3.xlarge", "1.8.0", true, true, "8\n")]
  #[case("d3.xlarge", "1.8.0", false, false, "10\n")]
  #[case("d3.xlarge", "1.9.0", true, false, "8\n")]
  #[case("d3.xlarge", "1.9.0", false, true, "110\n")]
  #[case("d3.xlarge", "1.9.0", true, true, "98\n")]
  #[case("d3.xlarge", "1.9.0", false, false, "10\n")]
  #[case("d3en.8xlarge", "1.8.0", true, false, "59\n")]
  #[case("d3en.8xlarge", "1.8.0", false, true, "78\n")]
  #[case("d3en.8xlarge", "1.8.0", true, true, "59\n")]
  #[case("d3en.8xlarge", "1.8.0", false, false, "78\n")]
  #[case("d3en.8xlarge", "1.9.0", true, false, "59\n")]
  #[case("d3en.8xlarge", "1.9.0", false, true, "250\n")]
  #[case("d3en.8xlarge", "1.9.0", true, true, "250\n")]
  #[case("d3en.8xlarge", "1.9.0", false, false, "78\n")]
  #[case("f1.16xlarge", "1.8.0", true, false, "250\n")]
  #[case("f1.16xlarge", "1.8.0", false, true, "250\n")]
  #[case("f1.16xlarge", "1.8.0", true, true, "250\n")]
  #[case("f1.16xlarge", "1.8.0", false, false, "250\n")]
  #[case("f1.16xlarge", "1.9.0", true, false, "250\n")]
  #[case("f1.16xlarge", "1.9.0", false, true, "250\n")]
  #[case("f1.16xlarge", "1.9.0", true, true, "250\n")]
  #[case("f1.16xlarge", "1.9.0", false, false, "250\n")]
  #[case("g5g.4xlarge", "1.8.0", true, false, "110\n")]
  #[case("g5g.4xlarge", "1.8.0", false, true, "110\n")]
  #[case("g5g.4xlarge", "1.8.0", true, true, "110\n")]
  #[case("g5g.4xlarge", "1.8.0", false, false, "110\n")]
  #[case("g5g.4xlarge", "1.9.0", true, false, "110\n")]
  #[case("g5g.4xlarge", "1.9.0", false, true, "110\n")]
  #[case("g5g.4xlarge", "1.9.0", true, true, "110\n")]
  #[case("g5g.4xlarge", "1.9.0", false, false, "110\n")]
  #[case("g5g.xlarge", "1.8.0", true, false, "44\n")]
  #[case("g5g.xlarge", "1.8.0", false, true, "58\n")]
  #[case("g5g.xlarge", "1.8.0", true, true, "44\n")]
  #[case("g5g.xlarge", "1.8.0", false, false, "58\n")]
  #[case("g5g.xlarge", "1.9.0", true, false, "44\n")]
  #[case("g5g.xlarge", "1.9.0", false, true, "110\n")]
  #[case("g5g.xlarge", "1.9.0", true, true, "110\n")]
  #[case("g5g.xlarge", "1.9.0", false, false, "58\n")]
  #[case("inf1.24xlarge", "1.8.0", true, false, "250\n")]
  #[case("inf1.24xlarge", "1.8.0", false, true, "250\n")]
  #[case("inf1.24xlarge", "1.8.0", true, true, "250\n")]
  #[case("inf1.24xlarge", "1.8.0", false, false, "250\n")]
  #[case("inf1.24xlarge", "1.9.0", true, false, "250\n")]
  #[case("inf1.24xlarge", "1.9.0", false, true, "250\n")]
  #[case("inf1.24xlarge", "1.9.0", true, true, "250\n")]
  #[case("inf1.24xlarge", "1.9.0", false, false, "250\n")]
  #[case("trn1.32xlarge", "1.8.0", true, false, "198\n")]
  #[case("trn1.32xlarge", "1.8.0", false, true, "247\n")]
  #[case("trn1.32xlarge", "1.8.0", true, true, "198\n")]
  #[case("trn1.32xlarge", "1.8.0", false, false, "247\n")]
  #[case("trn1.32xlarge", "1.9.0", true, false, "198\n")]
  #[case("trn1.32xlarge", "1.9.0", false, true, "250\n")]
  #[case("trn1.32xlarge", "1.9.0", true, true, "250\n")]
  #[case("trn1.32xlarge", "1.9.0", false, false, "247\n")]
  #[case("m1.medium", "1.8.0", true, false, "7\n")]
  #[case("m1.medium", "1.8.0", false, true, "12\n")]
  #[case("m1.medium", "1.8.0", true, true, "7\n")]
  #[case("m1.medium", "1.8.0", false, false, "12\n")]
  #[case("m1.medium", "1.9.0", true, false, "7\n")]
  #[case("m1.medium", "1.9.0", false, true, "12\n")]
  #[case("m1.medium", "1.9.0", true, true, "7\n")]
  #[case("m1.medium", "1.9.0", false, false, "12\n")]
  #[case("m4.large", "1.8.0", true, false, "11\n")]
  #[case("m4.large", "1.8.0", false, true, "20\n")]
  #[case("m4.large", "1.8.0", true, true, "11\n")]
  #[case("m4.large", "1.8.0", false, false, "20\n")]
  #[case("m4.large", "1.9.0", true, false, "11\n")]
  #[case("m4.large", "1.9.0", false, true, "20\n")]
  #[case("m4.large", "1.9.0", true, true, "11\n")]
  #[case("m4.large", "1.9.0", false, false, "20\n")]
  #[case("t1.micro", "1.8.0", true, false, "3\n")]
  #[case("t1.micro", "1.8.0", false, true, "4\n")]
  #[case("t1.micro", "1.8.0", true, true, "3\n")]
  #[case("t1.micro", "1.8.0", false, false, "4\n")]
  #[case("t1.micro", "1.9.0", true, false, "3\n")]
  #[case("t1.micro", "1.9.0", false, true, "4\n")]
  #[case("t1.micro", "1.9.0", true, true, "3\n")]
  #[case("t1.micro", "1.9.0", false, false, "4\n")]
  #[case("t2.large", "1.8.0", true, false, "24\n")]
  #[case("t2.large", "1.8.0", false, true, "35\n")]
  #[case("t2.large", "1.8.0", true, true, "24\n")]
  #[case("t2.large", "1.8.0", false, false, "35\n")]
  #[case("t2.large", "1.9.0", true, false, "24\n")]
  #[case("t2.large", "1.9.0", false, true, "35\n")]
  #[case("t2.large", "1.9.0", true, true, "24\n")]
  #[case("t2.large", "1.9.0", false, false, "35\n")]
  #[case("t2.medium", "1.8.0", true, false, "12\n")]
  #[case("t2.medium", "1.8.0", false, true, "17\n")]
  #[case("t2.medium", "1.8.0", true, true, "12\n")]
  #[case("t2.medium", "1.8.0", false, false, "17\n")]
  #[case("t2.medium", "1.9.0", true, false, "12\n")]
  #[case("t2.medium", "1.9.0", false, true, "17\n")]
  #[case("t2.medium", "1.9.0", true, true, "12\n")]
  #[case("t2.medium", "1.9.0", false, false, "17\n")]
  #[case("t2.small", "1.8.0", true, false, "8\n")]
  #[case("t2.small", "1.8.0", false, true, "11\n")]
  #[case("t2.small", "1.8.0", true, true, "8\n")]
  #[case("t2.small", "1.8.0", false, false, "11\n")]
  #[case("t2.small", "1.9.0", true, false, "8\n")]
  #[case("t2.small", "1.9.0", false, true, "11\n")]
  #[case("t2.small", "1.9.0", true, true, "8\n")]
  #[case("t2.small", "1.9.0", false, false, "11\n")]
  fn calc_max_pods_test(
    #[case] instance_type: &str,
    #[case] cni_version: &str,
    #[case] custom_networking: bool,
    #[case] prefix_delegation: bool,
    #[case] expected: String,
  ) {
    let bin_under_test = escargot::CargoBuild::new()
      .bin("eksami")
      .current_release()
      .current_target()
      .run()
      .unwrap();

    let mut cmd = bin_under_test.command();

    cmd
      .arg("calc-max-pods")
      .arg("--instance-type")
      .arg(instance_type)
      .arg("--cni-version")
      .arg(cni_version);

    if custom_networking {
      cmd.arg("--cni-custom-networking-enabled");
    }

    if prefix_delegation {
      cmd.arg("--cni-prefix-delegation-enabled");
    }

    cmd.assert().success().stdout(expected);
  }
}
