//! Do not manually edit - this file is automatically generated with:
//! cargo run --bin eksnode-gen
//!
//! EC2 instance types and the properties used by `eksnode`
//! This is a static map of instances to avoid API calls when adding the node to the cluster
use phf::phf_map;

#[derive(Debug)]
pub struct Instance {
  /// The default number of vCPUs for the instance
  pub default_vcpus: i32,

  /// The (theoretical) maximum number of pods
  ///
  /// This is based off the maximum number of ENIs and the maximum number of IPv4 addresses per ENI
  pub eni_maximum_pods: i32,

  /// The hypervisor (nitro | xen | unknown)
  pub hypervisor: &'static str,

  /// Indicates whether instance storage is supported
  pub instance_storage_supported: bool,

  /// The maximum number of IPv4 addresses per ENI
  pub ipv4_addresses_per_interface: i32,

  /// The maximum number of ENIs
  pub maximum_network_interfaces: i32,
}

pub static INSTANCES: phf::Map<&'static str, Instance> = phf_map! {
{{ #each instances as |instance| }}
  "{{ @key }}" => Instance {
    default_vcpus: {{ instance.default_vcpus }},
    eni_maximum_pods: {{ instance.eni_maximum_pods }},
    hypervisor: "{{ instance.hypervisor }}",
    instance_storage_supported: {{ instance.instance_storage_supported }},
    ipv4_addresses_per_interface: {{ instance.ipv4_addresses_per_interface }},
    maximum_network_interfaces: {{ instance.maximum_network_interfaces }},
  },
{{ /each }}
};