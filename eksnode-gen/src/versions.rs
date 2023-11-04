use std::{
  collections::{BTreeMap, HashSet},
  fs::{self, File},
  io::BufReader,
  path::Path,
};

use anyhow::Result;
use aws_sdk_s3::{config::Region, Client};
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// The Amazon EKS S3 bucket where the artifacts are stored
static S3_BUCKET_NAME: &str = "amazon-eks";

/// The minimum supported Kubernetes version for this project
/// EKS retains all of the build artifacts in S3, but we do not output all of them
static MIN_SUPPORTED_KUBERNETES_VERSION: i32 = 24;

#[derive(Debug, Serialize, Deserialize)]
struct Versions {
  versions: BTreeMap<String, Version>,

  /// Preserve any additional values provided in the variables file
  #[serde(flatten, skip_serializing_if = "BTreeMap::is_empty")]
  other: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Version {
  /// The <major>.<minor>.<patch> version provided by EKS
  kubernetes_version: String,

  /// The date the artifacts were built and stored in S3,
  /// which is also the prefix under which the artifacts are stored
  kubernetes_build_date: String,

  /// The version of runc - this is not pulled from S3, but statically set in `versions.yaml`
  runc_version: String,

  /// The version of containerd - this is not pulled from S3, but statically set in `versions.yaml`
  containerd_version: String,

  /// The version of nerdctl - this is not pulled from S3, but statically set in `versions.yaml`
  /// nerdctl is used in place of ctr
  nerdctl_version: String,

  /// The version of the CNI plugin - this is not pulled from S3, but statically set in `versions.yaml`
  cni_plugin_version: String,
}

impl Versions {
  pub fn read<P: AsRef<Path>>(path: P) -> Result<Self> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let versions: Versions = serde_yaml::from_reader(reader)?;

    Ok(versions)
  }

  pub fn write<P: AsRef<Path>>(&self, path: P, cur_dir: &Path) -> Result<()> {
    let mut handlebars = Handlebars::new();
    let template = cur_dir.join("eksnode-gen").join("templates").join("versions.tpl");
    handlebars.register_template_file("tpl", template)?;

    let other = serde_yaml::to_string(&self.other)?;
    let data = json!({"versions": self.versions, "other": other});

    let rendered = handlebars.render("tpl", &data)?;
    fs::write(path, rendered).map_err(anyhow::Error::from)
  }
}

pub async fn update_artifact_versions(cur_dir: &Path) -> Result<()> {
  let dest_path = cur_dir.join("ami").join("playbooks").join("vars").join("versions.yaml");

  // Open existing file in project
  let mut versions = Versions::read(&dest_path)?;

  let build_date_versions = get_build_date_versions().await?;
  for (k, v) in &mut versions.versions {
    v.kubernetes_build_date = build_date_versions.get(k).unwrap().kubernetes_build_date.to_owned();
    v.kubernetes_version = build_date_versions.get(k).unwrap().kubernetes_version.to_owned();
  }

  versions.write(&dest_path, cur_dir)
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct BuildDateVersion {
  kubernetes_build_date: String,
  kubernetes_version: String,
}

async fn get_build_date_versions() -> Result<BTreeMap<String, BuildDateVersion>> {
  let config = aws_config::from_env().region(Region::new("us-west-2")).load().await;
  let client = Client::new(&config);

  let mut object_paginator = client
    .list_objects_v2()
    .bucket(S3_BUCKET_NAME)
    .prefix("1.")
    .into_paginator()
    .send();

  let mut build_dates = HashSet::new();

  // Reduces list of files down to unique version/build-date
  while let Some(page) = object_paginator.next().await {
    for obj in page?.contents.unwrap_or_default().iter() {
      let key = obj.key().unwrap();
      // <kubernetes-ver>/<build-date>/<artifact-type>/<os-type>/<os-arch>/<name>
      let split = key.split('/').collect::<Vec<&str>>();
      build_dates.insert((split[0].to_owned(), split[1].to_owned()));
    }
  }

  let mut max_versions = BTreeMap::new();

  for (kubernetes_version, build_date) in build_dates.iter() {
    let version_split = kubernetes_version.split('.').collect::<Vec<&str>>();
    let minor_version = version_split[..2].join(".");
    let entry = BuildDateVersion {
      kubernetes_build_date: build_date.to_owned(),
      kubernetes_version: kubernetes_version.to_owned(),
    };

    if version_split[1].parse::<i32>().unwrap() >= MIN_SUPPORTED_KUBERNETES_VERSION {
      match max_versions.get(&minor_version) {
        Some(max_build_date) => {
          if &entry > max_build_date {
            max_versions.insert(minor_version, entry);
          }
        }
        None => {
          max_versions.insert(minor_version, entry);
        }
      }
    }
  }

  Ok(max_versions)
}
