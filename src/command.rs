use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use axum::{extract::State, routing::post, Json, Router};
use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};
use tracing::log::info;

use crate::http::{self};

use self::shutdown::ShutdownCommand;

pub mod shutdown;

pub fn router() -> Router {
    Router::new()
        .route("/api/v1/commands", post(execute_cmd))
        .with_state(Arc::new(init_commands()))
}

fn init_commands() -> HashMap<String, CommandsKind> {
    let mut commands = HashMap::new();
    commands.insert(shutdown::KEY.to_owned(), CommandsKind::ShutdownCommand(ShutdownCommand));

    commands
}

struct StubCommand {
    status: Status,
}

#[async_trait]
impl Command for StubCommand {
    async fn execute(&self, mut _params: Vec<String>) -> eyre::Result<CommandResp> {
        Ok(CommandResp { status: self.status.clone() })
    }
}

#[enum_dispatch]
enum CommandsKind {
    ShutdownCommand,
    StubCommand,
}

#[async_trait]
#[enum_dispatch(CommandsKind)]
pub trait Command {
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum Status {
    Success,
    NotFound,
    Failed,
}

async fn execute_cmd(
    State(commands): State<Arc<HashMap<String, CommandsKind>>>,
    Json(req): Json<CommandReq>,
) -> http::Result<Json<CommandResp>> {
    info!("execute_cmd request received with body: {:#?}", req);

    let resp = match commands.iter()
        .filter(|(k, _)| *k == &req.name)
        .map(|(_, cmd)| cmd)
        .next() {
            Some(cmd) => cmd.execute(req.params.unwrap_or_else(|| vec![])).await?,
            None => CommandResp { status: Status::NotFound, },
        };

    Ok(Json(resp))
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use axum::{Json, extract::State};

    use super::{CommandsKind, Status, StubCommand, CommandReq, execute_cmd};

    #[tokio::test]
    async fn commands_execute_correctly() {
        let stub_cmd = "stub";
        let status = Status::Success;

        let mut commands = HashMap::new();
        commands.insert(stub_cmd.to_owned(), CommandsKind::StubCommand(StubCommand { status: status.clone() }));
        let req = CommandReq { name: stub_cmd.to_owned(), params: None };

        let resp = execute_cmd(State(Arc::new(commands)), Json(req)).await.unwrap();

        assert_eq!(status, resp.status);
    }
}
