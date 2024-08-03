use anstyle::{AnsiColor, Color, Style};
use anyhow::Result;
use aws_config::SdkConfig;
use aws_sdk_ec2::{
  config::{self, retry::RetryConfig},
  Client,
};
use clap::{builder::Styles, Parser, Subcommand};
use clap_verbosity_flag::Verbosity;

pub mod ec2;
pub mod versions;

/// Construct and return the EC2 client
pub(crate) async fn get_client(config: SdkConfig, retries: u32) -> Result<Client> {
  let client = Client::from_conf(
    config::Builder::from(&config)
      .retry_config(RetryConfig::standard().with_max_attempts(retries))
      .build(),
  );

  Ok(client)
}

/// Styles for CLI
fn get_styles() -> Styles {
  Styles::styled()
    .header(
      Style::new()
        .bold()
        .underline()
        .fg_color(Some(Color::Ansi(AnsiColor::Green))),
    )
    .literal(Style::new().bold().fg_color(Some(Color::Ansi(AnsiColor::Cyan))))
    .usage(
      Style::new()
        .bold()
        .underline()
        .fg_color(Some(Color::Ansi(AnsiColor::Green))),
    )
    .placeholder(
      Style::new()
        .bold()
        .underline()
        .fg_color(Some(Color::Ansi(AnsiColor::Yellow))),
    )
}

#[derive(Debug, Parser)]
#[command(author, about, version)]
#[command(propagate_version = true)]
#[command(styles=get_styles())]
pub struct Cli {
  #[command(subcommand)]
  pub command: Commands,

  #[clap(flatten)]
  pub verbose: Verbosity,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
  /// Update the EC2 file `ec2-instances.yaml` with the latest data
  UpdateEc2,

  /// Update the Ansible playbook variables `versions.yaml` with the latest artifact data from S3
  UpdateArtifactVersions,
}
