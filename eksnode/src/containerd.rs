use std::{collections::BTreeMap, path::Path};

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Config provides containerd configuration data for the server
///
/// https://github.com/containerd/containerd/blob/main/services/server/config/config.go
#[derive(Serialize, Deserialize, Debug)]
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
  #[serde(skip_serializing_if = "Vec::is_empty", default)]
  disabled_plugins: Vec<String>,

  /// RequiredPlugins are IDs of required plugins. Containerd exits if any
  /// required plugin doesn't exist or fails to be initialized or started.
  #[serde(skip_serializing_if = "Vec::is_empty", default)]
  required_plugins: Vec<String>,

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
  #[serde(skip_serializing_if = "Vec::is_empty", default)]
  imports: Vec<String>,

  /// StreamProcessors configuration
  #[serde(skip_serializing_if = "Option::is_none")]
  stream_processors: Option<BTreeMap<String, StreamProcessor>>,
}

impl ContainerdConfiguration {
  pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
    let file = std::fs::read_to_string(path)?;
    let conf: ContainerdConfiguration = toml::from_str(&file)?;

    Ok(conf)
  }

  pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<()> {
    let conf = toml::to_string_pretty(self)?;
    std::fs::write(path, conf)?;

    Ok(())
  }
}

/// GRPCConfig provides GRPC configuration for the socket
#[derive(Serialize, Deserialize, Debug)]
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
#[derive(Serialize, Deserialize, Debug)]
struct TtrpcConfig {
  address: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  uid: Option<i32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  gid: Option<i32>,
}

/// Debug provides debug configuration
#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug, Default)]
enum DebugFormat {
  Text,
  #[default]
  Json,
}

/// MetricsConfig provides metrics configuration
#[derive(Serialize, Deserialize, Debug)]
struct MetricsConfig {
  address: String,
  grpc_histogram: bool,
}

// CgroupConfig provides cgroup configuration
#[derive(Serialize, Deserialize, Debug)]
struct CgroupConfig {
  path: String,
}

// ProxyPlugin provides a proxy plugin configuration
#[derive(Serialize, Deserialize, Debug)]
struct ProxyPlugin {
  #[serde(rename = "type")]
  type_: String,
  address: String,
  platform: String,
}
/// StreamProcessor provides configuration for diff content processors
#[derive(Serialize, Deserialize, Debug)]
struct StreamProcessor {
  /// Accepts specific media-types
  accepts: Vec<String>,
  /// Returns the media-type
  returns: String,
  /// Path or name of the binary
  path: String,
  /// Args to the binary
  #[serde(skip_serializing_if = "Vec::is_empty", default)]
  args: Vec<String>,
  /// Environment variables for the binary
  #[serde(skip_serializing_if = "Vec::is_empty", default)]
  env: Vec<String>,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn it_serializes_containerd_config() {
    let config = r#"
    version = 2
    root = "/var/lib/containerd"
    state = "/run/containerd"

    [grpc]
    address = "/run/containerd/containerd.sock"

    [plugins."io.containerd.grpc.v1.cri".containerd]
    default_runtime_name = "runc"

    [plugins."io.containerd.grpc.v1.cri"]
    sandbox_image = "SANDBOX_IMAGE"

    [plugins."io.containerd.grpc.v1.cri".registry]
    config_path = "/etc/containerd/certs.d:/etc/docker/certs.d"

    [plugins."io.containerd.grpc.v1.cri".containerd.runtimes.runc]
    runtime_type = "io.containerd.runc.v2"

    [plugins."io.containerd.grpc.v1.cri".containerd.runtimes.runc.options]
    SystemdCgroup = true

    [plugins."io.containerd.grpc.v1.cri".cni]
    bin_dir = "/opt/cni/bin"
    conf_dir = "/etc/cni/net.d"
    "#;

    let deserialized: ContainerdConfiguration = toml::from_str(config).unwrap();
    insta::assert_debug_snapshot!(deserialized);

    let serialized = toml::to_string_pretty(&deserialized).unwrap();
    insta::assert_debug_snapshot!(serialized);
  }
}