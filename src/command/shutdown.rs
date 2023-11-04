use std::process::Command;

use async_trait::async_trait;
use tracing::info;

use super::{CommandExecutor, CmdArg, CommandResp, Status};

pub const KEY: &str = "shutdown";

pub struct ShutdownCommandExecutor;

#[async_trait]
impl CommandExecutor for ShutdownCommandExecutor {
    async fn execute(&self, _params: Vec<CmdArg>) -> eyre::Result<CommandResp> {
        info!("executing shutdown command");
        
        // TODO: impl args
        let output = if cfg!(target_os = "windows") {
            Command::new("shutdown").arg("-s").output()?
        } else {
            Command::new("shutdown").arg("now").output()?
        };

        info!("shutdown command executed with exit code {}", output.status);
        Ok(CommandResp { status: Status::Success })
    }
}
