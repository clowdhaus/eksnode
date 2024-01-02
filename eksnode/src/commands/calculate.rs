use anyhow::{anyhow, Result};
use clap::Args;
use serde::{Deserialize, Serialize};

use crate::{ec2, resource};

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(group = clap::ArgGroup::new("instance-type").multiple(false).required(true))]
pub struct CalculateMaxPodsInput {
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

impl CalculateMaxPodsInput {
  pub async fn calculate(&self) -> Result<i32> {
    let instance_type = if self.instance_type_from_imds {
      ec2::get_instance_type().await?
    } else {
      self.instance_type.to_owned().unwrap()
    };
    let instance = match ec2::get_instance(&instance_type)? {
      Some(instance) => instance,
      None => return Err(anyhow!("Instance type {instance_type} is not supported or invalid")),
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

    Ok(result)
  }

  pub async fn result(&self) -> Result<()> {
    let result = self.calculate().await?;

    println!("{result}");

    Ok(())
  }
}
