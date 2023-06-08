use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use clap_verbosity_flag::Verbosity;
use serde::{Deserialize, Serialize};

/// Styles for CLI
fn get_styles() -> clap::builder::Styles {
  clap::builder::Styles::styled()
    .header(
      anstyle::Style::new()
        .bold()
        .underline()
        .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green))),
    )
    .literal(
      anstyle::Style::new()
        .bold()
        .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::BrightCyan))),
    )
    .usage(
      anstyle::Style::new()
        .bold()
        .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green))),
    )
    .placeholder(
      anstyle::Style::new()
        .bold()
        .underline()
        .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow))),
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
  Bootstrap(Bootstrap),
}

#[derive(Args, Debug, Serialize, Deserialize)]
pub struct Bootstrap {
  /// GitHub organization
  #[arg(short, long)]
  organization: String,

  /// GitHub access token environment variable
  #[arg(long)]
  env_var: String,

  /// Exclude repositories matching this regex
  #[arg(short, long, default_value = "")]
  include: String,

  /// Exclude repositories matching this regex
  #[arg(short, long, default_value = "")]
  exclude: String,
}

impl Bootstrap {
  pub async fn run(&self) -> Result<()> {
    Ok(())
  }
}
