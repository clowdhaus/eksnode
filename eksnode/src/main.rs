use anyhow::Result;
use clap::Parser;
use eksnode::{Cli, Commands};
use tracing_log::AsTrace;
use tracing_subscriber::FmtSubscriber;

#[cfg(not(tarpaulin_include))]
#[tokio::main]
async fn main() -> Result<()> {
  let cli = Cli::parse();
  let subscriber = FmtSubscriber::builder()
    .with_max_level(cli.verbose.log_level_filter().as_trace())
    .without_time()
    .with_ansi(!cli.no_color)
    .finish();
  tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber failed");

  match &cli.command {
    Commands::CalculateMaxPods(maxpods) => maxpods.result().await,
    Commands::Pull(image) => image.pull().await,
    Commands::Join(node) => node.join_node_to_cluster().await,
    Commands::Validate(validate) => validate.validate().await,
  }
}
