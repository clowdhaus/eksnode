use std::{
  collections::{btree_map, BTreeMap},
  env, fs,
  path::Path,
};

use anyhow::Result;
use aws_config::SdkConfig;
use aws_sdk_ec2::{
  config::{self, retry::RetryConfig},
  types::InstanceTypeInfo,
  Client,
};
use aws_types::region::Region;
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio_stream::StreamExt;

#[derive(Debug, Serialize, Deserialize)]
struct Instance {
  instance_storage_supported: bool,
  maximum_network_interfaces: i32,
  ipv4_addresses_per_interface: i32,
  maximum_pods: i32,
}

/// Get the EC2 client
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

/// TODO - move to eksami
fn calc_max_pods(network_interfaces: i32, ipv4_addresses: i32) -> i32 {
  // # of ENI * (# of IPv4 per ENI - 1) + 2
  network_interfaces * (ipv4_addresses - 1) + 2
}

/// Creates a manually generated map of instances that are missing or faulty
///
/// https://github.com/aws/amazon-vpc-cni-k8s/blob/4bd975383285cc9607f2bde3229bdefe2a44d815/scripts/gen_vpc_ip_limits.go#L193
fn get_manual_instances() -> Result<BTreeMap<String, Instance>> {
  let mut result = BTreeMap::new();
  for inst in vec![
    ("cr1.8xlarge", false, 8, 30),
    ("hs1.8xlarge", false, 8, 30),
    ("u-12tb1.metal", false, 5, 30),
    ("u-18tb1.metal", false, 15, 50),
    ("u-24tb1.metal", false, 15, 50),
    ("u-6tb1.metal", false, 5, 30),
    ("u-9tb1.metal", false, 5, 30),
    ("c5a.metal", false, 15, 50),
    ("c5ad.metal", true, 15, 50),
    ("p4de.24xlarge", true, 15, 50),
    ("bmn-sf1.metal", false, 15, 50),
  ] {
    let instance_type = inst.0.to_string();
    let instance = Instance {
      instance_storage_supported: inst.1,
      maximum_network_interfaces: inst.2,
      ipv4_addresses_per_interface: inst.3,
      maximum_pods: calc_max_pods(inst.2, inst.3),
    };
    result.insert(instance_type, instance);
  }
  Ok(result)
}

/// Writes the max pods per instance type to text file
///
/// This file will be copied to the instance when creating the AMI
fn write_max_pods(instances: &BTreeMap<String, Instance>, regions: Vec<&str>, cur_dir: &Path) -> Result<()> {
  let mut handlebars = Handlebars::new();
  let template = cur_dir.join("eksami-gen").join("templates").join("eni-max-pods.tpl");
  handlebars.register_template_file("tpl", template)?;

  let data = json!({
    "regions": regions,
    "instances": instances,
  });
  let rendered = handlebars.render("tpl", &data)?;
  let dest_path = cur_dir.join("files").join("eni-max-pods.txt");
  fs::write(dest_path, rendered)?;

  Ok(())
}

/// Writes the EC2 instance details collected to a rust file
///
/// This generates a static map that will be used by eksami to lookup instance details
/// without the need to re-query the EC2 API
fn write_ec2_instances(instances: &BTreeMap<String, Instance>, cur_dir: &Path) -> Result<()> {
  let mut handlebars = Handlebars::new();
  let template = cur_dir.join("eksami-gen").join("templates").join("ec2_instances.tpl");
  handlebars.register_template_file("tpl", template)?;

  let data = json!({"instances": instances});
  let rendered = handlebars.render("tpl", &data)?;
  let dest_path = cur_dir.join("eksami").join("src").join("ec2_instances.rs");
  fs::write(dest_path, rendered)?;

  Ok(())
}

/// Generates the max pods per instance type file and the EC2 instance details file
///
/// If running from eksami-gen directory:
/// ```bash
/// cargo run
/// ```
///
/// If running from other locations within the project:
/// ```bash
/// cargo run --bin eksami-gen
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
            instance_storage_supported: instance.instance_storage_supported.unwrap(),
            maximum_network_interfaces: network_interfaces,
            ipv4_addresses_per_interface: ipv4_addresses,
            maximum_pods: calc_max_pods(network_interfaces, ipv4_addresses),
          };
          e.insert(inst);
        }
      })
      .collect::<Vec<_>>();
  }

  // Generate files
  let cur_exe = env::current_exe()?;
  let cur_dir = cur_exe.parent().unwrap().parent().unwrap().parent().unwrap();
  write_max_pods(&instances, regions, cur_dir)?;
  write_ec2_instances(&instances, cur_dir)?;

  Ok(())
}
