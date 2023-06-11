use phf::phf_map;

#[derive(Debug)]
pub struct Instance {
  pub instance_storage_supported: bool,
  pub maximum_network_interfaces: i32,
  pub ipv4_addresses_per_interface: i32,
  pub maximum_pods: i32,
}

pub static MAP: phf::Map<&'static str, Instance> = phf_map! {
{{ #each instances as |instance| }}
  "{{ @key }}" => Instance {
    instance_storage_supported: {{ instance.instance_storage_supported }},
    maximum_network_interfaces: {{ instance.maximum_network_interfaces }},
    ipv4_addresses_per_interface: {{ instance.ipv4_addresses_per_interface }},
    maximum_pods: {{ instance.maximum_pods }},
  },
{{ /each }}
};
