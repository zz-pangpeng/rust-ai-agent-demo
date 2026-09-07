use crate::agent::event::ToolCallStatus;
use crate::modals::tool::ToolView;
use crate::permission::Permission;
use crate::tools::output::{TOOL_EXECUTE_FAILURE, TOOL_EXECUTE_TIMEOUT};
use anyhow::anyhow;
use async_openai::types::chat::{ChatCompletionTool, ChatCompletionTools, FunctionObjectArgs};
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, error, info};

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

        Ok(ChatCompletionTools::Function(ChatCompletionTool {
            function,
        }))
    }

    async fn execute(&mut self, args: &str) -> anyhow::Result<String>;

    async fn execute_with_timeout(
        &mut self,
        args: &str,
        tool_view: &ToolView,
        permission: &Permission,
    ) -> (ToolCallStatus, String) {
        timeout(
            Duration::from_secs(tool_view.config.tool_execute_timeout),
            async {
                if let Some(result) = self.before_execute(&tool_view, permission).await {
                    info!("tool before callback: {:?}", result);
                    result
                } else {
                    match self.execute(args).await {
                        Ok(result) => {
                            debug!("tool execute success: {}", result);
                            let after_callback_result =
                                self.after_execute(&tool_view, result).await;
                            debug!("tool after callback result: {}", after_callback_result);
                            (ToolCallStatus::Success, after_callback_result)
                        }
                        Err(error) => {
                            error!("tool execute error: {}", error);
                            (
                                ToolCallStatus::Failure,
                                format!("{}: {}", TOOL_EXECUTE_FAILURE, error),
                            )
                        }
                    }
                }
            },
        )
        .await
        .unwrap_or_else(|_| (ToolCallStatus::Failure, TOOL_EXECUTE_TIMEOUT.to_string()))
    }

    async fn before_callback(
        &mut self,
        _tool_view: &ToolView,
        _permission: &Permission,
    ) -> Option<(ToolCallStatus, String)> {
        None
    }

    async fn before_execute(
        &mut self,
        tool_view: &ToolView,
        permission: &Permission,
    ) -> Option<(ToolCallStatus, String)> {
        timeout(
            Duration::from_secs(tool_view.config.tool_callback_execute_timeout),
            self.before_callback(tool_view, permission),
        )
        .await
        .unwrap_or_else(|_| {
            let text = "tool before_callback execute timeout";
            info!("{text}");
            Some((ToolCallStatus::Failure, text.to_string()))
        })
    }

    async fn after_callback(&mut self, _tool_view: &ToolView, result: String) -> String {
        result
    }

    async fn after_execute(&mut self, tool_view: &ToolView, result: String) -> String {
        match timeout(
            Duration::from_secs(tool_view.config.tool_callback_execute_timeout),
            self.after_callback(tool_view, result.clone()),
        )
        .await
        {
            Ok(after_callback_result) => after_callback_result,
            Err(_) => {
                info!("{} tool after_callback execute timeout", self.name());
                result
            }
        }
    }
}
