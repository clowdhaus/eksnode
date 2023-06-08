use anyhow::Result;
use aws_config::{imds::client::Client, provider_config::ProviderConfig};
use tokio::time::Duration;

/// Get the IMDS client
async fn get_client() -> Result<Client> {
  let config = ProviderConfig::with_default_region().await;
  let client = Client::builder()
    // Start with the shared environment configuration
    .configure(&config)
    .max_attempts(5)
    .token_ttl(Duration::from_secs(900))
    .connect_timeout(Duration::from_secs(5))
    .read_timeout(Duration::from_secs(5))
    .build()
    .await?;

  Ok(client)
}

pub async fn get_imds_data() -> Result<()> {
  let client = get_client().await?;
  let metadata = client.get("/latest/meta-data").await?;

  println!("{metadata:#?}");

  Ok(())
}
