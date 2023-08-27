//! Script-like crate for generating files used by `eksnode` or image creation process

use std::{
  collections::{btree_map, BTreeMap},
  env, fs,
  path::{Path, PathBuf},
};

use anyhow::Result;
use aws_config::SdkConfig;
use aws_sdk_ec2::{
  config::{self, retry::RetryConfig},
  types::InstanceTypeInfo,
  Client,
};
use aws_types::region::Region;
use eksnode::resource::calculate_eni_max_pods;
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio_stream::StreamExt;

#[derive(Debug, Serialize, Deserialize)]
struct Instance {
  /// The default number of vCPUs for the instance
  default_vcpus: i32,

  /// The (theoretical) maximum number of pods
  ///
  /// This is based off the maximum number of ENIs and the maximum number of IPv4 addresses per ENI
  eni_maximum_pods: i32,

  /// The hypervisor (nitro | xen | unknown)
  hypervisor: String,

  /// Indicates whether instance storage is supported
  instance_storage_supported: bool,

  /// The maximum number of IPv4 addresses per ENI
  ipv4_addresses_per_interface: i32,

  /// The maximum number of ENIs
  maximum_network_interfaces: i32,
}

/// Construct and return the EC2 client
async fn get_client(config: SdkConfig, retries: u32) -> Result<Client> {
  let client = Client::from_conf(
    config::Builder::from(&config)
      .retry_config(RetryConfig::standard().with_max_attempts(retries))
      .build(),
  );

  Ok(client)
}

/// Collects all instances and their details from the region provided
async fn get_instances(region: Region) -> Result<Vec<InstanceTypeInfo>> {
  let config = aws_config::from_env().region(region).load().await;
  let client = get_client(config, 3).await.unwrap();

  let results = client
    .describe_instance_types()
    .into_paginator()
    .items()
    .send()
    .collect::<Result<Vec<_>, _>>()
    .await?;

  Ok(results)
}

/// Creates a manually generated map of instances that are missing or faulty
///
/// https://github.com/aws/amazon-vpc-cni-k8s/blob/4bd975383285cc9607f2bde3229bdefe2a44d815/scripts/gen_vpc_ip_limits.go#L193
fn get_manual_instances() -> Result<BTreeMap<String, Instance>> {
  let mut result = BTreeMap::new();
  for inst in vec![
    ("cr1.8xlarge", 32, "unknown", true, 30, 8),
    ("hs1.8xlarge", 16, "unknown", true, 30, 8),
    ("u-12tb1.metal", 448, "unknown", false, 30, 5),
    ("u-18tb1.metal", 448, "unknown", false, 50, 15),
    ("u-24tb1.metal", 448, "unknown", false, 50, 15),
    ("u-6tb1.metal", 448, "unknown", false, 30, 5),
    ("u-9tb1.metal", 448, "unknown", false, 30, 5),
    ("c5a.metal", 96, "unknown", false, 50, 15),
    ("c5ad.metal", 96, "unknown", true, 50, 15),
    ("p4de.24xlarge", 96, "nitro", true, 50, 15),
    ("bmn-sf1.metal", 1, "unknown", false, 50, 15),
  ] {
    let instance_type = inst.0.to_string();
    let instance = Instance {
      default_vcpus: inst.1,
      eni_maximum_pods: calculate_eni_max_pods(inst.5, inst.4, false),
      hypervisor: inst.2.to_string(),
      instance_storage_supported: inst.3,
      ipv4_addresses_per_interface: inst.4,
      maximum_network_interfaces: inst.5,
    };
    result.insert(instance_type, instance);
  }
  Ok(result)
}

/// Writes the max pods per instance type to text file
///
/// This file will be copied to the instance when creating the AMI
fn write_eni_max_pods(instances: &BTreeMap<String, Instance>, regions: Vec<&str>, cur_dir: &Path) -> Result<()> {
  let mut handlebars = Handlebars::new();
  let template = cur_dir.join("eksnode-gen").join("templates").join("eni-max-pods.tpl");
  handlebars.register_template_file("tpl", template)?;

  let data = json!({
    "regions": regions,
    "instances": instances,
  });
  let rendered = handlebars.render("tpl", &data)?;
  let path: PathBuf = ["ami", "playbooks", "roles", "eks", "files", "eni-max-pods.txt"]
    .iter()
    .collect();
  let dest_path = cur_dir.join(path);
  fs::write(dest_path, rendered)?;

  Ok(())
}

/// Writes the EC2 instance details collected to a rust file
///
/// This generates a static map that will be used by eksnode to lookup instance details without the need to re-query the
/// EC2 API
fn write_ec2(instances: &BTreeMap<String, Instance>, cur_dir: &Path) -> Result<()> {
  let mut handlebars = Handlebars::new();
  let template = cur_dir.join("eksnode-gen").join("templates").join("ec2-instances.tpl");
  handlebars.register_template_file("tpl", template)?;

  let data = json!({"instances": instances});
  let rendered = handlebars.render("tpl", &data)?;
  let dest_path = cur_dir.join("eksnode").join("files").join("ec2-instances.yaml");
  fs::write(dest_path, rendered)?;

  Ok(())
}

/// Generates the max pods per instance type file and the EC2 instance details file
///
/// ```bash
/// cargo run --bin eksnode-gen
/// ```
///
/// Based off of the VPC CNI go equivalent:
/// https://github.com/aws/amazon-vpc-cni-k8s/blob/master/scripts/gen_vpc_ip_limits.go
#[tokio::main]
async fn main() -> Result<()> {
  let regions = vec![
    // "ap-northeast-1",
    // "ap-northeast-2",
    // "ap-northeast-3",
    // "ap-south-1",
    // "ap-southeast-1",
    // "ap-southeast-2",
    // "ca-central-1",
    // "eu-central-1",
    // "eu-north-1",
    "eu-west-1",
    // "eu-west-2",
    // "eu-west-3",
    // "sa-east-1",
    "us-east-1",
    // "us-east-2",
    // "us-west-1",
    "us-west-2",
  ];

  // Start with manually inserted instances
  let mut instances = get_manual_instances()?;

  for region in &regions {
    let results = get_instances(Region::new(region.to_owned())).await?;
    let _ = results
      .into_iter()
      .map(|instance| {
        let instance_type = instance.instance_type.as_ref().unwrap();
        let instance_type = instance_type.as_str().to_string();

        if let btree_map::Entry::Vacant(e) = instances.entry(instance_type) {
          let net_info = instance.network_info.as_ref().unwrap();
          let ipv4_addresses = net_info.ipv4_addresses_per_interface.unwrap();

          // only one network card is supported, so use the maximum_network_interfaces from the default card
          let def_net_card_idx = net_info.default_network_card_index.unwrap();
          let network_interfaces = net_info
            .network_cards
            .as_ref()
            .unwrap()
            .get(def_net_card_idx as usize)
            .unwrap()
            .maximum_network_interfaces()
            .unwrap();

          let inst = Instance {
            default_vcpus: instance.v_cpu_info.unwrap().default_v_cpus().unwrap(),
            eni_maximum_pods: calculate_eni_max_pods(network_interfaces, ipv4_addresses, false),
            hypervisor: match instance.hypervisor {
              Some(hypervisor) => hypervisor.as_str().to_owned(),
              None => "unknown".to_string(),
            },
            instance_storage_supported: instance.instance_storage_supported.unwrap(),
            ipv4_addresses_per_interface: ipv4_addresses,
            maximum_network_interfaces: network_interfaces,
          };
          e.insert(inst);
        }
      })
      .collect::<Vec<_>>();
  }

  // Generate files
  let cur_exe = env::current_exe()?;
  let cur_dir = cur_exe.parent().unwrap().parent().unwrap().parent().unwrap();
  write_eni_max_pods(&instances, regions, cur_dir)?;
  write_ec2(&instances, cur_dir)
}
