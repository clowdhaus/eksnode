use std::{
  fs,
  io::Write,
  net::IpAddr,
  path::{Path, PathBuf},
};

use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use clap::{Args, ValueEnum};
use ipnet::IpNet;
use rand::{seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::{ec2, eks, imds, kubelet};

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
  async fn get_cluster_bootstrap(&self) -> Result<eks::ClusterBootstrap> {
    let config = crate::get_sdk_config(None).await?;
    let imds_data = crate::imds::get_imds_data().await?;
    debug!("Instance metadata: {:#?}", imds_data);

    // Details required to join node to cluster
    let cluster_boostrap = eks::collect_or_get_cluster_bootstrap(config, self, &imds_data.vpc_ipv4_cidr_blocks).await?;
    debug!("Cluster bootstrap details: {:#?}", cluster_boostrap);

    Ok(cluster_boostrap)
  }

  /// Create kubeconfig file for kubelet
  ///
  /// If cluster is local cluster on Outpost, use aws-iam-authenticator as bootstrap auth for kubelet
  /// TLS bootstrapping which downloads client X.509 certificate and generates kubelet kubeconfig file
  /// which uses the client cert. This allows the worker node can be authenticated through
  /// X.509 certificate which works for both connected and disconnected states.
  fn create_kubelet_kubeconfig(&self, cluster: &eks::ClusterBootstrap) -> Result<()> {
    let name = match self.is_local_cluster {
      true => &cluster.name,
      false => self
        .cluster_id
        .as_ref()
        .expect("Cluster ID is required when your local Amazon EKS cluster is on an Amazon Web Services Outpost"),
    };

    let path = match self.is_local_cluster {
      true => "/var/lib/kubelet/bootstrap-kubeconfig",
      false => "/var/lib/kubelet/kubeconfig",
    };

    let kubeconfig = kubelet::KubeConfig::new(&cluster.endpoint, name, &cluster.b64_ca)?;
    kubeconfig.write(Path::new(path))?;

    Ok(())
  }

  fn create_ca_cert(&self, base64_ca: &str) -> Result<()> {
    let decoded = general_purpose::STANDARD_NO_PAD.decode(base64_ca.clone())?;

    fs::create_dir_all("/etc/kubernetes/pki")?;
    Ok(fs::write("/etc/kubernetes/pki/ca.crt", decoded)?)
  }

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

  async fn get_max_pods(&self, instance_type: &str) -> Result<i32> {
    match ec2::INSTANCES.get(instance_type) {
      Some(instance) => Ok(instance.eni_maximum_pods),
      None => {
        info!("Instance type {instance_type} not found in static instance data. Attempting to derive max pods");

        let max_pods = crate::cli::MaxPods {
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

  pub async fn join_node_to_cluster(&self) -> Result<()> {
    let instance_metadata = imds::get_imds_data().await?;
    let cluster_details = self.get_cluster_bootstrap().await?;

    // TODO - get kubelet version

    self.create_ca_cert(&cluster_details.b64_ca)?;
    self.create_kubelet_kubeconfig(&cluster_details)?;

    if self.is_local_cluster {
      self.update_etc_hosts(&cluster_details.endpoint, PathBuf::from("/etc/hosts"))?;
    }

    let _max_pods = self.get_max_pods(&instance_metadata.instance_type).await?;

    Ok(())
  }
}
