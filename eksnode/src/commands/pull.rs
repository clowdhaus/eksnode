use anyhow::Result;
use clap::Args;
use containerd_client as client;
use containerd_client::{
  services::v1::{images_client::ImagesClient, GetImageRequest},
  tonic::Request,
  with_namespace,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

const NAMESPACE: &str = "k8s.io";
const CONTAINERD_SOCK: &str = "/run/containerd/containerd.sock";

use crate::utils;

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
          return Ok(());
        }

        let out = utils::cmd_exec(
          "nerdctl",
          vec![
            "pull",
            "--unpack=false",
            &format!("--namespace={}", &self.namespace),
            &image,
          ],
        )?;
        debug!("Pull image {}:\n {}", &image, &out.stdout);

        Ok(())
      }
      None => pull_cached_images(),
    }
  }

  // eksnode pull --image=602401143452.dkr.ecr.us-east-1.amazonaws.com/eks/pause:3.9 -vvv

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
            info!("Image not found - pull {}", image);
            Ok(false)
          }
        }
      }
    }
  }
}

fn pull_cached_images() -> Result<()> {
  todo!()
}
