use crate::agent::context::Context;
use crate::agent::event::{ContentItem, Event, ToolCallStatus};
use crate::agent::output::{NOT_OUTPUT, OUT_MAX_STEPS, TIMEOUT, TOOL_NOT_FOUND};
use crate::modals::chat_client::{ChatClient, RealChatClient};
use crate::modals::config::AgentConfig;
use crate::modals::permission_input::{
    DeniedPermissionInput, GrantedAlwaysPermissionInput, PermissionInput, PermissionMode,
    StdinPermissionInput,
};
use crate::modals::tool::ToolView;
use crate::permission::Permission;
use crate::tools::tool::Tool;
use anyhow::anyhow;
use async_openai::types::chat::{
    ChatCompletionMessageToolCall, ChatCompletionMessageToolCalls,
    ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestToolMessageArgs,
    ChatCompletionRequestUserMessageArgs, ChatCompletionTools, CreateChatCompletionRequestArgs,
    FunctionCall,
};
use backon::{ExponentialBuilder, Retryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{Instrument, Level, debug, error, span};

pub struct Agent {
    model: String,
    system_instruction: Option<String>,
    max_steps: usize,
    tool_box_map: HashMap<String, Box<dyn Tool>>,
    tool_box: Vec<ChatCompletionTools>,
    chat_client: Box<dyn ChatClient>,
    context: Context,
    config: AgentConfig,
    permission: Permission,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AgentResult {
    pub input: String,
    pub output: String,
    pub context: Context,
}
fn get_permission_input(permission_mode: &PermissionMode) -> Box<dyn PermissionInput> {
    match permission_mode {
        PermissionMode::AutoDeny => Box::new(DeniedPermissionInput {}),
        PermissionMode::Interactive => Box::new(StdinPermissionInput::new()),
        PermissionMode::AutoApprove => Box::new(GrantedAlwaysPermissionInput {}),
    }
}
impl Agent {
    pub fn new(model: impl Into<String>, system_instruction: Option<impl Into<String>>) -> Self {
        let config = AgentConfig::new();
        let permission_input = get_permission_input(&config.permission_mode);
        Self {
            model: model.into(),
            system_instruction: system_instruction.map(Into::into),
            max_steps: config.agent_max_steps,
            tool_box: Vec::new(),
            tool_box_map: HashMap::new(),
            chat_client: Box::new(RealChatClient::new()),
            context: Context::new(),
            config: config.clone(),
            permission: Permission::new(permission_input),
        }
    }

    pub fn bind_tool_calls(
        &mut self,
        tool_box: Vec<ChatCompletionTools>,
        tool_box_map: HashMap<String, Box<dyn Tool>>,
    ) -> &mut Self {
        self.tool_box = tool_box;
        self.tool_box_map = tool_box_map;
        self
    }

    pub fn set_max_steps(&mut self, max_steps: usize) -> &mut Self {
        self.max_steps = max_steps;
        self
    }

    pub fn bind_chat_client(&mut self, chat_client: Box<dyn ChatClient>) -> &mut Self {
        self.chat_client = chat_client;
        self
    }

    pub fn bind_config(&mut self, agent_config: AgentConfig) -> &mut Self {
        self.max_steps = agent_config.agent_max_steps.clone();
        self.permission = Permission::new(get_permission_input(&agent_config.permission_mode));
        self.config = agent_config;
        self
    }

    pub fn bind_permission_input(
        &mut self,
        permission_input: Box<dyn PermissionInput>,
    ) -> &mut Self {
        self.permission = Permission::new(permission_input);
        self
    }

    pub fn build_message(&self) -> anyhow::Result<Vec<ChatCompletionRequestMessage>> {
        let mut messages = Vec::new();

        if let Some(system_instruction) = &self.system_instruction {
            messages.push(
                ChatCompletionRequestSystemMessageArgs::default()
                    .content(system_instruction.as_str())
                    .build()?
                    .into(),
            );
        }

        for event in &self.context.event {
            for content_item in &event.content {
                match content_item {
                    ContentItem::Message { role, content } => {
                        let message = if role == "user" {
                            ChatCompletionRequestUserMessageArgs::default()
                                .content(content.as_str())
                                .build()?
                                .into()
                        } else {
                            ChatCompletionRequestAssistantMessageArgs::default()
                                .content(content.as_str())
                                .build()?
                                .into()
                        };
                        messages.push(message);
                    }
                    ContentItem::ToolCall {
                        tool_call_id,
                        name,
                        arguments,
                    } => {
                        let tool_call = ChatCompletionMessageToolCalls::Function(
                            ChatCompletionMessageToolCall {
                                id: tool_call_id.clone(),
                                function: FunctionCall {
                                    name: name.clone(),
                                    arguments: arguments.to_string(),
                                },
                            },
                        );
                        if let Some(ChatCompletionRequestMessage::Assistant(data)) =
                            messages.last_mut()
                        {
                            data.tool_calls.get_or_insert_with(Vec::new).push(tool_call);
                        } else {
                            messages.push(
                                ChatCompletionRequestAssistantMessageArgs::default()
                                    .tool_calls(vec![tool_call])
                                    .build()?
                                    .into(),
                            );
                        }
                    }

                    ContentItem::ToolCallResult {
                        tool_call_id,
                        content,
                        ..
                    } => {
                        messages.push(
                            ChatCompletionRequestToolMessageArgs::default()
                                .content(content.as_str())
                                .tool_call_id(tool_call_id.clone())
                                .build()?
                                .into(),
                        );
                    }
                }
            }
        }
        Ok(messages)
    }

    pub async fn run(&mut self, prompt: String) -> anyhow::Result<AgentResult> {
        self.context = Context::new();
        let event = Event::new(
            self.context.execution_id.clone(),
            "user",
            vec![ContentItem::Message {
                role: "user".to_string(),
                content: prompt.clone(),
            }],
        );
        self.context.add_event(event);
        let mut result = AgentResult {
            input: prompt.clone(),
            output: "".to_string(),
            context: self.context.clone(),
        };

        self.permission.reset();

        loop {
            if self.context.current_step >= self.max_steps {
                result.output = OUT_MAX_STEPS.to_string();
                result.context = self.context.clone();
                return Ok(result);
            }
            let messages = self.build_message()?;
            let request = CreateChatCompletionRequestArgs::default()
                .messages(messages)
                .model(&self.model)
                .tools(self.tool_box.clone())
                .build()?;

            let response = (|| async {
                match timeout(
                    Duration::from_secs(self.config.agent_execute_timeout),
                    self.chat_client.chat_create(request.clone()),
                )
                .await
                {
                    Ok(result) => result.map_err(Into::into),
                    Err(_) => Err(anyhow!(TIMEOUT)),
                }
            })
            .retry(ExponentialBuilder::new().with_max_times(self.config.agent_execute_retry_count))
            .await?;

            let message = response
                .choices
                .into_iter()
                .next()
                .ok_or_else(|| anyhow!(NOT_OUTPUT))?
                .message;

            debug!("message: {:?}", message);

            if let Some(tool_calls) = message.tool_calls {
                self.add_tool_call(&tool_calls);
                self.tool_execute(&tool_calls).await;
                self.context.increment_step();
            } else {
                let output = message.content.ok_or_else(|| anyhow!(NOT_OUTPUT))?;
                result.output = output;
                result.context = self.context.clone();
                return Ok(result);
            }
        }
    }

    pub fn add_tool_call(&mut self, tool_calls: &Vec<ChatCompletionMessageToolCalls>) {
        let mut content = vec![];
        for tool_call in tool_calls {
            if let ChatCompletionMessageToolCalls::Function(tool) = tool_call {
                // 参数解析失败也保留 ToolCall,保证 tool_call_id 在 assistant 消息里存在;
                // 具体结果/错误统一由 tool_execute 产生唯一一条 ToolCallResult
                let arguments = serde_json::from_str::<Value>(&tool.function.arguments)
                    .unwrap_or_else(|_| Value::String(tool.function.arguments.clone()));
                content.push(ContentItem::ToolCall {
                    tool_call_id: tool.id.clone(),
                    name: tool.function.name.clone(),
                    arguments,
                });
            }
        }
        let event = Event::new(self.context.execution_id.clone(), "assistant", content);
        self.context.add_event(event);
    }

    pub async fn tool_execute(&mut self, tool_calls: &Vec<ChatCompletionMessageToolCalls>) {
        for tool_call in tool_calls {
            if let ChatCompletionMessageToolCalls::Function(tool) = tool_call {
                let name = tool.function.name.clone();
                let args = tool.function.arguments.clone();
                let span = span!(Level::INFO, "tool call", name = name, id = tool.id.clone());
                let tool_view = ToolView {
                    tool_call_id: tool.id.clone(),
                    name: name.clone(),
                    arguments: args.clone(),
                    model: self.model.clone(),
                    config: self.config.clone(),
                };

                let (status, content) = async {
                    match self.tool_box_map.get_mut(&name) {
                        Some(tool) => {
                            tool.execute_with_timeout(
                                args.as_str(),
                                &tool_view,
                                &mut self.permission,
                            )
                            .await
                        }
                        None => {
                            error!("tool not found");
                            (
                                ToolCallStatus::Failure,
                                format!("{}: {}", TOOL_NOT_FOUND, name.clone()),
                            )
                        }
                    }
                }
                .instrument(span)
                .await;

                let event = Event::new(
                    self.context.execution_id.clone(),
                    "tool_calls",
                    vec![ContentItem::ToolCallResult {
                        tool_call_id: tool.id.clone(),
                        name,
                        status,
                        content,
                    }],
                );
                self.context.add_event(event);
            }
        }
    }
}
