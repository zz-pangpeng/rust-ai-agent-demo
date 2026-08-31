use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use anyhow::anyhow;
use async_openai::types::chat::{ChatCompletionTool, ChatCompletionTools, FunctionObjectArgs};
use tokio::time::timeout;
use tracing::{debug, error, info};
use crate::agent::event::ToolCallStatus;
use crate::modals::tool::ToolView;
use crate::permission::Permission;
use crate::tools::calculator::CalculatorTool;
use crate::tools::mcp::client::McpClient;
use crate::tools::mcp::tool::McpTool;
use crate::tools::output::{TOOL_EXECUTE_FAILURE, TOOL_EXECUTE_TIMEOUT};
use crate::tools::web_search::WebSearch;

#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> serde_json::Value;
    fn definition(&self) -> anyhow::Result<ChatCompletionTools> {
        let function = FunctionObjectArgs::default()
            .name(self.name())
            .description(self.description())
            .parameters(self.parameters())
            .build()
            .map_err(|e| anyhow!("{} tool create failed: {}", self.name(), e))?;

        Ok( ChatCompletionTools::Function(ChatCompletionTool {
            function
        }))
    }

    async fn execute(&self, args: &str) -> anyhow::Result<String>;

    async fn execute_with_timeout(&self, args: &str, tool_view: &ToolView, permission: &mut Permission) -> (ToolCallStatus, String) {
        match timeout(Duration::from_secs(tool_view.config.tool_execute_timeout), async {
            if let Some(result) = self.before_execute(&tool_view, permission).await {
                info!("tool before callback: {:?}", result);
                result
            } else {
                match self.execute(args).await {
                    Ok(result) => {
                        debug!("tool execute success: {}", result);
                        let after_callback_result = self.after_execute(&tool_view, result).await;
                        debug!("tool after callback result: {}", after_callback_result);
                        (ToolCallStatus::Success, after_callback_result)
                    },
                    Err(error) => {
                        error!("tool execute error: {}", error);
                        (ToolCallStatus::Failure, format!("{}: {}", TOOL_EXECUTE_FAILURE, error))
                    },
                }
            }
        }).await {
            Ok(result) => {
                result
            },
            Err(_) => {
                (ToolCallStatus::Failure, TOOL_EXECUTE_TIMEOUT.to_string())
            }
        }
    }
    
    async fn before_callback(&self, _tool_view: &ToolView,  _permission: &mut Permission) -> Option<(ToolCallStatus, String)> {
        None
    }

    async fn before_execute(&self, tool_view: &ToolView, permission: &mut Permission) -> Option<(ToolCallStatus, String)> {
        match timeout(Duration::from_secs(tool_view.config.tool_callback_execute_timeout), self.before_callback(tool_view, permission)).await {
            Ok(result) => {
                result
            },
            Err(_) => {
                let text = "tool before_callback execute timeout";
                info!("{text}");
                Some((ToolCallStatus::Failure, text.to_string()))
            }
        }
    }
    
    async fn after_callback(&self, _tool_view: &ToolView, result: String) -> String {
        result
    }


    async fn after_execute(&self,tool_view: &ToolView, result: String) -> String {
        match timeout(Duration::from_secs(tool_view.config.tool_callback_execute_timeout), self.after_callback(tool_view, result.clone())).await {
            Ok(after_callback_result) => {
                after_callback_result
            },
            Err(_) => {
                info!("{} tool after_callback execute timeout", self.name());
                result
            }
        }
    }

}

pub async fn get_tools() -> anyhow::Result<(Vec<ChatCompletionTools>, HashMap<String, Box<dyn Tool>>)> {
    let mut tools: Vec<Box<dyn Tool>> = vec![
        Box::new(CalculatorTool),
        Box::new(WebSearch)
    ];

    let mcp_client = Arc::new(McpClient::connect().await?);
    for tool in mcp_client.list_tools().await? {
        tools.push(
            Box::new(McpTool::new(mcp_client.clone(), tool))
        );
    }

    let mut chat_completion_tools = vec![];
    let mut tools_map = HashMap::new();
    for tool in tools {
        if let Ok(chat_completion_tool) = tool.definition() {
            chat_completion_tools.push(chat_completion_tool);
            tools_map.insert(tool.name().to_string(), *Box::new(tool));
        }
    }

    Ok((chat_completion_tools, tools_map))
}