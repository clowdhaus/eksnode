use std::{
  collections::BTreeMap,
  fs::File,
  io::{BufReader, BufWriter},
  path::{Path, PathBuf},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KubeConfig {
  /// Kind is a string value representing the REST resource this object represents.
  kind: String,

  /// APIVersion defines the versioned schema of this representation of an object.
  api_version: String,

  /// Clusters defined in the kubeconfig
  clusters: Vec<NamedCluster>,

  /// Contexts binds names to cluster/user/namespaces tuples
  contexts: Vec<NamedContext>,

  /// CurrentContext is the name of the default context
  #[serde(rename = "current-context")]
  current_context: String,

  /// Users defined in the kubeconfig
  users: Vec<NamedAuthInfo>,
}

impl KubeConfig {
  pub fn new(server: &str, cluster_name: &str, region: &str) -> Result<Self> {
    Ok(KubeConfig {
      kind: "Config".to_owned(),
      api_version: "v1".to_owned(),
      clusters: vec![NamedCluster {
        cluster: Cluster {
          server: server.into(),
          certificate_authority: Some(PathBuf::from("/etc/kubernetes/pki/ca.crt")),
          certificate_authority_data: None,
          insecure_skip_tls_verify: None,
          proxy_url: None,
          tls_server_name: None,
          disable_compression: None,
          extensions: None,
        },
        name: "kubernetes".to_owned(),
      }],
      contexts: vec![NamedContext {
        context: Context {
          cluster: "kubernetes".to_owned(),
          namespace: None,
          user: "kubelet".to_owned(),
          extensions: None,
        },
        name: "kubelet".to_owned(),
      }],
      current_context: "kubelet".to_owned(),
      users: vec![NamedAuthInfo {
        user: AuthInfo {
          client_certificate: None,
          client_certificate_data: None,
          client_key: None,
          client_key_data: None,
          token: None,
          token_file: None,
          _as: None,
          as_uid: None,
          as_groups: None,
          as_user_extra: None,
          username: None,
          password: None,
          auth_provider: None,
          exec: Some(ExecConfig {
            api_version: Some("client.authentication.k8s.io/v1beta1".to_owned()),
            command: "/usr/bin/aws-iam-authenticator".to_owned(),
            args: Some(vec![
              "token".to_owned(),
              "-i".to_owned(),
              cluster_name.into(),
              "--region".to_owned(),
              region.into(),
            ]),
            env: None,
            install_hint: None,
            provide_cluster_info: None,
            interactive_mode: None,
          }),
          extensions: None,
        },
        name: "kubelet".to_owned(),
      }],
    })
  }

  pub fn read<P: AsRef<Path>>(path: P) -> Result<Self> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let conf: KubeConfig = serde_yaml::from_reader(reader)?;

    Ok(conf)
  }

  pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<()> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    serde_yaml::to_writer(writer, self).map_err(anyhow::Error::from)
  }
}

/// NamedCluster relates nicknames to cluster information
#[derive(Debug, Serialize, Deserialize)]
struct NamedCluster {
  /// Cluster holds the cluster information
  cluster: Cluster,

  /// Name is the nickname for this Cluster
  name: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Cluster {
  /// Server is the address of the kubernetes cluster (https://hostname:port)
  server: String,

  /// TLSServerName is used to check server certificate. If TLSServerName is empty, the
  /// hostname used to contact the server is used
  #[serde(skip_serializing_if = "Option::is_none")]
  tls_server_name: Option<String>,

  /// InsecureSkipTLSVerify skips the validity check for the server's certificate.
  /// This will make your HTTPS connections insecure
  #[serde(skip_serializing_if = "Option::is_none")]
  insecure_skip_tls_verify: Option<bool>,

  /// CertificateAuthority is the path to a cert file for the certificate authority
  #[serde(skip_serializing_if = "Option::is_none")]
  certificate_authority: Option<PathBuf>,

  /// CertificateAuthorityData contains PEM-encoded certificate authority certificates.
  /// Overrides CertificateAuthority
  #[serde(skip_serializing_if = "Option::is_none")]
  certificate_authority_data: Option<Vec<u8>>,

  /// ProxyURL is the URL to the proxy to be used for all requests made by this client.
  ///
  /// URLs with "http", "https", and "socks5" schemes are supported.
  /// If this configuration is not provided or the empty string, the client attempts to construct a
  /// proxy configuration from http_proxy and https_proxy environment variables.
  /// If these environment variables are not set, the client does not attempt to proxy requests.
  /// socks5 proxying does not currently support spdy streaming endpoints (exec, attach, port forward).
  #[serde(skip_serializing_if = "Option::is_none")]
  proxy_url: Option<String>,

  /// DisableCompression allows client to opt-out of response compression for all requests to the
  /// server. This is useful to speed up requests (specifically lists) when client-server
  /// network bandwidth is ample, by saving time on compression (server-side) and decompression
  /// (client-side): https://github.com/kubernetes/kubernetes/issues/112296.
  #[serde(skip_serializing_if = "Option::is_none")]
  disable_compression: Option<bool>,

  /// Extensions holds additional information.
  /// This is useful for extenders so that reads and writes don't clobber unknown fields
  #[serde(skip_serializing_if = "Option::is_none")]
  extensions: Option<Vec<NamedExtension>>,
}

/// NamedExtension relates nicknames to extension information
#[derive(Debug, Serialize, Deserialize)]
struct NamedExtension {
  /// Name is the nickname for this Extension
  name: String,

  /// Extension holds the extension information
  extension: Context,
}

/// NamedContext relates nicknames to context information
#[derive(Debug, Serialize, Deserialize)]
struct NamedContext {
  /// Name is the nickname for this Context
  name: String,

  /// Context holds the context information
  context: Context,
}

/// Context is a tuple of references to a cluster (how do I communicate
/// with a kubernetes cluster), a user (how do I identify myself),
/// and a namespace (what subset of resources do I want to work with)
#[derive(Debug, Serialize, Deserialize)]
struct Context {
  /// Cluster is the name of the cluster for this context
  cluster: String,

  /// User is the name of the authInfo for this context
  user: String,

  /// Namespace is the default namespace to use on unspecified requests
  #[serde(skip_serializing_if = "Option::is_none")]
  namespace: Option<String>,

  /// Extensions holds additional information.
  /// This is useful for extenders so that reads and writes don't clobber unknown fields
  #[serde(skip_serializing_if = "Option::is_none")]
  extensions: Option<Vec<NamedExtension>>,
}

/// NamedAuthInfo relates nicknames to auth information
#[derive(Debug, Serialize, Deserialize)]
struct NamedAuthInfo {
  /// Name is the nickname for this AuthInfo
  name: String,

  /// AuthInfo holds the auth information
  user: AuthInfo,
}

/// AuthInfo contains information that describes identity information
///
/// This is use to tell the kubernetes cluster who you are
#[derive(Debug, Serialize, Deserialize)]
struct AuthInfo {
  /// ClientCertificate is the path to a client cert file for TLS
  #[serde(skip_serializing_if = "Option::is_none")]
  client_certificate: Option<PathBuf>,

  /// ClientCertificateData contains PEM-encoded data from a client cert file for TLS. Overrides ClientCertificate
  #[serde(skip_serializing_if = "Option::is_none")]
  client_certificate_data: Option<Vec<u8>>,

  /// ClientKey is the path to a client key file for TLS
  #[serde(skip_serializing_if = "Option::is_none")]
  client_key: Option<PathBuf>,

  /// ClientKeyData contains PEM-encoded data from a client key file for TLS. Overrides ClientKey
  #[serde(skip_serializing_if = "Option::is_none")]
  client_key_data: Option<Vec<u8>>,

  /// Token is the bearer token for authentication to the kubernetes cluster
  #[serde(skip_serializing_if = "Option::is_none")]
  token: Option<String>,

  /// TokenFile is a pointer to a file that contains a bearer token (as described above). If both Token and TokenFile
  /// are present, Token takes precedence
  #[serde(skip_serializing_if = "Option::is_none")]
  token_file: Option<PathBuf>,

  /// Impersonate is the username to impersonate. The name matches the flag
  #[serde(skip_serializing_if = "Option::is_none", rename = "as")]
  _as: Option<String>,

  /// ImpersonateUID is the uid to impersonate
  #[serde(skip_serializing_if = "Option::is_none")]
  as_uid: Option<String>,

  /// ImpersonateGroups is the groups to impersonate
  #[serde(skip_serializing_if = "Option::is_none")]
  as_groups: Option<Vec<String>>,

  /// ImpersonateUserExtra contains additional information for impersonated user.
  #[serde(skip_serializing_if = "Option::is_none")]
  as_user_extra: Option<BTreeMap<String, Vec<String>>>,

  /// Username is the username for basic authentication to the kubernetes cluster
  #[serde(skip_serializing_if = "Option::is_none")]
  username: Option<String>,

  /// Password is the password for basic authentication to the kubernetes cluster
  #[serde(skip_serializing_if = "Option::is_none")]
  password: Option<String>,

  /// AuthProvider specifies a custom authentication plugin for the kubernetes cluster
  #[serde(skip_serializing_if = "Option::is_none")]
  auth_provider: Option<AuthProviderConfig>,

  /// Exec specifies a custom exec-based authentication plugin for the kubernetes cluster
  #[serde(skip_serializing_if = "Option::is_none")]
  exec: Option<ExecConfig>,

  /// Extensions holds additional information.
  /// This is useful for extenders so that reads and writes don't clobber unknown fields
  #[serde(skip_serializing_if = "Option::is_none")]
  extensions: Option<Vec<NamedExtension>>,
}

/// AuthProviderConfig holds the configuration for a specified auth provider
#[derive(Debug, Serialize, Deserialize)]
struct AuthProviderConfig {
  /// Name is the name of the auth provider
  name: String,

  /// Config holds the auth provider configuration information.
  /// The contents of this field depends on the provider being used
  config: BTreeMap<String, String>,
}

/// ExecConfig specifies a command to provide client credentials.
/// The command is exec'd and outputs structured stdout holding credentials.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExecConfig {
  /// Preferred input version of the ExecInfo.
  /// The returned ExecCredentials MUST use the same encoding version as the input
  #[serde(skip_serializing_if = "Option::is_none")]
  api_version: Option<String>,

  /// Command to execute
  command: String,

  /// Arguments to pass to the command when executing it
  #[serde(skip_serializing_if = "Option::is_none")]
  pub args: Option<Vec<String>>,

  /// Env defines additional environment variables to expose to the process.
  /// These are unioned with the host's environment, as well as variables
  /// client-go uses to pass argument to the plugin.
  #[serde(skip_serializing_if = "Option::is_none")]
  env: Option<Vec<EnvVar>>,

  /// This text is shown to the user when the executable doesn't seem to be present.
  /// For example, brew install foo-cli might be a good InstallHint for foo-cli on Mac OS systems.
  // TODO - this deviates from the API https://kubernetes.io/docs/reference/config-api/kubeconfig.v1/#ExecConfig
  #[serde(skip_serializing_if = "Option::is_none")]
  install_hint: Option<String>,

  /// ProvideClusterInfo determines whether or not to provide cluster information,
  /// which could potentially contain very large CA data, to this exec plugin as a
  /// part of the KUBERNETES_EXEC_INFO environment variable. By default, it is set to false.
  /// Package k8s.io/client-go/tools/auth/exec provides helper methods for reading
  /// this environment variable.
  // TODO - this deviates from the API https://kubernetes.io/docs/reference/config-api/kubeconfig.v1/#ExecConfig
  #[serde(skip_serializing_if = "Option::is_none")]
  provide_cluster_info: Option<bool>,

  /// InteractiveMode determines this plugin's relationship with standard input.
  ///
  /// If APIVersion is client.authentication.k8s.io/v1alpha1 or client.authentication.k8s.io/v1beta1,
  /// then this field is optional and defaults to "IfAvailable" when unset.
  /// Otherwise, this field is required.
  #[serde(skip_serializing_if = "Option::is_none", default)]
  interactive_mode: Option<ExecInteractiveMode>,
}

/// ExecEnvVar is used for setting environment variables when executing an exec-based credential plugin
#[derive(Debug, Serialize, Deserialize)]
struct EnvVar {
  /// Name of the environment variable
  name: String,

  /// Value of the environment variable
  value: String,
}

/// ExecInteractiveMode is a string that describes an exec plugin's relationship with standard input.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ExecInteractiveMode {
  /// This exec plugin never uses standard input
  Never,

  /// This exec plugin wants to use standard input if it is available
  #[default]
  IfAvailable,

  /// This exec plugin requires standard input to function
  Always,
}

#[cfg(test)]
mod tests {
  use std::io::{Read, Seek, SeekFrom};

  use tempfile::NamedTempFile;

  use super::*;

  #[test]
  fn it_serializes_kubeconfig() {
    let config = r#"
      apiVersion: v1
      kind: Config
      clusters:
      - cluster:
          certificate-authority: /etc/kubernetes/pki/ca.crt
          server: MASTER_ENDPOINT
        name: kubernetes
      contexts:
      - context:
          cluster: kubernetes
          user: kubelet
        name: kubelet
      current-context: kubelet
      users:
      - name: kubelet
        user:
          exec:
            apiVersion: client.authentication.k8s.io/v1beta1
            command: /usr/bin/aws-iam-authenticator
            args:
              - "token"
              - "-i"
              - "CLUSTER_NAME"
              - --region
              - "AWS_REGION"
    "#;

    let deserialized: KubeConfig = serde_yaml::from_str(config).unwrap();
    insta::assert_debug_snapshot!(deserialized);

    let serialized = serde_yaml::to_string(&deserialized).unwrap();
    insta::assert_debug_snapshot!(serialized);
  }

  #[test]
  fn it_creates_kubeconfig() {
    let new = KubeConfig::new("http://localhost:8080", "example", "us-west-2").unwrap();
    insta::assert_debug_snapshot!(new);

    let mut file = NamedTempFile::new().unwrap();
    new.write(&file).unwrap();

    // Seek to start
    file.seek(SeekFrom::Start(0)).unwrap();

    // Read
    let mut buf = String::new();
    file.read_to_string(&mut buf).unwrap();
    insta::assert_debug_snapshot!(buf);
  }
}
