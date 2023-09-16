use anyhow::Result;
use clap::Args;
use containerd_client as client;
use containerd_client::{
  services::v1::{images_client::ImagesClient, GetImageRequest},
  tonic::Request,
  with_namespace,
};
use serde::{Deserialize, Serialize};
use tracing::info;

const NAMESPACE: &str = "k8s.io";
const CONTAINERD_SOCK: &str = "/run/containerd/containerd.sock";

use crate::{ecr, utils};

#[derive(Args, Debug, Serialize, Deserialize)]
pub struct Image {
  /// Container image
  #[arg(short, long, env)]
  image: String,

  /// The container image intended namespace
  #[arg(short, long, env, default_value = NAMESPACE)]
  namespace: String,
}

impl Image {
  /// Fetch all content for the image into containerd
  ///
  /// This is used to cache images on the host
  /// Ref: https://github.com/containerd/containerd/pull/7922
  /// TODO: https://github.com/containerd/rust-extensions/issues/197
  pub async fn fetch(&self) -> Result<()> {
    if self.exists().await? {
      return Ok(());
    }

    let client = ecr::get_client().await?;
    let token = ecr::get_authorization_token(&client).await?;

    utils::cmd_exec(
      "sudo",
      vec![
        "ctr",
        "--namespace",
        &self.namespace,
        "content",
        "fetch",
        &self.image,
        "--user",
        &format!("AWS:{token}"),
      ],
    )?;

    Ok(())
  }

  /// Check if the image exists in the namespace
  async fn exists(&self) -> Result<bool> {
    let channel = client::connect(CONTAINERD_SOCK).await?;
    let mut client = ImagesClient::new(channel);
    let img_req = GetImageRequest {
      name: self.image.to_owned(),
    };

    match client.get(with_namespace!(img_req, NAMESPACE)).await {
      Ok(rsp) => {
        let rsp = rsp.into_inner();
        match rsp.image {
          Some(_) => {
            info!("Image found: {}", self.image);
            Ok(true)
          }
          None => Ok(false), // TODO - handle better?
        }
      }
      Err(_) => {
        info!("Image not found - fetching {}", self.image);
        Ok(false)
      }
    }
  }
}
