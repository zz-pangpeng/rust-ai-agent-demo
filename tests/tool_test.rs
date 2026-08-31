use std::collections::HashMap;
use async_openai::types::chat::ChatCompletionTools;
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ai_agent::agent::runtime::Agent;
use ai_agent::tools::output::TOOL_EXECUTE_FAILURE;
use ai_agent::tools::tool::Tool;

mod common;
use common::*;


#[derive(Deserialize, Serialize, JsonSchema)]
struct SearchArguments {
    query: String,
}

#[derive(PartialEq)]
enum SearchStatus {
    Timeout,
    Success,
    Failure
}

struct Search {
    status: SearchStatus,
}

impl Search {
    fn new(status: SearchStatus) -> Search {
        Search { status }
    }
}

#[async_trait::async_trait]
impl Tool for Search {
    fn name(&self) -> &str { "search" }

    fn description(&self) -> &str { "模拟搜索" }

    fn parameters(&self) -> Value {
        serde_json::to_value(schema_for!(SearchArguments)).unwrap()
    }

    async fn execute(&self, _args: &str) -> anyhow::Result<String> {
        match self.status {
            SearchStatus::Timeout => {
                let () = std::future::pending().await;
                Ok("".to_string())
            },
            SearchStatus::Success => {
                Ok( r#"search content"#.to_string())
            },
            SearchStatus::Failure => {
                Err(anyhow::anyhow!(TOOL_EXECUTE_FAILURE))
            }
        }
    }
}

fn get_tool_config(search_status: SearchStatus) -> ( Vec<ChatCompletionTools>, HashMap<String, Box<dyn Tool>>) {
    let mut tool_map: HashMap<String, Box<dyn Tool>> = HashMap::new();
    let mut tool_list = Vec::new();
    let search = Search::new(search_status);
    if let Ok(chat_completion_tool) = search.definition() {
        tool_list.push(chat_completion_tool);
        tool_map.insert(search.name().to_string(), Box::new(search));
    }
    (tool_list, tool_map)
}

fn get_agent_bind_tool(llm_client_status: ModeChatClientStatus, search_status: SearchStatus) -> Agent {
    let mut agent = get_client(llm_client_status);
    let (tool_list, tool_map) = get_tool_config(search_status);
    agent.bind_tool_calls(tool_list, tool_map);
    agent
}

#[cfg(test)]
mod test {

    use ai_agent::agent::output::{OUT_MAX_STEPS, TOOL_NOT_FOUND};
    use ai_agent::modals::config::AgentConfig;
    use ai_agent::tools::output::{TOOL_EXECUTE_FAILURE, TOOL_EXECUTE_TIMEOUT};
    use crate::common::get_client;
    use super::{get_agent_bind_tool, ModeChatClientStatus, SearchStatus};
    #[tokio::test]
    async fn tool_not_found_test() {
        let mut agent = get_client(ModeChatClientStatus::ToolNotFound);
        let result = agent.run("".to_string()).await;

        assert!(result.is_ok());
        assert!(result.unwrap().output.contains(TOOL_NOT_FOUND));
    }
    #[tokio::test]
    async fn tool_execute_success_test() {
        let mut agent = get_agent_bind_tool(ModeChatClientStatus::ToolExecuteSuccess, SearchStatus::Success);

        let result = agent.run("".to_string()).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().output, "search content");
    }

    #[tokio::test]
    async fn out_max_steps_test() {
        let mut agent = get_agent_bind_tool(ModeChatClientStatus::OutMaxSteps, SearchStatus::Success);

        let result = agent.run("".to_string()).await;

        assert!(result.is_ok());
        assert!(result.unwrap().output.contains(OUT_MAX_STEPS));
    }

    #[tokio::test]
    async fn tool_execute_timeout_test() {
        let mut agent = get_agent_bind_tool(ModeChatClientStatus::ToolTimeout, SearchStatus::Timeout);

        let mut config = AgentConfig::new();
        config.tool_execute_timeout = 1;

        agent.bind_config(config);

        let result = agent.run("".to_string()).await;

        assert!(result.is_ok());
        assert!(result.unwrap().output.contains(TOOL_EXECUTE_TIMEOUT));
    }

    #[tokio::test]
    async fn tool_execute_failure_test() {
        let mut agent = get_agent_bind_tool(ModeChatClientStatus::ToolTimeout, SearchStatus::Failure);

        let mut config = AgentConfig::new();
        config.tool_execute_timeout = 1;

        agent.bind_config(config);

        let result = agent.run("".to_string()).await;

        assert!(result.is_ok());
        assert!(result.unwrap().output.contains(TOOL_EXECUTE_FAILURE));
    }
}