use crate::common::client::{ModeChatClientStatus, get_client};
use ai_agent::agent::event::ToolCallStatus;
use ai_agent::agent::runtime::Agent;
use ai_agent::modals::tool::ToolView;
use ai_agent::permission::Permission;
use ai_agent::tools::output::TOOL_EXECUTE_FAILURE;
use ai_agent::tools::tool::Tool;
use async_openai::types::chat::ChatCompletionTools;
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Deserialize, Serialize, JsonSchema)]
struct SearchArguments {
    query: String,
}

#[derive(PartialEq)]
pub enum SearchStatus {
    Timeout,
    Success,
    Failure,
}

struct Search {
    status: SearchStatus,
    need_permission: bool,
}

impl Search {
    fn new(status: SearchStatus, need_permission: bool) -> Search {
        Search {
            status,
            need_permission,
        }
    }
}

#[async_trait::async_trait]
impl Tool for Search {
    fn name(&self) -> &str {
        "search"
    }

    fn description(&self) -> &str {
        "模拟搜索"
    }

    fn parameters(&self) -> Value {
        serde_json::to_value(schema_for!(SearchArguments)).unwrap()
    }

    async fn execute(&mut self, _args: &str) -> anyhow::Result<String> {
        match self.status {
            SearchStatus::Timeout => {
                let () = std::future::pending().await;
                Ok("".to_string())
            }
            SearchStatus::Success => Ok(r#"search content"#.to_string()),
            SearchStatus::Failure => Err(anyhow::anyhow!(TOOL_EXECUTE_FAILURE)),
        }
    }

    async fn before_callback(
        &mut self,
        tool_view: &ToolView,
        permission: &mut Permission,
    ) -> Option<(ToolCallStatus, String)> {
        if !self.need_permission {
            return None;
        }
        let result = permission.query(tool_view).await;
        println!("{:?}", result);
        Some((
            ToolCallStatus::Success,
            serde_json::to_string(&result).unwrap(),
        ))
    }
}

fn get_tool_config(
    list: Vec<Box<dyn Tool>>,
) -> (Vec<ChatCompletionTools>, HashMap<String, Box<dyn Tool>>) {
    let mut tool_map: HashMap<String, Box<dyn Tool>> = HashMap::new();
    let mut tool_list = Vec::new();
    for tool in list {
        if let Ok(chat_completion_tool) = tool.definition() {
            tool_list.push(chat_completion_tool);
            tool_map.insert(tool.name().to_string(), tool);
        }
    }
    (tool_list, tool_map)
}

pub fn get_agent_bind_tool(
    llm_client_status: ModeChatClientStatus,
    search_status: SearchStatus,
    need_permission: bool,
) -> Agent {
    let mut agent = get_client(llm_client_status);
    let (tool_list, tool_map) =
        get_tool_config(vec![Box::new(Search::new(search_status, need_permission))]);
    agent.bind_tool_calls(tool_list, tool_map);
    agent
}
