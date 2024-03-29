use std::{
  collections::{btree_map, BTreeMap},
  fs,
  path::Path,
};

use anyhow::Result;
use aws_sdk_ec2::types::InstanceTypeInfo;
use aws_types::region::Region;
use eksnode::{ec2::Instance, resource::calculate_eni_max_pods};
use handlebars::Handlebars;
use serde_json::json;

/// Collects all instances and their details from the region provided
async fn get_instances(region: Region) -> Result<Vec<InstanceTypeInfo>> {
  // Using region specific client to pull instance data for that region
  let config = aws_config::from_env().region(region).load().await;
  let client = crate::get_client(config, 3).await.unwrap();

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
    ("cr1.8xlarge", 32, "unknown", true, 30, 8, "None"),
    ("hs1.8xlarge", 16, "unknown", true, 30, 8, "None"),
    ("u-12tb1.metal", 448, "unknown", false, 30, 5, "None"),
    ("u-18tb1.metal", 448, "unknown", false, 50, 15, "None"),
    ("u-24tb1.metal", 448, "unknown", false, 50, 15, "None"),
    ("u-6tb1.metal", 448, "unknown", false, 30, 5, "None"),
    ("u-9tb1.metal", 448, "unknown", false, 30, 5, "None"),
    ("c5a.metal", 96, "unknown", false, 50, 15, "None"),
    ("c5ad.metal", 96, "unknown", true, 50, 15, "None"),
    ("p4de.24xlarge", 96, "nitro", true, 50, 15, "NVIDIA"),
    ("bmn-sf1.metal", 1, "unknown", false, 50, 15, "None"),
  ] {
    let instance_type = inst.0.to_string();
    let instance = Instance {
      default_vcpus: inst.1,
      gpu_manufacturer: inst.6.to_string(),
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

pub async fn write_files(cur_dir: &Path) -> Result<()> {
  let regions = vec!["us-east-1", "us-west-2"];

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

          let gpu_manufacturer = match instance.gpu_info.as_ref() {
            Some(gpu_info) => gpu_info
              .gpus
              .as_ref()
              .unwrap()
              .first()
              .unwrap()
              .manufacturer
              .as_ref()
              .unwrap()
              .to_string(),
            None => "none".to_string(),
          };

          let inst = Instance {
            default_vcpus: instance.v_cpu_info.unwrap().default_v_cpus().unwrap(),
            eni_maximum_pods: calculate_eni_max_pods(network_interfaces, ipv4_addresses, false),
            gpu_manufacturer,
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

  write_ec2(&instances, cur_dir)
}
