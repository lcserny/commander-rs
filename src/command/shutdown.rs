use std::{process::Command, env};

use async_trait::async_trait;
use tracing::info;

use super::{CommandExecutor, CommandResp, Status};

pub const KEY: &str = "shutdown";

pub struct ShutdownCommandExecutor;

#[async_trait]
impl CommandExecutor for ShutdownCommandExecutor {
    async fn execute(&self, mut params: Vec<String>) -> eyre::Result<CommandResp> {
        info!("executing shutdown command");

        handle_params(&mut params);

        let output = Command::new("shutdown").args(&params).output()?;
        info!("shutdown command executed with exit code {}", output.status);

        Ok(CommandResp {
            status: Status::Success,
        })
    }
}

fn handle_params(params: &mut Vec<String>) {
    if params.is_empty() {
        if env::consts::OS == "windows" {
            params.push("-s".to_owned());
        } else if env::consts::OS == "linux" {
            params.push("now".to_owned());
        }    
    }
}
