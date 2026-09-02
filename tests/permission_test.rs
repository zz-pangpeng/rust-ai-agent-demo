mod common;

use crate::common::client::ModeChatClientStatus;
use crate::common::search_tool::{SearchStatus, get_agent_bind_tool};
use ai_agent::agent::runtime::Agent;
use ai_agent::modals::config::AgentConfig;
use ai_agent::modals::permission_input::{
    PermissionEntry, PermissionInput, PermissionMode, PermissionResult,
};
use ai_agent::modals::tool::ToolView;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;

fn get_client(permission_mode: PermissionMode) -> Agent {
    let mut agent = get_agent_bind_tool(
        ModeChatClientStatus::ToolExecuteSuccess,
        SearchStatus::Success,
        true,
    );
    let mut config = AgentConfig::new();
    config.permission_mode = permission_mode;
    agent.bind_config(config);
    agent
}

struct ScriptPermissionInput {
    result: VecDeque<PermissionResult>,
    logs: Arc<Mutex<Vec<String>>>,
    mode: PermissionMode,
}
impl ScriptPermissionInput {
    fn new(result: Vec<PermissionResult>, logs: Arc<Mutex<Vec<String>>>) -> Self {
        ScriptPermissionInput {
            result: result.into(),
            logs,
            mode: PermissionMode::Interactive,
        }
    }
}
#[async_trait::async_trait]
impl PermissionInput for ScriptPermissionInput {
    async fn ask(&mut self, permission_entry: &PermissionEntry) -> PermissionResult {
        self.logs
            .lock()
            .await
            .push(permission_entry.tool_view.name.clone());
        self.result.pop_front().unwrap_or(PermissionResult::NoInput)
    }

    fn mode(&self) -> PermissionMode {
        self.mode.clone()
    }
}

struct PendingPermissionInput;
#[async_trait::async_trait]
impl PermissionInput for PendingPermissionInput {
    async fn ask(&mut self, _permission_entry: &PermissionEntry) -> PermissionResult {
        std::future::pending().await
    }

    fn mode(&self) -> PermissionMode {
        PermissionMode::Interactive
    }
}

fn tool_view(id: &str, name: &str, args: &str) -> ToolView {
    ToolView {
        tool_call_id: id.to_string(),
        name: name.to_string(),
        arguments: args.to_string(),
        model: "".to_string(),
        config: AgentConfig::default(),
    }
}

#[cfg(test)]
mod tests {
    use crate::{PendingPermissionInput, ScriptPermissionInput, get_client, tool_view};
    use ai_agent::modals::permission_input::{PermissionMode, PermissionResult, input_match};
    use ai_agent::permission::Permission;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[tokio::test]
    async fn auto_approve_by_client_test() {
        let mut agent = get_client(PermissionMode::AutoApprove);
        let result = agent.run("".to_string()).await;

        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().output,
            serde_json::to_string(&PermissionResult::GrantedAlways).unwrap()
        );
    }

    #[tokio::test]
    async fn auto_approve_by_permission_input_test() {
        let logs = Arc::new(Mutex::new(Vec::new()));
        let result = vec![PermissionResult::GrantedAlways];
        let mut permission_input = ScriptPermissionInput::new(result, logs.clone());
        permission_input.mode = PermissionMode::AutoApprove;

        let tool_view = tool_view("1", "search", "1");
        let mut permission = Permission::new(Box::new(permission_input));

        assert_eq!(
            permission.query(&tool_view).await,
            PermissionResult::GrantedAlways
        );
        assert_eq!(logs.lock().await.len(), 0);
    }

    #[tokio::test]
    async fn auto_deny_by_client_test() {
        let mut agent = get_client(PermissionMode::AutoDeny);
        let result = agent.run("".to_string()).await;

        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().output,
            serde_json::to_string(&PermissionResult::Denied).unwrap()
        );
    }

    #[tokio::test]
    async fn auto_deny_by_permission_input_test() {
        let logs = Arc::new(Mutex::new(Vec::new()));
        let result = vec![PermissionResult::Denied];
        let mut permission_input = ScriptPermissionInput::new(result, logs.clone());
        permission_input.mode = PermissionMode::AutoDeny;

        let tool_view = tool_view("1", "search", "1");
        let mut permission = Permission::new(Box::new(permission_input));

        assert_eq!(permission.query(&tool_view).await, PermissionResult::Denied);
        assert_eq!(logs.lock().await.len(), 0);
    }

    #[tokio::test]
    async fn granted_always_test() {
        let logs = Arc::new(Mutex::new(Vec::new()));
        let result = vec![PermissionResult::GrantedAlways];
        let script_permission_input = ScriptPermissionInput::new(result, logs.clone());
        let mut permission = Permission::new(Box::new(script_permission_input));
        let tool_view = tool_view("1", "search", "a");

        assert_eq!(
            permission.query(&tool_view).await,
            PermissionResult::GrantedAlways
        );
        assert_eq!(
            permission.query(&tool_view).await,
            PermissionResult::GrantedAlways
        );
        assert_eq!(logs.lock().await.len(), 1);
        assert_eq!(logs.lock().await.first().unwrap(), "search");
    }

    #[tokio::test]
    async fn granted_once_test() {
        let logs = Arc::new(Mutex::new(Vec::new()));
        let result = vec![PermissionResult::GrantedOnce, PermissionResult::GrantedOnce];
        let script_permission_input = ScriptPermissionInput::new(result, logs.clone());
        let mut permission = Permission::new(Box::new(script_permission_input));
        let tool_view = tool_view("1", "search", "a");

        assert_eq!(
            permission.query(&tool_view).await,
            PermissionResult::GrantedOnce
        );
        assert_eq!(
            permission.query(&tool_view).await,
            PermissionResult::GrantedOnce
        );
        assert_eq!(logs.lock().await.len(), 2);
        assert_eq!(logs.lock().await.first().unwrap(), "search");
        assert_eq!(logs.lock().await.last().unwrap(), "search");
    }

    #[tokio::test]
    async fn deny_test() {
        let logs = Arc::new(Mutex::new(Vec::new()));
        let result = vec![PermissionResult::Denied];
        let script_permission_input = ScriptPermissionInput::new(result, logs.clone());
        let mut permission = Permission::new(Box::new(script_permission_input));
        let tool_view = tool_view("1", "search", "a");

        assert_eq!(permission.query(&tool_view).await, PermissionResult::Denied);
        assert_eq!(permission.query(&tool_view).await, PermissionResult::Denied);
        assert_eq!(logs.lock().await.len(), 1);
        assert_eq!(logs.lock().await.first().unwrap(), "search");
    }

    #[tokio::test]
    async fn tool_always_granted_test() {
        let logs = Arc::new(Mutex::new(Vec::new()));
        let result = vec![PermissionResult::GrantedAlways];
        let script_permission_input = ScriptPermissionInput::new(result, logs.clone());
        let mut permission = Permission::new(Box::new(script_permission_input));
        let tool_view_search_a = tool_view("1", "search", "a");
        let tool_view_search_b = crate::tool_view("1", "search", "b");

        assert_eq!(
            permission.query(&tool_view_search_a).await,
            PermissionResult::GrantedAlways
        );
        assert_eq!(
            permission.query(&tool_view_search_b).await,
            PermissionResult::GrantedAlways
        );
        assert_eq!(logs.lock().await.len(), 1);
        assert_eq!(logs.lock().await.first().unwrap(), "search");
    }

    #[tokio::test]
    async fn timeout_test() {
        let permission_input = PendingPermissionInput {};
        let mut permission = Permission::new(Box::new(permission_input));

        let mut tool_view_search = crate::tool_view("1", "search", "a");
        tool_view_search.config.permission_timeout = 1;

        assert_eq!(
            permission.query(&tool_view_search).await,
            PermissionResult::Timeout
        );
    }

    #[tokio::test]
    async fn input_match_test() {
        assert_eq!(input_match("y"), PermissionResult::GrantedOnce);
        assert_eq!(input_match("Y"), PermissionResult::GrantedOnce);
        assert_eq!(input_match("a"), PermissionResult::GrantedAlways);
        assert_eq!(input_match("n"), PermissionResult::Denied);
        assert_eq!(input_match("x"), PermissionResult::NoInput);
    }
}
