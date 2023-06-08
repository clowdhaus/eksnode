use std::process;

use anyhow::Result;
use eksami::{Cli, Commands};
use clap::Parser;
use tracing::debug;
use tracing_log::AsTrace;
use tracing_subscriber::FmtSubscriber;

#[cfg(not(tarpaulin_include))]
#[tokio::main]
async fn main() -> Result<()> {
  let cli = Cli::parse();
  let subscriber = FmtSubscriber::builder()
    .with_max_level(cli.verbose.log_level_filter().as_trace())
    .without_time()
    .finish();
  tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber failed");

  match &cli.command {
    Commands::Bootstrap(update) => match update.run().await {
      Ok(result) => debug!("{:#?}", result),
      Err(err) => {
        eprintln!("{err}");
        process::exit(2);
      }
    },
  }

  Ok(())
}
