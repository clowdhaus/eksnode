use std::{fs, io::Write, net::IpAddr, path::PathBuf};

use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use clap::{Args, ValueEnum};
use ipnet::IpNet;
use rand::{seq::SliceRandom, thread_rng};
use semver::Version;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::{commands, containerd, ec2, ecr, eks, imds, kubelet, resource, utils};

#[derive(Args, Debug, Default, Serialize, Deserialize)]
pub struct Node {
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

  /// The ID of your local Amazon EKS cluster on an Amazon Web Services Outpost
  #[arg(long)]
  pub cluster_id: Option<String>,

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

struct KubeletKubeConfig {
  config: kubelet::KubeConfig,
  path: PathBuf,
}

impl Node {
  /// Get the cluster details required to join the node to the cluster
  async fn get_cluster(&self) -> Result<eks::Cluster> {
    let config = crate::get_sdk_config(None).await?;
    let imds_data = crate::imds::get_imds_data().await?;
    debug!("Instance metadata: {imds_data:#?}");

    // Details required to join node to cluster
    let cluster = eks::collect_or_get_cluster(config, self, &imds_data.vpc_ipv4_cidr_blocks).await?;
    debug!("Node details: {cluster:#?}");

    Ok(cluster)
  }

  /// Get the configuration for kubelet
  fn get_kubelet_config(
    &self,
    dns_cluster_ip: IpAddr,
    max_pods: i32,
    kubelet_version: &Version,
  ) -> Result<kubelet::KubeletConfiguration> {
    let mebibytes_to_reserve = resource::memory_mebibytes_to_reserve(max_pods)?;
    let cpu_millicores_to_reserve = resource::cpu_millicores_to_reserve(max_pods, num_cpus::get() as i32)?;

    let mut config: kubelet::KubeletConfiguration =
      kubelet::KubeletConfiguration::new(dns_cluster_ip, mebibytes_to_reserve, cpu_millicores_to_reserve);

    // Increase the API priority and fairness for the K8s versions that support it.
    // Starting with 1.27, the default is already increased to 50/100, so leave the higher defaults
    if kubelet_version.ge(&Version::parse("1.22.0")?) && kubelet_version.lt(&Version::parse("1.27.0")?) {
      config.kube_api_qps = Some(10);
      config.kube_api_burst = Some(20);
    }

    match kubelet_version.lt(&Version::parse("1.26.0")?) {
      true => config.provider_id = Some("aws".to_string()),
      false => config.provider_id = Some("external".to_string()),
    }

    if self.use_max_pods {
      config.max_pods = Some(max_pods);
    }

    Ok(config)
  }

  /// Get the kubeconfig for kubelet
  ///
  /// If cluster is local cluster on Outpost, use aws-iam-authenticator as bootstrap auth for kubelet
  /// TLS bootstrapping which downloads client X.509 certificate and generates kubelet kubeconfig file
  /// which uses the client cert. This allows the worker node can be authenticated through
  /// X.509 certificate which works for both connected and disconnected states.
  fn get_kubelet_kubeconfig(&self, cluster: &eks::Cluster, region: &str) -> Result<KubeletKubeConfig> {
    let name = match self.is_local_cluster {
      true => self
        .cluster_id
        .as_ref()
        .expect("Cluster ID is required when your local Amazon EKS cluster is on an Amazon Web Services Outpost"),
      false => &cluster.name,
    };

    let path = match self.is_local_cluster {
      true => "/var/lib/kubelet/bootstrap-kubeconfig",
      false => "/var/lib/kubelet/kubeconfig",
    };

    let config = kubelet::KubeConfig::new(&cluster.endpoint, name, region)?;

    Ok(KubeletKubeConfig {
      config,
      path: PathBuf::from(path),
    })
  }

  async fn get_containerd_config(&self, imds: imds::InstanceMetadata) -> Result<containerd::ContainerdConfiguration> {
    let uri = format!("{}/eks/pause:3.9", ecr::get_ecr_uri(&imds.region, &imds.domain, false)?);
    let sandbox_img = match &self.pause_container_image {
      Some(img) => img,
      None => &uri,
    };

    let config = containerd::ContainerdConfiguration::new(sandbox_img)?;

    Ok(config)
  }

  /// Decode the base64 encoded CA certificate and write it to disk
  fn write_ca_cert(&self, base64_ca: &str) -> Result<()> {
    let decoded = general_purpose::STANDARD_NO_PAD.decode(base64_ca)?;

    utils::write_file(&decoded, None, "/etc/kubernetes/pki/ca.crt")
  }

  /// Update /etc/hosts for the cluster endpoint IPs for Outpost local cluster
  fn update_etc_hosts(&self, endpoint: &str, path: PathBuf) -> Result<()> {
    let mut hostfile = fs::OpenOptions::new().append(true).open(path)?;
    let mut ips: Vec<IpAddr> = dns_lookup::lookup_host(endpoint)?;

    // Shuffle the IPs to avoid always using the first IP
    ips.shuffle(&mut thread_rng());
    let entries: Vec<String> = ips.iter().map(|ip| format!("{ip} {endpoint}\n")).collect();

    hostfile
      .write_all(entries.join("").as_bytes())
      .map_err(anyhow::Error::from)
  }

  /// Get the max pods for the instance
  async fn get_max_pods(&self, instance_type: &str) -> Result<i32> {
    match ec2::INSTANCES.get(instance_type) {
      Some(instance) => Ok(instance.eni_maximum_pods),
      None => {
        info!("Instance type {instance_type} not found in static instance data. Attempting to derive max pods");

        let max_pods = commands::calc::MaxPods {
          instance_type: Some(instance_type.to_owned()),
          instance_type_from_imds: false,
          cni_version: "1.10.0".to_owned(),
          cni_custom_networking_enabled: false,
          cni_prefix_delegation_enabled: false,
          cni_max_enis: None,
        };
        max_pods.calc().await
      }
    }
  }

  /// Configure the node to join the cluster
  pub async fn join_node_to_cluster(&self) -> Result<()> {
    let instance_metadata = imds::get_imds_data().await?;
    let cluster = self.get_cluster().await?;
    let kubelet_version = kubelet::get_kubelet_version()?;
    let max_pods = self.get_max_pods(&instance_metadata.instance_type).await?;

    self.write_ca_cert(&cluster.b64_ca)?;
    if self.is_local_cluster {
      self.update_etc_hosts(&cluster.endpoint, PathBuf::from("/etc/hosts"))?;
    }

    let kubelet_kubeconfig = self.get_kubelet_kubeconfig(&cluster, &instance_metadata.region)?;
    kubelet_kubeconfig.config.write(kubelet_kubeconfig.path, Some(0))?;

    let kubelet_config = self.get_kubelet_config(cluster.dns_cluster_ip, max_pods, &kubelet_version)?;
    kubelet_config.write("/etc/kubernetes/kubelet/kubelet-config.json", Some(0))?;

    let containerd_config = self.get_containerd_config(instance_metadata).await?;
    containerd_config.write("/etc/containerd/config.toml")?;

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use std::net::{IpAddr, Ipv4Addr};

  use super::*;

  #[test]
  fn it_gets_kubelet_config_122() {
    let cluster = Node {
      use_max_pods: true,
      ..Node::default()
    };

    let kubelet_config = cluster
      .get_kubelet_config(
        IpAddr::V4(Ipv4Addr::new(10, 1, 0, 10)),
        110,
        &Version::parse("1.22.0").unwrap(),
      )
      .unwrap();

    assert_eq!(kubelet_config.kube_api_qps, Some(10));
    assert_eq!(kubelet_config.kube_api_burst, Some(20));
    assert_eq!(kubelet_config.provider_id, Some("aws".to_string()));
    assert_eq!(kubelet_config.max_pods, Some(110));
  }

  #[test]
  fn it_gets_kubelet_config_126() {
    let cluster = Node::default();

    let kubelet_config = cluster
      .get_kubelet_config(
        IpAddr::V4(Ipv4Addr::new(10, 1, 0, 10)),
        110,
        &Version::parse("1.26.0").unwrap(),
      )
      .unwrap();

    assert_eq!(kubelet_config.kube_api_qps, Some(10));
    assert_eq!(kubelet_config.kube_api_burst, Some(20));
    assert_eq!(kubelet_config.provider_id, Some("external".to_string()));
    assert_eq!(kubelet_config.max_pods, None);
  }

  #[test]
  fn it_gets_kubelet_config_127() {
    let cluster = Node::default();

    let kubelet_config = cluster
      .get_kubelet_config(
        IpAddr::V4(Ipv4Addr::new(10, 1, 0, 10)),
        110,
        &Version::parse("1.27.0").unwrap(),
      )
      .unwrap();

    assert_eq!(kubelet_config.kube_api_qps, None);
    assert_eq!(kubelet_config.kube_api_burst, None);
    assert_eq!(kubelet_config.provider_id, Some("external".to_string()));
    assert_eq!(kubelet_config.max_pods, None);
  }

  #[test]
  fn it_gets_kubelet_kubeconfig_local() {
    let node = Node {
      is_local_cluster: true,
      cluster_id: Some("6B29FC40-CA47-1067-B31D-00DD010662DA".to_string()),
      ..Node::default()
    };

    let cluster = eks::Cluster {
      name: "example".to_string(),
      endpoint: "http://localhost:8080".to_string(),
      b64_ca: "c3VwZXIgc2VjcmV0IGNsdXN0ZXIgY2VydGlmaWNhdGU".to_string(),
      is_local_cluster: true,
      dns_cluster_ip: IpAddr::V4(Ipv4Addr::new(10, 1, 0, 10)),
    };

    let kubelet_kubeconfig = node.get_kubelet_kubeconfig(&cluster, "us-west-2").unwrap();

    assert_eq!(
      kubelet_kubeconfig.path,
      PathBuf::from("/var/lib/kubelet/bootstrap-kubeconfig")
    );
    insta::assert_debug_snapshot!(kubelet_kubeconfig.config);
  }

  #[test]
  fn it_gets_kubelet_kubeconfig_eks() {
    let node = Node::default();
    let cluster = eks::Cluster {
      name: "example".to_string(),
      endpoint: "http://localhost:8080".to_string(),
      b64_ca: "c3VwZXIgc2VjcmV0IGNsdXN0ZXIgY2VydGlmaWNhdGU".to_string(),
      is_local_cluster: false,
      dns_cluster_ip: IpAddr::V4(Ipv4Addr::new(10, 1, 0, 10)),
    };

    let kubelet_kubeconfig = node.get_kubelet_kubeconfig(&cluster, "eu-west-1").unwrap();

    assert_eq!(kubelet_kubeconfig.path, PathBuf::from("/var/lib/kubelet/kubeconfig"));
    insta::assert_debug_snapshot!(kubelet_kubeconfig.config);
  }
}
