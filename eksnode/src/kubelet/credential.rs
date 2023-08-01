use std::{
  fs::{File, OpenOptions},
  io::{BufReader, BufWriter},
  os::unix::fs::{chown, OpenOptionsExt},
  path::Path,
};

use anyhow::Result;
use semver::Version;
use serde::{Deserialize, Serialize};

/// CredentialProviderConfig is the configuration containing information about each exec credential provider. Kubelet
/// reads this configuration from disk and enables each provider as specified by the CredentialProvider type.
///
/// https://kubernetes.io/docs/reference/config-api/kubelet-config.v1/
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialProviderConfig {
  /// Kind is a string value representing the REST resource this object represents.
  kind: String,

  /// APIVersion defines the versioned schema of this representation of an object.
  api_version: String,

  /// providers is a list of credential provider plugins that will be enabled by the kubelet. Multiple providers may
  /// match against a single image, in which case credentials from all providers will be returned to the kubelet. If
  /// multiple providers are called for a single image, the results are combined. If providers return overlapping auth
  /// keys, the value from the provider earlier in this list is used.
  providers: Vec<CredentialProvider>,
}

/// CredentialProvider represents an exec plugin to be invoked by the kubelet. The plugin is only invoked when an image
/// being pulled matches the images handled by the plugin (see matchImages).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialProvider {
  /// name is the required name of the credential provider. It must match  the name of the provider executable as seen
  /// by the kubelet. The executable must be in the kubelet's bin directory (set by the
  /// --image-credential-provider-bin-dir flag).
  name: String,

  /// matchImages is a required list of strings used to match against images in order to determine if this provider
  /// should be invoked. If one of the strings matches the requested image from the kubelet, the plugin will be invoked
  /// and given a chance to provide credentials. Images are expected to contain the registry domain and URL path.
  match_images: Vec<String>,

  /// defaultCacheDuration is the default duration the plugin will cache credentials in-memory if a cache duration is
  /// not provided in the plugin response.
  default_cache_duration: String,

  /// Required input version of the exec CredentialProviderRequest. The returned CredentialProviderResponse MUST use
  /// the same encoding version as the input
  api_version: String,

  /// Arguments to pass to the command when executing it.
  #[serde(skip_serializing_if = "Option::is_none")]
  args: Option<Vec<String>>,

  /// Env defines additional environment variables to expose to the process. These are unioned with the host's
  /// environment, as well as variables client-go uses to pass argument to the plugin.
  #[serde(skip_serializing_if = "Option::is_none")]
  env: Option<Vec<ExecEnvVar>>,
}

/// ExecEnvVar is used for setting environment variables when executing an exec-based credential plugin
#[derive(Clone, Debug, Serialize, Deserialize)]
struct ExecEnvVar {
  /// Name of the environment variable
  name: String,

  /// Value of the environment variable
  value: String,
}

impl CredentialProviderConfig {
  pub fn new(kubelet_version: &Version) -> Result<Self> {
    // ecr-credential-provider only implements credentialprovider.kubelet.k8s.io/v1alpha1 prior to 1.27.1: https://github.com/kubernetes/cloud-provider-aws/pull/597
    let api_version = match kubelet_version.lt(&Version::parse("1.27.0")?) {
      true => "credentialprovider.kubelet.k8s.io/v1alpha1".to_string(),
      false => "credentialprovider.kubelet.k8s.io/v1".to_string(),
    };

    Ok(CredentialProviderConfig {
      api_version,
      kind: "CredentialProviderConfig".to_owned(),
      providers: vec![CredentialProvider {
        name: "ecr-credential-provider".to_owned(),
        match_images: vec![
          "*.dkr.ecr.*.amazonaws.com".to_owned(),
          "*.dkr.ecr.*.amazonaws.com.cn".to_owned(),
          "*.dkr.ecr-fips.*.amazonaws.com".to_owned(),
          "*.dkr.ecr.us-iso-east-1.c2s.ic.gov".to_owned(),
          "*.dkr.ecr.us-isob-east-1.sc2s.sgov.gov".to_owned(),
        ],
        default_cache_duration: "12h".to_owned(),
        api_version: "credentialprovider.kubelet.k8s.io/v1".to_owned(),
        args: None,
        env: None,
      }],
    })
  }

  pub fn read<P: AsRef<Path>>(path: P) -> Result<Self> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let conf: CredentialProviderConfig = serde_json::from_reader(reader)?;

    Ok(conf)
  }

  pub fn write<P: AsRef<Path>>(&self, path: P, id: Option<u32>) -> Result<()> {
    let file = OpenOptions::new().write(true).create(true).mode(0o644).open(&path)?;
    let writer = BufWriter::new(file);

    serde_json::to_writer_pretty(writer, self).map_err(anyhow::Error::from)?;
    Ok(chown(path, id, id)?)
  }
}

#[cfg(test)]
mod tests {
  use std::io::{Read, Seek, SeekFrom};

  use tempfile::NamedTempFile;

  use super::*;

  #[test]
  fn it_serializes_credential_provider() {
    let config = r#"{
      "kind": "CredentialProviderConfig",
      "apiVersion": "kubelet.config.k8s.io/v1",
      "providers": [
        {
          "name": "ecr-credential-provider",
          "matchImages": [
            "*.dkr.ecr.*.amazonaws.com",
            "*.dkr.ecr.*.amazonaws.com.cn",
            "*.dkr.ecr-fips.*.amazonaws.com",
            "*.dkr.ecr.us-iso-east-1.c2s.ic.gov",
            "*.dkr.ecr.us-isob-east-1.sc2s.sgov.gov"
          ],
          "defaultCacheDuration": "12h",
          "apiVersion": "credentialprovider.kubelet.k8s.io/v1"
        }
      ]
    }"#;

    let deserialized: CredentialProviderConfig = serde_json::from_str(config).unwrap();
    insta::assert_debug_snapshot!(deserialized);

    let serialized = serde_json::to_string_pretty(&deserialized).unwrap();
    insta::assert_debug_snapshot!(serialized);
  }

  #[test]
  fn it_creates_v1alpha1() {
    let kubelet_version = Version::parse("1.26.0").unwrap();
    let new = CredentialProviderConfig::new(&kubelet_version).unwrap();
    insta::assert_debug_snapshot!(new);
    assert_eq!(new.api_version, "credentialprovider.kubelet.k8s.io/v1alpha1".to_owned());

    let mut file = NamedTempFile::new().unwrap();
    new.write(&file, None).unwrap();

    // Seek to start
    file.seek(SeekFrom::Start(0)).unwrap();

    // Read
    let mut buf = String::new();
    file.read_to_string(&mut buf).unwrap();
    insta::assert_debug_snapshot!(buf);
  }

  #[test]
  fn it_creates_v1() {
    let kubelet_version = Version::parse("1.27.0").unwrap();
    let new = CredentialProviderConfig::new(&kubelet_version).unwrap();
    insta::assert_debug_snapshot!(new);
    assert_eq!(new.api_version, "credentialprovider.kubelet.k8s.io/v1".to_owned());

    // Write to file
    let mut file = NamedTempFile::new().unwrap();
    new.write(&file, None).unwrap();
    file.seek(SeekFrom::Start(0)).unwrap();

    // Read back contents written to file
    let mut buf = String::new();
    file.read_to_string(&mut buf).unwrap();
    insta::assert_debug_snapshot!(buf);
  }
}
