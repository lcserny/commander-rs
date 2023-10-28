use commander::{config::{init_logging, init_config}, http};
use eyre::Result;

// TODO: see https://github.com/launchbadge/realworld-axum-sqlx for example

#[tokio::main]
async fn main() -> Result<()> {
    init_logging("commander.log")?;

    let settings = init_config("config/settings", "CMDR")?;
    http::serve(settings).await?;

    Ok(())
}
