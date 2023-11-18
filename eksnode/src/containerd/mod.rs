use std::{collections::BTreeMap, path::Path};

use anyhow::Result;
use clap::ValueEnum;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};

use crate::utils;

pub const SANDBOX_IMAGE_SERVICE_PATH: &str = "/etc/systemd/system/sandbox-image.service";
pub const SANDBOX_IMAGE_TAG: &str = "3.8";

/// Embeds the contents of the `templates/` directory into the binary
///
/// This struct contains both the templates used for rendering the playbook
/// as well as the static data used for populating the playbook templates
/// embedded into the binary for distribution
#[derive(RustEmbed)]
#[folder = "src/containerd/templates/"]
pub struct Templates;

#[derive(Copy, Clone, Debug, ValueEnum, Serialize, Deserialize)]
pub enum DefaultRuntime {
  Containerd,
  Neuron,
  Nvidia,
}

impl Default for DefaultRuntime {
  fn default() -> Self {
    Self::Containerd
  }
}

pub fn create_sandbox_image_service<P: AsRef<Path>>(path: P, pause_image: &str, chown: bool) -> Result<()> {
  let tmpl = Templates::get("sandbox-image.service").unwrap();
  let tmpl = std::str::from_utf8(tmpl.data.as_ref())?;

  let contents = tmpl.replace(
    "{{EXEC_START}}",
    &format!("eksnode pull --image {pause_image} --namespace k8s.io"),
  );
  utils::write_file(contents.as_bytes(), path, Some(0o644), chown)
}

/// Config provides containerd configuration data for the server
///
/// https://github.com/containerd/containerd/blob/main/services/server/config/config.go
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ContainerdConfiguration {
  /// Version of the config file
  version: i32,

  /// Root is the path to a directory where containerd will store persistent data
  #[serde(skip_serializing_if = "Option::is_none")]
  root: Option<String>,

  /// State is the path to a directory where containerd will store transient data
  #[serde(skip_serializing_if = "Option::is_none")]
  state: Option<String>,

  /// TempDir is the path to a directory where to place containerd temporary files
  #[serde(rename = "temp", skip_serializing_if = "Option::is_none")]
  temp_dir: Option<String>,

  /// PluginDir is the directory for dynamic plugins to be stored
  #[serde(skip_serializing_if = "Option::is_none")]
  plugin_dir: Option<String>,

  /// GRPC configuration settings
  #[serde(skip_serializing_if = "Option::is_none")]
  grpc: Option<GrpcConfig>,

  /// TTRPC configuration settings
  #[serde(skip_serializing_if = "Option::is_none")]
  ttrpc: Option<TtrpcConfig>,

  /// Debug and profiling settings
  #[serde(skip_serializing_if = "Option::is_none")]
  debug: Option<DebugConfig>,

  /// Metrics and monitoring settings
  #[serde(skip_serializing_if = "Option::is_none")]
  metrics: Option<MetricsConfig>,

  /// DisabledPlugins are IDs of plugins to disable. Disabled plugins won't be
  /// initialized and started.
  #[serde(skip_serializing_if = "Option::is_none")]
  disabled_plugins: Option<Vec<String>>,

  /// RequiredPlugins are IDs of required plugins. Containerd exits if any
  /// required plugin doesn't exist or fails to be initialized or started.
  #[serde(skip_serializing_if = "Option::is_none")]
  required_plugins: Option<Vec<String>>,

  /// Plugins provides plugin specific configuration for the initialization of a plugin
  #[serde(flatten, skip_serializing_if = "Option::is_none")]
  plugins: Option<BTreeMap<String, serde_json::Value>>,

  /// OOMScore adjust the containerd's oom score
  #[serde(skip_serializing_if = "Option::is_none")]
  oom_score: Option<i32>,

  /// Cgroup specifies cgroup information for the containerd daemon process
  #[serde(skip_serializing_if = "Option::is_none")]
  cgroup: Option<CgroupConfig>,

  /// ProxyPlugins configures plugins which are communicated to over GRPC
  #[serde(skip_serializing_if = "Option::is_none")]
  proxy_plugins: Option<BTreeMap<String, ProxyPlugin>>,

  /// Timeouts specified as a duration
  #[serde(skip_serializing_if = "Option::is_none")]
  timeouts: Option<BTreeMap<String, String>>,

  /// Imports are additional file path list to config files that can overwrite main config file fields
  #[serde(skip_serializing_if = "Option::is_none")]
  imports: Option<Vec<String>>,

  /// StreamProcessors configuration
  #[serde(skip_serializing_if = "Option::is_none")]
  stream_processors: Option<BTreeMap<String, StreamProcessor>>,
}

impl ContainerdConfiguration {
  pub fn new(default_runtime: &DefaultRuntime, sandbox_image: &str) -> Result<Self> {
    let tmpl = Templates::get("containerd-config.toml").unwrap();
    let tmpl = std::str::from_utf8(tmpl.data.as_ref())?;

    let runtime = match default_runtime {
      DefaultRuntime::Containerd => {
        r#"
[plugins."io.containerd.grpc.v1.cri".containerd]
  default_runtime_name = "runc"
  discard_unpacked_layers = true

  [plugins."io.containerd.grpc.v1.cri".containerd.runtimes.runc]
    runtime_type = "io.containerd.runc.v2"

    [plugins."io.containerd.grpc.v1.cri".containerd.runtimes.runc.options]
      SystemdCgroup = true
"#
      }
      DefaultRuntime::Neuron => {
        r#"
[plugins."io.containerd.grpc.v1.cri".containerd]
  default_runtime_name = "neuron"
  discard_unpacked_layers = true

[plugins."io.containerd.grpc.v1.cri".containerd.runtimes.neuron]
   [plugins."io.containerd.grpc.v1.cri".containerd.runtimes.neuron.options]
      BinaryName = "/opt/aws/neuron/bin/oci_neuron_hook_wrapper.sh"

  [plugins."io.containerd.grpc.v1.cri".containerd.runtimes.neuron]
    runtime_type = "io.containerd.runc.v2"

    [plugins."io.containerd.grpc.v1.cri".containerd.runtimes.neuron.options]
      SystemdCgroup = true
      BinaryName = "/opt/aws/neuron/bin/oci_neuron_hook_wrapper.sh"
"#
      }
      DefaultRuntime::Nvidia => {
        r#"
[plugins."io.containerd.grpc.v1.cri".containerd]
  default_runtime_name = "nvidia"
  discard_unpacked_layers = true

  [plugins."io.containerd.grpc.v1.cri".containerd.runtimes.nvidia]
    runtime_type = "io.containerd.runc.v2"

    [plugins."io.containerd.grpc.v1.cri".containerd.runtimes.nvidia.options]
      SystemdCgroup = true
      BinaryName = "/usr/bin/nvidia-container-runtime"
"#
      }
    };

    let contents = tmpl
      .replace("{{SANDBOX_IMAGE}}", sandbox_image)
      .replace("{{RUNTIME}}", runtime.trim());
    let config: ContainerdConfiguration = toml::from_str(&contents)?;

    Ok(config)
  }

  pub fn read<P: AsRef<Path>>(path: P) -> Result<Self> {
    let file = std::fs::read_to_string(path)?;
    let config: ContainerdConfiguration = toml::from_str(&file)?;

    Ok(config)
  }

  pub fn write<P: AsRef<Path>>(&self, path: P, chown: bool) -> Result<()> {
    let conf = toml::to_string_pretty(self)?;
    utils::write_file(conf.as_bytes(), path, Some(0o644), chown)
  }
}

/// GRPCConfig provides GRPC configuration for the socket
#[derive(Debug, Default, Serialize, Deserialize)]
struct GrpcConfig {
  #[serde(skip_serializing_if = "Option::is_none")]
  address: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  tcp_address: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  tcp_tls_ca: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  tcp_tls_cert: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  tcp_tls_key: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  uid: Option<i32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  gid: Option<i32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  max_recv_message_size: Option<i32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  max_send_message_size: Option<i32>,
}

/// TTRPCConfig provides TTRPC configuration for the socket
#[derive(Debug, Default, Serialize, Deserialize)]
struct TtrpcConfig {
  address: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  uid: Option<i32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  gid: Option<i32>,
}

/// Debug provides debug configuration
#[derive(Debug, Default, Serialize, Deserialize)]
struct DebugConfig {
  address: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  uid: Option<i32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  gid: Option<i32>,
  level: String,
  /// Format represents the logging format. Supported values are 'text' and 'json'.
  format: DebugFormat,
}

#[derive(Debug, Default, Serialize, Deserialize)]
enum DebugFormat {
  Text,
  #[default]
  Json,
}

/// MetricsConfig provides metrics configuration
#[derive(Debug, Default, Serialize, Deserialize)]
struct MetricsConfig {
  address: String,
  grpc_histogram: bool,
}

// CgroupConfig provides cgroup configuration
#[derive(Debug, Default, Serialize, Deserialize)]
struct CgroupConfig {
  path: String,
}

// ProxyPlugin provides a proxy plugin configuration
#[derive(Debug, Default, Serialize, Deserialize)]
struct ProxyPlugin {
  #[serde(rename = "type")]
  type_: String,
  address: String,
  platform: String,
}
/// StreamProcessor provides configuration for diff content processors
#[derive(Debug, Default, Serialize, Deserialize)]
struct StreamProcessor {
  /// Accepts specific media-types
  accepts: Vec<String>,
  /// Returns the media-type
  returns: String,
  /// Path or name of the binary
  path: String,
  /// Args to the binary
  #[serde(skip_serializing_if = "Option::is_none")]
  args: Option<Vec<String>>,
  /// Environment variables for the binary
  #[serde(skip_serializing_if = "Option::is_none")]
  env: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
  use std::io::{Read, Seek, SeekFrom};

  use tempfile::NamedTempFile;

  use super::*;

  #[test]
  fn it_serializes_containerd_config() {
    let config = r#"
    version = 2
    root = "/var/lib/containerd"
    state = "/run/containerd"
    disabled_plugins = [
        "io.containerd.internal.v1.opt",
        "io.containerd.snapshotter.v1.aufs",
        "io.containerd.snapshotter.v1.devmapper",
        "io.containerd.snapshotter.v1.native",
        "io.containerd.snapshotter.v1.zfs",
    ]

    [grpc]
    address = "/run/containerd/containerd.sock"

    [plugins."io.containerd.grpc.v1.cri"]
    sandbox_image = "602401143452.dkr.ecr.us-east-1.amazonaws.com/eks/pause:3.8"

    [plugins."io.containerd.grpc.v1.cri".cni]
    bin_dir = "/opt/cni/bin"
    conf_dir = "/etc/cni/net.d"

    [plugins."io.containerd.grpc.v1.cri".containerd]
    default_runtime_name = "runc"

    [plugins."io.containerd.grpc.v1.cri".containerd.runtimes.runc]
    runtime_type = "io.containerd.runc.v2"

    [plugins."io.containerd.grpc.v1.cri".containerd.runtimes.runc.options]
    SystemdCgroup = true

    [plugins."io.containerd.grpc.v1.cri".registry]
    config_path = "/etc/containerd/certs.d:/etc/docker/certs.d"
    "#;

    let deserialized: ContainerdConfiguration = toml::from_str(config).unwrap();
    insta::assert_debug_snapshot!(deserialized);

    let serialized = toml::to_string_pretty(&deserialized).unwrap();
    insta::assert_debug_snapshot!(serialized);
  }

  #[test]
  fn it_creates_containerd_config() {
    let sandbox_img = "602401143452.dkr.ecr.us-east-1.amazonaws.com/eks/pause:3.8";
    let config = ContainerdConfiguration::new(&DefaultRuntime::Containerd, sandbox_img).unwrap();
    insta::assert_debug_snapshot!(config);

    let mut file = NamedTempFile::new().unwrap();
    config.write(&file, false).unwrap();

    // Seek to start
    file.seek(SeekFrom::Start(0)).unwrap();

    // Read
    let mut buf = String::new();
    file.read_to_string(&mut buf).unwrap();
    insta::assert_debug_snapshot!(buf);
  }

  #[test]
  fn it_creates_sandbox_image_service() {
    let sandbox_img = "602401143452.dkr.ecr.us-east-1.amazonaws.com/eks/pause:3.9";

    // Write to file
    let mut file = NamedTempFile::new().unwrap();
    create_sandbox_image_service(&file, sandbox_img, false).unwrap();
    file.seek(SeekFrom::Start(0)).unwrap();

    // Read back contents written to file
    let mut buf = String::new();
    file.read_to_string(&mut buf).unwrap();
    insta::assert_debug_snapshot!(buf);
  }
}
