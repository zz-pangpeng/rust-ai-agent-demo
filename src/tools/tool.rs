use std::collections::HashMap;
use std::sync::Arc;
use anyhow::anyhow;
use async_openai::types::chat::{ChatCompletionTool, ChatCompletionTools, FunctionObjectArgs};
use crate::modals::tool::ToolView;
use crate::tools::calculator::CalculatorTool;
use crate::tools::mcp::client::McpClient;
use crate::tools::mcp::tool::McpTool;
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
    
    async fn before_callback(&self, _tool_view: &ToolView) -> Option<String> {
        None
    }
    
    async fn after_callback(&self, _tool_view: &ToolView, result: String) -> String {
        result
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
