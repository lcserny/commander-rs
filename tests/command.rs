#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use axum::{extract::State, Json};
    use commander::command::{execute_cmd, CommandReq, CommandsKind, Status, StubCommand};

    #[tokio::test]
    async fn commands_execute_correctly() {
        let stub_cmd = "stub";
        let status = Status::Success;

        let mut commands = HashMap::new();
        commands.insert(
            stub_cmd.to_owned(),
            CommandsKind::StubCommand(StubCommand {
                status: status.clone(),
            }),
        );
        let req = CommandReq {
            name: stub_cmd.to_owned(),
            params: None,
        };

        let resp = execute_cmd(State(Arc::new(commands)), Json(req))
            .await
            .unwrap();

        assert_eq!(status, resp.status);
    }
}
