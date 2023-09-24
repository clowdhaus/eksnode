use anyhow::{bail, Result};
use clap::Args;
use containerd_client as client;
use containerd_client::{
  services::v1::{images_client::ImagesClient, GetImageRequest, TransferRequest},
  tonic::Request,
  with_namespace, Client as ContainerdClient,
};
use prost_types::Any;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

const NAMESPACE: &str = "k8s.io";
const CONTAINERD_SOCK: &str = "/run/containerd/containerd.sock";

use crate::{ec2, ecr, eks, kubelet, utils};

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(group = clap::ArgGroup::new("pull").multiple(false).required(true))]
pub struct Image {
  /// Container image
  #[arg(short, long, group = "pull")]
  image: Option<String>,

  /// The container image intended namespace
  #[arg(short, long, default_value = NAMESPACE)]
  namespace: String,

  /// Cache common set of images on host/AMI
  #[arg(long, group = "pull")]
  cached_images: bool,

  /// Enable FIPS mode
  #[arg(long)]
  enable_fips: bool,
}

impl Image {
  /// Pull an image from a registry
  ///
  /// This is used to cache images on the host
  /// Ref: https://github.com/containerd/containerd/pull/7922
  ///
  /// Note: this is currently using the amazon-ecr-credential-helper
  /// for authentication to ECR (see ~/.docker/config.json)
  /// TODO: https://github.com/containerd/rust-extensions/issues/197
  // pub async fn pull(&self) -> Result<Option<utils::CmdResult>> {
  pub async fn pull(&self) -> Result<()> {
    match &self.image {
      Some(image) => {
        if self.exists().await? {
          Ok(())
        } else {
          pull_image(image, &self.namespace).await
        }
      }
      None => pull_cached_images(self.enable_fips).await,
    }
  }

  /// Check if the image exists in the namespace
  async fn exists(&self) -> Result<bool> {
    match &self.image {
      None => Ok(false),
      Some(_) => {
        let image = self.image.to_owned().unwrap();
        let channel = client::connect(CONTAINERD_SOCK).await?;
        let mut client = ImagesClient::new(channel);
        let img_req = GetImageRequest { name: image.to_owned() };

        match client.get(with_namespace!(img_req, NAMESPACE)).await {
          Ok(rsp) => {
            let rsp = rsp.into_inner();
            match rsp.image {
              Some(_) => {
                info!("Image found: {}", image);
                Ok(true)
              }
              None => Ok(false), // TODO - handle better?
            }
          }
          Err(_) => {
            info!("Image not found {}", image);
            Ok(false)
          }
        }
      }
    }
  }
}

async fn pull_image(image: &str, namespace: &str) -> Result<()> {
  info!("Pulling image: {image}");
  let out = utils::cmd_exec(
    "nerdctl",
    vec!["pull", "--unpack=false", &format!("--namespace={namespace}"), image],
  )?;

  if out.status == 0 {
    debug!("Image pulled {image}:\n {}", &out.stdout);
  } else {
    bail!("Failed to pull image: {image}");
  };

  Ok(())
}

async fn pull_cached_images(enable_fips: bool) -> Result<()> {
  let region = ec2::get_region().await?;
  let ecr_uri = ecr::get_ecr_uri(&region, enable_fips)?;
  let kubelet_version = kubelet::get_kubelet_version()?;
  let kubernetes_version = format!("{}.{}", kubelet_version.major, kubelet_version.minor);

  let images = get_images_to_cache(&ecr_uri, &kubernetes_version).await?;
  for image in &images {
    pull_image(image, NAMESPACE).await?;
  }

  tag_images(images).await?;

  Ok(())
}

// - `<ECR-ENDPOINT>/eks/kube-proxy:<default and latest>-eksbuild.<BUILD_VERSION>`
// - `<ECR-ENDPOINT>/eks/kube-proxy:<default and latest>-minimal-eksbuild.<BUILD_VERSION>`
// - `<ECR-ENDPOINT>/eks/pause:3.5`
// - `<ECR-ENDPOINT>/amazon-k8s-cni-init:<default and latest>`
// - `<ECR-ENDPOINT>/amazon-k8s-cni:<default and latest>`

async fn get_images_to_cache(ecr_uri: &str, kubernetes_version: &str) -> Result<Vec<String>> {
  let mut images = vec![format!("{ecr_uri}/eks/pause:3.9")];

  let kube_proxy_version = eks::get_addon_versions("kube-proxy", kubernetes_version).await?;
  images.push(format!("{ecr_uri}/eks/kube-proxy:{}", kube_proxy_version.default));
  images.push(format!("{ecr_uri}/eks/kube-proxy:{}", kube_proxy_version.latest));
  images
    .push(format!("{ecr_uri}/eks/kube-proxy:{}", kube_proxy_version.default).replace("eksbuild", "minimal-eksbuild"));
  images
    .push(format!("{ecr_uri}/eks/kube-proxy:{}", kube_proxy_version.latest).replace("eksbuild", "minimal-eksbuild"));

  let vpc_cni_version = eks::get_addon_versions("vpc-cni", kubernetes_version).await?;
  images.push(format!("{ecr_uri}/amazon-k8s-cni:{}", vpc_cni_version.default));
  images.push(format!("{ecr_uri}/amazon-k8s-cni-init:{}", vpc_cni_version.default));
  images.push(format!("{ecr_uri}/amazon-k8s-cni:{}", vpc_cni_version.latest));
  images.push(format!("{ecr_uri}/amazon-k8s-cni-init:{}", vpc_cni_version.latest));

  Ok(images)
}

async fn tag_images(images: Vec<String>) -> Result<()> {
  let client = ContainerdClient::from_path(CONTAINERD_SOCK)
    .await
    .expect("Failed to connect to containerd socket {CONTAINERD_SOCK}");

  for image in images {
    let source = Any {
      type_url: "types.containerd.io/opencontainers/runtime-spec/1/Spec".to_string(),
      value: image.clone().into_bytes(), // TODO
    };

    let destination = Any {
      type_url: "types.containerd.io/opencontainers/runtime-spec/1/Spec".to_string(),
      value: image.clone().into_bytes(), // TODO
    };

    let tx_req = TransferRequest {
      source: Some(source),
      destination: Some(destination),
      options: None,
    };

    let _resp = client.transfer().transfer(with_namespace!(tx_req, NAMESPACE)).await?;
  }

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn it_gets_ecr_uri_apeast1() {
    let ecr_uri = ecr::get_ecr_uri("us-east-1", false).unwrap();
    let imgs = get_images_to_cache(&ecr_uri, "1.27").await.unwrap();
    println!("{:#?}", imgs);

    assert_eq!(1, 1)
  }
}
