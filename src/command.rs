use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use tracing::log::info;

use crate::http::{self};

use self::shutdown::ShutdownCommandExecutor;

pub mod shutdown;

pub fn router() -> Router {
    Router::new()
        .route("/api/v1/commands", post(execute_cmd))
        .with_state(Arc::new(init_commands()))
}

type Commands = HashMap<String, Box<dyn CommandExecutor + Send + Sync>>;

fn init_commands() -> Commands {
    let mut commands = Commands::new();
    commands.insert(shutdown::KEY.to_owned(), Box::new(ShutdownCommandExecutor));
    // TODO: add more commands

    commands
}

#[async_trait]
pub trait CommandExecutor {
    async fn execute(&self, mut params: Vec<String>) -> eyre::Result<CommandResp>;
}

#[derive(Debug, Serialize, Deserialize)]
struct CommandReq {
    name: String,
    params: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandResp {
    pub status: Status,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Status {
    Success,
    NotFound,
    Failed,
}

async fn execute_cmd(
    State(commands): State<Arc<Commands>>,
    Json(req): Json<CommandReq>,
) -> http::Result<Json<CommandResp>> {
    info!("execute_cmd request received with body: {:#?}", req);

    Ok(Json(
        match commands
            .iter()
            .filter(|(k, _)| *k == &req.name)
            .map(|(_, cmd)| cmd)
            .next()
        {
            Some(cmd) => cmd.execute(req.params.unwrap_or_else(|| vec![])).await?,
            None => CommandResp {
                status: Status::NotFound,
            },
        },
    ))
}
