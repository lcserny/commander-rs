use std::sync::Arc;

use commander::{
    http, mongo::MongoDbWrapper, db::DbClient, config::Settings,
};
use eyre::Result;
use mongodb::Client;
use utils::config::{init_config, init_logging};

#[tokio::main]
async fn main() -> Result<()> {
    init_logging("commander.log")?;

    let settings = Arc::new(init_config::<Settings>("config/settings", "CMDR")?);
    let client = Client::with_uri_str(&settings.mongodb.connection_url).await?;
    let db_client = DbClient::new(Arc::new(MongoDbWrapper::new(client, settings.clone())));

    http::serve(settings, db_client).await?;

    Ok(())
}
