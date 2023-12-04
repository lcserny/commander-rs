use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use axum::{extract::State, routing::post, Json, Router};
use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};
use tracing::log::info;
use utoipa::ToSchema;

use crate::http::{self};

use self::shutdown::ShutdownCommand;

pub mod shutdown;

pub fn router() -> Router {
    Router::new().route("/api/v1/commands", post(execute_cmd))
        .with_state(Arc::new(init_commands()))
}

fn init_commands() -> HashMap<String, CommandsKind> {
    let mut commands = HashMap::new();
    commands.insert( shutdown::KEY.to_owned(), CommandsKind::ShutdownCommand(ShutdownCommand));

    commands
}

pub struct StubCommand {
    pub status: Status,
}

#[async_trait]
impl Command for StubCommand {
    async fn execute(&self, mut _params: Vec<String>) -> eyre::Result<CommandResp> {
        Ok(CommandResp {
            status: self.status.clone(),
        })
    }
}

#[enum_dispatch]
pub enum CommandsKind {
    ShutdownCommand,
    StubCommand,
}

#[async_trait]
#[enum_dispatch(CommandsKind)]
pub trait Command {
    async fn execute(&self, mut params: Vec<String>) -> eyre::Result<CommandResp>;
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CommandReq {
    pub name: String,
    pub params: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CommandResp {
    pub status: Status,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum Status {
    Success,
    NotFound,
    Failed,
}

#[utoipa::path(post, path = "/api/v1/commands",
    request_body = CommandReq,
    responses(
        (status = 200, description = "Execute command given", body = CommandResp)
    )
)]
pub async fn execute_cmd(
    State(commands): State<Arc<HashMap<String, CommandsKind>>>,
    Json(req): Json<CommandReq>,
) -> http::Result<Json<CommandResp>> {
    info!("execute_cmd request received with body: {:#?}", req);

    let resp = match commands
        .iter()
        .filter(|(k, _)| *k == &req.name)
        .map(|(_, cmd)| cmd)
        .next()
    {
        Some(cmd) => cmd.execute(req.params.unwrap_or_else(|| vec![])).await?,
        None => CommandResp { status: Status::NotFound },
    };

    Ok(Json(resp))
}
