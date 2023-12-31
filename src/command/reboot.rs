use std::{process, env};

use async_trait::async_trait;
use tracing::info;

use super::{Command, CommandResp, Status};

pub const KEY: &str = "reboot";

pub struct RebootCommand;

#[async_trait]
impl Command for RebootCommand {
    async fn execute(&self, mut _params: Vec<String>) -> eyre::Result<CommandResp> {
        info!("executing reboot command");

        if env::consts::OS == "windows" {
            info!("reboot command not available for Windows OS");
            return Ok(CommandResp { status: Status::NotFound });
        }

        let output = process::Command::new("reboot").output()?;
        info!("reboot command executed with exit code {}", output.status);

        Ok(CommandResp { status: Status::Success, })
    }
}