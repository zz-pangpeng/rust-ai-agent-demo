use crate::agent::context::Context;
use crate::agent::event::{ContentItem, Event, ToolCallStatus};
use crate::agent::output::{NOT_OUTPUT, OUT_MAX_STEPS, TIMEOUT, TOOL_NOT_FOUND};
use crate::modals::chat_client::{ChatClient, RealChatClient};
use crate::modals::config::SystemConfig;
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
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tracing::{Instrument, Level, debug, error, span, warn};
use crate::state::DEEPSEEK_V4_FLASH;
use crate::token::manager::TokenManager;
use crate::vector::compress::compress;

pub struct Agent {
    model: String,
    system_instruction: Option<String>,
    user_instruction: String,
    max_steps: usize,
    tool_box_map: HashMap<String, Arc<Mutex<Box<dyn Tool>>>>,
    tool_box: Vec<ChatCompletionTools>,
    chat_client: Box<dyn ChatClient>,
    context: Arc<Mutex<Context>>,
    config: SystemConfig,
    permission: Permission,
    token_manager: TokenManager
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

fn get_max_tokens(model: &str) -> u64 {
    match model {
        DEEPSEEK_V4_FLASH => 128 * 1024,
        _ => 64 * 1024
    }
}
impl Agent {
    pub fn new(model: impl Into<String>, system_instruction: Option<impl Into<String>>) -> Self {
        let config = SystemConfig::new();
        let permission_input = get_permission_input(&config.permission.mode);
        let model = model.into();
        let max_tokens = get_max_tokens(model.as_str());
        let reserve_ratio = 1.0 - config.compression.trigger_threshold_ratio;
        let reserve_budget= max_tokens  * reserve_ratio as u64;
        Self {
            model,
            system_instruction: system_instruction.map(Into::into),
            user_instruction: "".to_string(),
            max_steps: config.agent.max_rounds,
            tool_box: Vec::new(),
            tool_box_map: HashMap::new(),
            chat_client: Box::new(RealChatClient::new()),
            context: Arc::new(Mutex::new(Context::new())),
            config: config.clone(),
            permission: Permission::new(permission_input),
            token_manager: TokenManager::new(reserve_budget, max_tokens)
        }
    }

    pub fn bind_tool_calls(&mut self, tool_list: Vec<Box<dyn Tool>>) -> &mut Self {
        for tool in tool_list {
            if let Ok(chat_tool) = tool.definition() {
                self.tool_box.push(chat_tool);
                self.tool_box_map
                    .insert(tool.name().to_string(), Arc::new(Mutex::new(tool)));
            }
        }
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

    pub fn bind_config(&mut self, system_config: SystemConfig) -> &mut Self {
        self.max_steps = system_config.agent.max_rounds.clone();
        self.permission = Permission::new(get_permission_input(&system_config.permission.mode));
        self.config = system_config;
        self
    }

    pub fn bind_permission_input(
        &mut self,
        permission_input: Box<dyn PermissionInput>,
    ) -> &mut Self {
        self.permission = Permission::new(permission_input);
        self
    }

    pub async fn build_message(&self) -> anyhow::Result<Vec<ChatCompletionRequestMessage>> {
        let mut messages = Vec::new();

        if let Some(system_instruction) = &self.system_instruction {
            messages.push(
                ChatCompletionRequestSystemMessageArgs::default()
                    .content(system_instruction.as_str())
                    .build()?
                    .into(),
            );
        }
        let context = self.context.lock().await;
        for event in &context.event {
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
        self.permission.reset().await;
        self.user_instruction = prompt.clone();
        let mut context = Context::new();
        let event = Event::new(
            context.execution_id.clone(),
            "user",
            vec![ContentItem::Message {
                role: "user".to_string(),
                content: prompt.clone(),
            }],
        );
        context.add_event(event);
        let mut result = AgentResult {
            input: prompt.clone(),
            output: "".to_string(),
            context: context.clone(),
        };
        self.context = Arc::new(Mutex::new(context));
        loop {
            {
                let context = self.context.lock().await;
                if context.current_step >= self.max_steps {
                    result.output = OUT_MAX_STEPS.to_string();
                    result.context = context.clone();
                    return Ok(result);
                }
            }
            let messages = self.build_message().await?;
            let request = CreateChatCompletionRequestArgs::default()
                .messages(messages)
                .model(&self.model)
                .tools(self.tool_box.clone())
                .build()?;

            let response = (|| async {
                match timeout(
                    Duration::from_secs(self.config.agent.execute_timeout),
                    self.chat_client.chat_create(request.clone()),
                )
                .await
                {
                    Ok(result) => result.map_err(Into::into),
                    Err(_) => Err(anyhow!(TIMEOUT)),
                }
            })
            .retry(ExponentialBuilder::new().with_max_times(self.config.agent.execute_retry_count))
            .await?;

            let message = response
                .choices
                .into_iter()
                .next()
                .ok_or_else(|| anyhow!(NOT_OUTPUT))?
                .message;

            debug!("message: {:?}", message);

            if let Some(tool_calls) = message.tool_calls {
                let add_tool_call_result = self.add_tool_call(&tool_calls);
                let tool_execute_result = self.tool_execute(&tool_calls).await;

                let mut context = self.context.lock().await;
                let id = context.execution_id.clone();
                context.add_event(Event::new(
                    id.clone(),
                    add_tool_call_result.0,
                    add_tool_call_result.1,
                ));
                tool_execute_result
                    .into_iter()
                    .for_each(|(author, content)| {
                        if !content.is_empty() {
                            context.add_event(Event::new(id.clone(), author, content));
                        }
                    });
                context.increment_step();
            } else {
                let context = self.context.lock().await;
                let output = message.content.ok_or_else(|| anyhow!(NOT_OUTPUT))?;
                result.output = output;
                result.context = context.clone();
                return Ok(result);
            }
        }
    }

    pub fn add_tool_call(
        &self,
        tool_calls: &Vec<ChatCompletionMessageToolCalls>,
    ) -> (&str, Vec<ContentItem>) {
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
        ("assistant", content)
    }

    pub async fn tool_execute(
        &self,
        tool_calls: &Vec<ChatCompletionMessageToolCalls>,
    ) -> Vec<(&str, Vec<ContentItem>)> {
        let futures = tool_calls
            .iter()
            .map(|call| async move { 
                self.tool_item_execute(call).await 
            });
        join_all(futures).await
    }
    async fn tool_item_execute(
        &self,
        tool_call: &ChatCompletionMessageToolCalls,
    ) -> (&str, Vec<ContentItem>) {
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
            let (status, mut content) = async {
                match self.tool_box_map.get(&name) {
                    Some(tool) => {
                        tool.lock()
                            .await
                            .execute_with_timeout(args.as_str(), &tool_view, &self.permission)
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

            let result_token = self.token_manager.calculate_token(&content);

            let mut query = String::new();
            if status == ToolCallStatus::Success {
                if let Some(tool) = self.tool_box_map.get(&name) {
                    query = tool.lock().await.compress_query(&args).unwrap_or_default();
                }
            }
            
            if !query.is_empty() && result_token > self.config.tool.result_max_token {
                match timeout(
                    Duration::from_secs(self.config.vector.execute_timeout),
                    async {
                        compress(&query, &content, &self.config.vector).await
                    }
                ).await {
                    Ok(result) => {
                        debug!("before compress token is {}", result_token);
                        let new_result_token = self.token_manager.calculate_token(&result);
                        debug!("after compress token is {}", new_result_token);
                        if new_result_token < result_token {
                            content = result;
                        }
                    },
                    Err(_) => {
                        warn!("向量压缩超时，跳过压缩");
                    }
                }
            }
            
            return (
                "tool_calls",
                vec![ContentItem::ToolCallResult {
                    tool_call_id: tool.id.clone(),
                    name,
                    status,
                    content,
                }],
            );
        }
        ("tool_calls", vec![])
    }
}
