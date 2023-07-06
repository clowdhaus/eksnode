use std::process;

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
    .pretty()
    .finish();
  tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber failed");

  match &cli.command {
    Commands::Bootstrap(bstrap) => match bstrap.join_node_to_cluster().await {
      Ok(_) => Ok(()),
      Err(err) => {
        eprintln!("{err}");
        process::exit(2);
      }
    },
    Commands::CalcMaxPods(maxpods) => match maxpods.calc().await {
      Ok(_) => Ok(()),
      Err(err) => {
        eprintln!("{err}");
        process::exit(2);
      }
    },
  }
}
