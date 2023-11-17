use std::sync::Arc;

use commander::{
    config::{init_config, init_logging},
    http, mongo::MongoDbWrapper, db::DbClient,
};
use eyre::Result;
use mongodb::Client;

// TODO: add tests

#[tokio::main]
async fn main() -> Result<()> {
    init_logging("commander.log")?;

    let settings = Arc::new(init_config("config/settings", "CMDR")?);
    let client = Client::with_uri_str(&settings.mongodb.connection_url).await?;
    let db_client = DbClient::new(Arc::new(MongoDbWrapper::new(client, settings.clone())));

    http::serve(settings, db_client).await?;

    Ok(())
}
