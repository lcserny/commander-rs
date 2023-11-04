use commander::{config::{init_logging, init_config}, http};
use eyre::Result;
use mongodb::Client;

#[tokio::main]
async fn main() -> Result<()> {
    init_logging("commander.log")?;

    let settings = init_config("config/settings", "CMDR")?;
    let client = Client::with_uri_str(&settings.mongodb.connection_url).await?;

    http::serve(settings, client).await?;

    Ok(())
}
