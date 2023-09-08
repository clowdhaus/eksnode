//! Script-like crate for generating files used by `eksnode` or image creation process
use std::{env, process};

use anyhow::Result;
use clap::Parser;
use eksnode_gen::{ec2, versions, Cli, Commands};
use tracing_log::AsTrace;
use tracing_subscriber::FmtSubscriber;

/// Generates files that are consumed by the `eksnode` project
///
/// ```bash
/// cargo run --bin eksnode-gen <COMMAND>
// / ```
#[cfg(not(tarpaulin_include))]
#[tokio::main]
async fn main() -> Result<()> {
  let cli = Cli::parse();
  let subscriber = FmtSubscriber::builder()
    .with_max_level(cli.verbose.log_level_filter().as_trace())
    .without_time()
    .finish();
  tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber failed");

  let cur_exe = env::current_exe()?;
  let cur_dir = cur_exe.parent().unwrap().parent().unwrap().parent().unwrap();

  match &cli.command {
    // Creates the `ec2-instances.yaml` which embeds EC2 details into the `eksnode` binary
    // to reduce the number of AWS API calls when provisioning a node and joining it to a cluster
    Commands::UpdateEc2 => match ec2::write_files(cur_dir).await {
      Ok(_) => Ok(()),
      Err(err) => {
        eprintln!("{err}");
        process::exit(2);
      }
    },

    // Updates the `versions.yaml` file which is used by the AMI build process to map the correct
    // artifact version to the given Kubernetes version. EKS vended artifacts are built and stored in S3
    // and are not available via a public API. This file is used to map the Kubernetes version to the
    // correct artifact version.
    Commands::UpdateArtifactVersions => match versions::update_artifact_versions(cur_dir).await {
      Ok(_) => Ok(()),
      Err(err) => {
        eprintln!("{err}");
        process::exit(2);
      }
    },
  }
}
