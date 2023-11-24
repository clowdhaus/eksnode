use anyhow::{bail, Result};
use clap::Args;
use containerd_client::{
  services::v1::{images_client::ImagesClient, CreateImageRequest, GetImageRequest, Image as ContainerdImage},
  tonic::{transport::Channel, Request},
  with_namespace, Client as ContainerdClient,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::{ec2, ecr, eks, kubelet, utils};

const NAMESPACE: &str = "k8s.io";
const CONTAINERD_SOCK: &str = "/run/containerd/containerd.sock";

#[derive(Args, Debug, Serialize, Deserialize)]
#[command(group = clap::ArgGroup::new("pull").multiple(false).required(true))]
pub struct PullImageInput {
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

impl PullImageInput {
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
        if !self.exists().await? {
          Ok(())
        } else {
          pull_image(image, &self.namespace).await?;
          Ok(()) // TODO - this is ugly
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
        let mut client = ContainerdClient::from_path(CONTAINERD_SOCK)
          .await
          .expect("Failed to connect to {CONTAINERD_SOCK}")
          .images();

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

async fn pull_image(image: &str, namespace: &str) -> Result<utils::CmdResult> {
  info!("Pulling image: {image}");
  let out = utils::cmd_exec(
    "nerdctl",
    vec!["pull", "--unpack=false", &format!("--namespace={namespace}"), image],
  )?;

  if out.status == 0 {
    debug!("Image pulled {image}: {}", &out.stdout);
  } else {
    bail!("Failed to pull image: {image}\n{}", &out.stderr);
  };

  Ok(out)
}

async fn pull_cached_images(enable_fips: bool) -> Result<()> {
  let region = ec2::get_region().await?;
  let kubelet_version = kubelet::get_kubelet_version()?;
  let kubernetes_version = format!("{}.{}", kubelet_version.major, kubelet_version.minor);

  let mut client = ContainerdClient::from_path(CONTAINERD_SOCK)
    .await
    .expect("Failed to connect to {CONTAINERD_SOCK}")
    .images();

  let images = get_images_to_cache(&region, enable_fips, &kubernetes_version).await?;
  for image in &images {
    // TODO - this should be integrated better when pulling with client and not nerdctl
    pull_image(image, NAMESPACE).await?;
    tag_image(image, &region, enable_fips, &mut client).await?;
  }

  Ok(())
}

async fn get_images_to_cache(region: &str, enable_fips: bool, kubernetes_version: &str) -> Result<Vec<String>> {
  let ecr_uri = ecr::get_ecr_uri(region, enable_fips)?;
  let mut images = vec![format!("{ecr_uri}/eks/pause:3.8")];

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

  // TODO - how to pull the correct image version
  // images.push(format!("{ecr_uri}/aws-network-policy-agent:v1.0.2-eksbuild.1"));

  Ok(images)
}

async fn tag_image(image: &str, cur_region: &str, enable_fips: bool, client: &mut ImagesClient<Channel>) -> Result<()> {
  for region in ec2::get_all_regions().await? {
    let img_req = GetImageRequest {
      name: image.to_string(),
    };

    // TODO - this feels like we should be passing around an image struct and simply updating one field
    let current_ecr_uri = ecr::get_ecr_uri(cur_region, enable_fips)?;
    let region_ecr_uri = ecr::get_ecr_uri(&region, enable_fips)?;
    if current_ecr_uri == region_ecr_uri {
      continue;
    }

    match client.get(with_namespace!(img_req, NAMESPACE)).await {
      Ok(rsp) => {
        if let Some(image) = rsp.into_inner().image {
          let tagged_name = image.name.replace(&current_ecr_uri, &region_ecr_uri);
          info!("Tagging image: {tagged_name}");
          let create_req = CreateImageRequest {
            image: Some(ContainerdImage {
              name: tagged_name,
              ..image
            }),
          };
          client.create(with_namespace!(create_req, NAMESPACE)).await?;
        }
      }
      Err(_) => bail!("Image not found, unable to tag"),
    }
  }

  Ok(())
}
#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn it_gets_images_to_cache_useast1_127() {
    match get_images_to_cache("us-east-1", false, "1.27").await {
      Ok(imgs) => insta::assert_debug_snapshot!(imgs),
      Err(e) => panic!("[ERROR] {:?}", e),
    }
  }
  #[tokio::test]
  async fn it_gets_images_to_cache_apeast1_127() {
    match get_images_to_cache("ap-east-1", false, "1.27").await {
      Ok(imgs) => insta::assert_debug_snapshot!(imgs),
      Err(e) => panic!("[ERROR] {:?}", e),
    }
  }
  #[tokio::test]
  async fn it_gets_images_to_cache_usgoveast1_fips_127() {
    match get_images_to_cache("us-gov-east-1", true, "1.27").await {
      Ok(imgs) => insta::assert_debug_snapshot!(imgs),
      Err(e) => panic!("[ERROR] {:?}", e),
    }
  }
  #[tokio::test]
  async fn it_gets_images_to_cache_useast1_124() {
    match get_images_to_cache("us-east-1", false, "1.24").await {
      Ok(imgs) => insta::assert_debug_snapshot!(imgs),
      Err(e) => panic!("[ERROR] {:?}", e),
    }
  }
  #[tokio::test]
  async fn it_gets_images_to_cache_apeast1_124() {
    match get_images_to_cache("ap-east-1", false, "1.24").await {
      Ok(imgs) => insta::assert_debug_snapshot!(imgs),
      Err(e) => panic!("[ERROR] {:?}", e),
    }
  }
  #[tokio::test]
  async fn it_gets_images_to_cache_usgoveast1_fips_124() {
    match get_images_to_cache("us-gov-east-1", true, "1.24").await {
      Ok(imgs) => insta::assert_debug_snapshot!(imgs),
      Err(e) => panic!("[ERROR] {:?}", e),
    }
  }
}
