use anstyle::{AnsiColor, Color, Style};
use clap::{builder::Styles, Parser, Subcommand};
use clap_verbosity_flag::Verbosity;

use crate::commands;

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
    .placeholder(Style::new().bold().fg_color(Some(Color::Ansi(AnsiColor::Yellow))))
    .error(Style::new().bold().fg_color(Some(Color::Ansi(AnsiColor::BrightRed))))
}

#[derive(Debug, Parser)]
#[command(author, about, version)]
#[command(author, version, about)]
#[command(propagate_version = true)]
#[command(styles=get_styles())]
pub struct Cli {
  #[command(subcommand)]
  pub command: Commands,

  #[clap(flatten)]
  pub verbose: Verbosity,

  /// Disable colors on logged output
  #[arg(long, global = true, default_value = "false")]
  pub no_color: bool,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
  /// Calculate the maximum number of pods that can be scheduled on an instance
  ///
  /// Unlike `calculate_eni_max_pods` which calculates the theoretical limit based on ENIs,
  /// this function calculates the actual limit based on all of the preceding factors including
  /// the theoretical max pods limit.
  CalculateMaxPods(commands::calculate::MaxPods),

  /// Get the versions of the components installed
  GetVersions(commands::versions::Versions),

  /// Expose and collect details about the node for debugging purposes
  Debug(commands::debug::Debug),

  /// Pull images from a registry
  ///
  /// Supports pulling one image as specified or for pulling commonly used images
  /// to be cached on the host/AMI
  PullImage(commands::pull::ImageInput),

  /// Join an instance to the cluster
  JoinCluster(commands::join::Node),

  /// Validate the node configuration
  ValidateNode(commands::validate::Validation),
}
