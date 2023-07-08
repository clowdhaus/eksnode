use clap::{Parser, Subcommand};
use clap_verbosity_flag::Verbosity;

use crate::commands;

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
  /// Join an instance to the cluster
  Join(commands::join::Node),

  /// Calculate the maximum number of pods that can be scheduled on an instance
  ///
  /// Unlike `calculate_eni_max_pods` which calculates the theoretical limit based on ENIs,
  /// this function calculates the actual limit based on all of the preceding factors including
  /// the theoretical max pods limit.
  CalcMaxPods(commands::calc::MaxPods),
}

#[cfg(test)]
mod tests {
  use assert_cmd::prelude::*;
  use rstest::*;

  #[rstest]
  #[case("c6g.medium", "1.8.0", true, false, "5\n")]
  #[case("c6g.medium", "1.8.0", false, true, "8\n")]
  #[case("c6g.medium", "1.8.0", true, true, "5\n")]
  #[case("c6g.medium", "1.8.0", false, false, "8\n")]
  #[case("c6g.medium", "1.9.0", true, false, "5\n")]
  #[case("c6g.medium", "1.9.0", false, true, "98\n")]
  #[case("c6g.medium", "1.9.0", true, true, "50\n")]
  #[case("c6g.medium", "1.9.0", false, false, "8\n")]
  #[case("c5.large", "1.8.0", true, false, "20\n")]
  #[case("c5.large", "1.8.0", false, true, "29\n")]
  #[case("c5.large", "1.8.0", true, true, "20\n")]
  #[case("c5.large", "1.8.0", false, false, "29\n")]
  #[case("c5.large", "1.9.0", true, false, "20\n")]
  #[case("c5.large", "1.9.0", false, true, "110\n")]
  #[case("c5.large", "1.9.0", true, true, "110\n")]
  #[case("c5.large", "1.9.0", false, false, "29\n")]
  #[case("c5.xlarge", "1.8.0", true, false, "44\n")]
  #[case("c5.xlarge", "1.8.0", false, true, "58\n")]
  #[case("c5.xlarge", "1.8.0", true, true, "44\n")]
  #[case("c5.xlarge", "1.8.0", false, false, "58\n")]
  #[case("c5.xlarge", "1.9.0", true, false, "44\n")]
  #[case("c5.xlarge", "1.9.0", false, true, "110\n")]
  #[case("c5.xlarge", "1.9.0", true, true, "110\n")]
  #[case("c5.xlarge", "1.9.0", false, false, "58\n")]
  #[case("c5.4xlarge", "1.8.0", true, false, "110\n")]
  #[case("c5.4xlarge", "1.8.0", false, true, "110\n")]
  #[case("c5.4xlarge", "1.8.0", true, true, "110\n")]
  #[case("c5.4xlarge", "1.8.0", false, false, "110\n")]
  #[case("c5.4xlarge", "1.9.0", true, false, "110\n")]
  #[case("c5.4xlarge", "1.9.0", false, true, "110\n")]
  #[case("c5.4xlarge", "1.9.0", true, true, "110\n")]
  #[case("c5.4xlarge", "1.9.0", false, false, "110\n")]
  #[case("c6in.32xlarge", "1.8.0", true, false, "250\n")]
  #[case("c6in.32xlarge", "1.8.0", false, true, "250\n")]
  #[case("c6in.32xlarge", "1.8.0", true, true, "250\n")]
  #[case("c6in.32xlarge", "1.8.0", false, false, "250\n")]
  #[case("c6in.32xlarge", "1.9.0", true, false, "250\n")]
  #[case("c6in.32xlarge", "1.9.0", false, true, "250\n")]
  #[case("c6in.32xlarge", "1.9.0", true, true, "250\n")]
  #[case("c6in.32xlarge", "1.9.0", false, false, "250\n")]
  #[case("c5.18xlarge", "1.8.0", true, false, "250\n")]
  #[case("c5.18xlarge", "1.8.0", false, true, "250\n")]
  #[case("c5.18xlarge", "1.8.0", true, true, "250\n")]
  #[case("c5.18xlarge", "1.8.0", false, false, "250\n")]
  #[case("c5.18xlarge", "1.9.0", true, false, "250\n")]
  #[case("c5.18xlarge", "1.9.0", false, true, "250\n")]
  #[case("c5.18xlarge", "1.9.0", true, true, "250\n")]
  #[case("c5.18xlarge", "1.9.0", false, false, "250\n")]
  #[case("d3.2xlarge", "1.8.0", true, false, "14\n")]
  #[case("d3.2xlarge", "1.8.0", false, true, "18\n")]
  #[case("d3.2xlarge", "1.8.0", true, true, "14\n")]
  #[case("d3.2xlarge", "1.8.0", false, false, "18\n")]
  #[case("d3.2xlarge", "1.9.0", true, false, "14\n")]
  #[case("d3.2xlarge", "1.9.0", false, true, "110\n")]
  #[case("d3.2xlarge", "1.9.0", true, true, "110\n")]
  #[case("d3.2xlarge", "1.9.0", false, false, "18\n")]
  #[case("d3.4xlarge", "1.8.0", true, false, "29\n")]
  #[case("d3.4xlarge", "1.8.0", false, true, "38\n")]
  #[case("d3.4xlarge", "1.8.0", true, true, "29\n")]
  #[case("d3.4xlarge", "1.8.0", false, false, "38\n")]
  #[case("d3.4xlarge", "1.9.0", true, false, "29\n")]
  #[case("d3.4xlarge", "1.9.0", false, true, "110\n")]
  #[case("d3.4xlarge", "1.9.0", true, true, "110\n")]
  #[case("d3.4xlarge", "1.9.0", false, false, "38\n")]
  #[case("d3.8xlarge", "1.8.0", true, false, "40\n")]
  #[case("d3.8xlarge", "1.8.0", false, true, "59\n")]
  #[case("d3.8xlarge", "1.8.0", true, true, "40\n")]
  #[case("d3.8xlarge", "1.8.0", false, false, "59\n")]
  #[case("d3.8xlarge", "1.9.0", true, false, "40\n")]
  #[case("d3.8xlarge", "1.9.0", false, true, "250\n")]
  #[case("d3.8xlarge", "1.9.0", true, true, "250\n")]
  #[case("d3.8xlarge", "1.9.0", false, false, "59\n")]
  #[case("d3.xlarge", "1.8.0", true, false, "8\n")]
  #[case("d3.xlarge", "1.8.0", false, true, "10\n")]
  #[case("d3.xlarge", "1.8.0", true, true, "8\n")]
  #[case("d3.xlarge", "1.8.0", false, false, "10\n")]
  #[case("d3.xlarge", "1.9.0", true, false, "8\n")]
  #[case("d3.xlarge", "1.9.0", false, true, "110\n")]
  #[case("d3.xlarge", "1.9.0", true, true, "98\n")]
  #[case("d3.xlarge", "1.9.0", false, false, "10\n")]
  #[case("d3en.8xlarge", "1.8.0", true, false, "59\n")]
  #[case("d3en.8xlarge", "1.8.0", false, true, "78\n")]
  #[case("d3en.8xlarge", "1.8.0", true, true, "59\n")]
  #[case("d3en.8xlarge", "1.8.0", false, false, "78\n")]
  #[case("d3en.8xlarge", "1.9.0", true, false, "59\n")]
  #[case("d3en.8xlarge", "1.9.0", false, true, "250\n")]
  #[case("d3en.8xlarge", "1.9.0", true, true, "250\n")]
  #[case("d3en.8xlarge", "1.9.0", false, false, "78\n")]
  #[case("f1.16xlarge", "1.8.0", true, false, "250\n")]
  #[case("f1.16xlarge", "1.8.0", false, true, "250\n")]
  #[case("f1.16xlarge", "1.8.0", true, true, "250\n")]
  #[case("f1.16xlarge", "1.8.0", false, false, "250\n")]
  #[case("f1.16xlarge", "1.9.0", true, false, "250\n")]
  #[case("f1.16xlarge", "1.9.0", false, true, "250\n")]
  #[case("f1.16xlarge", "1.9.0", true, true, "250\n")]
  #[case("f1.16xlarge", "1.9.0", false, false, "250\n")]
  #[case("g5g.4xlarge", "1.8.0", true, false, "110\n")]
  #[case("g5g.4xlarge", "1.8.0", false, true, "110\n")]
  #[case("g5g.4xlarge", "1.8.0", true, true, "110\n")]
  #[case("g5g.4xlarge", "1.8.0", false, false, "110\n")]
  #[case("g5g.4xlarge", "1.9.0", true, false, "110\n")]
  #[case("g5g.4xlarge", "1.9.0", false, true, "110\n")]
  #[case("g5g.4xlarge", "1.9.0", true, true, "110\n")]
  #[case("g5g.4xlarge", "1.9.0", false, false, "110\n")]
  #[case("g5g.xlarge", "1.8.0", true, false, "44\n")]
  #[case("g5g.xlarge", "1.8.0", false, true, "58\n")]
  #[case("g5g.xlarge", "1.8.0", true, true, "44\n")]
  #[case("g5g.xlarge", "1.8.0", false, false, "58\n")]
  #[case("g5g.xlarge", "1.9.0", true, false, "44\n")]
  #[case("g5g.xlarge", "1.9.0", false, true, "110\n")]
  #[case("g5g.xlarge", "1.9.0", true, true, "110\n")]
  #[case("g5g.xlarge", "1.9.0", false, false, "58\n")]
  #[case("inf1.24xlarge", "1.8.0", true, false, "250\n")]
  #[case("inf1.24xlarge", "1.8.0", false, true, "250\n")]
  #[case("inf1.24xlarge", "1.8.0", true, true, "250\n")]
  #[case("inf1.24xlarge", "1.8.0", false, false, "250\n")]
  #[case("inf1.24xlarge", "1.9.0", true, false, "250\n")]
  #[case("inf1.24xlarge", "1.9.0", false, true, "250\n")]
  #[case("inf1.24xlarge", "1.9.0", true, true, "250\n")]
  #[case("inf1.24xlarge", "1.9.0", false, false, "250\n")]
  #[case("trn1.32xlarge", "1.8.0", true, false, "198\n")]
  #[case("trn1.32xlarge", "1.8.0", false, true, "247\n")]
  #[case("trn1.32xlarge", "1.8.0", true, true, "198\n")]
  #[case("trn1.32xlarge", "1.8.0", false, false, "247\n")]
  #[case("trn1.32xlarge", "1.9.0", true, false, "198\n")]
  #[case("trn1.32xlarge", "1.9.0", false, true, "250\n")]
  #[case("trn1.32xlarge", "1.9.0", true, true, "250\n")]
  #[case("trn1.32xlarge", "1.9.0", false, false, "247\n")]
  #[case("m1.medium", "1.8.0", true, false, "7\n")]
  #[case("m1.medium", "1.8.0", false, true, "12\n")]
  #[case("m1.medium", "1.8.0", true, true, "7\n")]
  #[case("m1.medium", "1.8.0", false, false, "12\n")]
  #[case("m1.medium", "1.9.0", true, false, "7\n")]
  #[case("m1.medium", "1.9.0", false, true, "12\n")]
  #[case("m1.medium", "1.9.0", true, true, "7\n")]
  #[case("m1.medium", "1.9.0", false, false, "12\n")]
  #[case("m4.large", "1.8.0", true, false, "11\n")]
  #[case("m4.large", "1.8.0", false, true, "20\n")]
  #[case("m4.large", "1.8.0", true, true, "11\n")]
  #[case("m4.large", "1.8.0", false, false, "20\n")]
  #[case("m4.large", "1.9.0", true, false, "11\n")]
  #[case("m4.large", "1.9.0", false, true, "20\n")]
  #[case("m4.large", "1.9.0", true, true, "11\n")]
  #[case("m4.large", "1.9.0", false, false, "20\n")]
  #[case("t1.micro", "1.8.0", true, false, "3\n")]
  #[case("t1.micro", "1.8.0", false, true, "4\n")]
  #[case("t1.micro", "1.8.0", true, true, "3\n")]
  #[case("t1.micro", "1.8.0", false, false, "4\n")]
  #[case("t1.micro", "1.9.0", true, false, "3\n")]
  #[case("t1.micro", "1.9.0", false, true, "4\n")]
  #[case("t1.micro", "1.9.0", true, true, "3\n")]
  #[case("t1.micro", "1.9.0", false, false, "4\n")]
  #[case("t2.large", "1.8.0", true, false, "24\n")]
  #[case("t2.large", "1.8.0", false, true, "35\n")]
  #[case("t2.large", "1.8.0", true, true, "24\n")]
  #[case("t2.large", "1.8.0", false, false, "35\n")]
  #[case("t2.large", "1.9.0", true, false, "24\n")]
  #[case("t2.large", "1.9.0", false, true, "35\n")]
  #[case("t2.large", "1.9.0", true, true, "24\n")]
  #[case("t2.large", "1.9.0", false, false, "35\n")]
  #[case("t2.medium", "1.8.0", true, false, "12\n")]
  #[case("t2.medium", "1.8.0", false, true, "17\n")]
  #[case("t2.medium", "1.8.0", true, true, "12\n")]
  #[case("t2.medium", "1.8.0", false, false, "17\n")]
  #[case("t2.medium", "1.9.0", true, false, "12\n")]
  #[case("t2.medium", "1.9.0", false, true, "17\n")]
  #[case("t2.medium", "1.9.0", true, true, "12\n")]
  #[case("t2.medium", "1.9.0", false, false, "17\n")]
  #[case("t2.small", "1.8.0", true, false, "8\n")]
  #[case("t2.small", "1.8.0", false, true, "11\n")]
  #[case("t2.small", "1.8.0", true, true, "8\n")]
  #[case("t2.small", "1.8.0", false, false, "11\n")]
  #[case("t2.small", "1.9.0", true, false, "8\n")]
  #[case("t2.small", "1.9.0", false, true, "11\n")]
  #[case("t2.small", "1.9.0", true, true, "8\n")]
  #[case("t2.small", "1.9.0", false, false, "11\n")]
  fn calc_max_pods_test(
    #[case] instance_type: &str,
    #[case] cni_version: &str,
    #[case] custom_networking: bool,
    #[case] prefix_delegation: bool,
    #[case] expected: String,
  ) {
    let bin_under_test = escargot::CargoBuild::new()
      .bin("eksnode")
      .current_release()
      .current_target()
      .run()
      .unwrap();

    let mut cmd = bin_under_test.command();

    cmd
      .arg("calc-max-pods")
      .arg("--instance-type")
      .arg(instance_type)
      .arg("--cni-version")
      .arg(cni_version);

    if custom_networking {
      cmd.arg("--cni-custom-networking-enabled");
    }

    if prefix_delegation {
      cmd.arg("--cni-prefix-delegation-enabled");
    }

    cmd.assert().success().stdout(expected);
  }
}
