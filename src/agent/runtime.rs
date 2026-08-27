use std::collections::HashMap;
use std::time::Duration;
use tokio::time::timeout;
use anyhow::anyhow;
use async_openai::types::chat::{ChatCompletionMessageToolCall, ChatCompletionMessageToolCalls, ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestToolMessageArgs, ChatCompletionRequestUserMessageArgs, ChatCompletionTools, CreateChatCompletionRequestArgs, FunctionCall};
use async_openai::Client;
use backon::{ExponentialBuilder, Retryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{error, span, Instrument, Level};
use crate::agent::context::Context;
use crate::agent::event::{ContentItem, Event, ToolCallStatus};
use crate::modals::tool::ToolView;
use crate::permission::Permission;
use crate::tools::tool::{Tool};

pub struct Agent {
    model: String,
    system_instruction:  Option<String>,
    max_steps: usize,
    tool_box_map: HashMap<String, Box<dyn Tool>>,
    tool_box: Vec<ChatCompletionTools>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AgentResult {
    pub input: String,
    pub output: String,
    pub context: Context
}

impl Agent {
    pub fn new(model: impl Into<String>, system_instruction: Option<impl Into<String>>) -> Self {

        Self {
            model: model.into(),
            system_instruction:  system_instruction.map(Into::into),
            max_steps: 10,
            tool_box: Vec::new(),
            tool_box_map: HashMap::new(),
        }
    }

    pub fn bind_tool_calls(&mut self, tool_box: Vec<ChatCompletionTools>, tool_box_map: HashMap<String, Box<dyn Tool>>) -> &mut Self{
        self.tool_box = tool_box;
        self.tool_box_map = tool_box_map;
        self
    }

    pub fn set_max_steps(&mut self, max_steps: usize) -> &mut Self {
        self.max_steps = max_steps;
        self
    }

    pub fn build_message(&self, context: &Context) -> anyhow::Result<Vec<ChatCompletionRequestMessage>> {
        let mut messages = Vec::new();

        if let Some(system_instruction) = &self.system_instruction {
            messages.push(
                ChatCompletionRequestSystemMessageArgs::default()
                    .content(system_instruction.as_str())
                    .build()?
                    .into()
            );
        }

        for event in &context.event {
            for content_item in &event.content {
                match content_item {
                    ContentItem::Message { role, content} => {
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
                    ContentItem::ToolCall { tool_call_id, name, arguments } => {
                        let tool_call = ChatCompletionMessageToolCalls::Function(
                            ChatCompletionMessageToolCall {
                                id: tool_call_id.clone(),
                                function: FunctionCall {
                                    name: name.clone(),
                                    arguments: arguments.to_string(),
                                },
                            },
                        );
                        if let Some(ChatCompletionRequestMessage::Assistant(data)) = messages.last_mut() {
                            data.tool_calls.get_or_insert_with(Vec::new).push(tool_call);
                        } else {
                            messages.push(
                                ChatCompletionRequestAssistantMessageArgs::default()
                                    .tool_calls(vec![tool_call])
                                    .build()?
                                    .into()
                            );
                        }
                    }

                ContentItem::ToolCallResult { tool_call_id, content, .. } => {
                    messages.push(
                        ChatCompletionRequestToolMessageArgs::default()
                            .content(content.as_str())
                            .tool_call_id(tool_call_id.clone())
                            .build()?.into()
                    );
                }
                }
            }
        }


        Ok(messages)
    }

    pub async fn run(&self, prompt: String) -> anyhow::Result<AgentResult> {
        let client = Client::new();
        let mut context = Context::new();
        let mut permission = Permission::new(Duration::from_secs(context.tool_config.permission_timeout));
        let event = Event::new(
            context.execution_id.clone(),
            "user",
            vec![
                ContentItem::Message {
                    role: "user".to_string(),
                    content: prompt.clone()
                }
            ]
        );
        context.add_event(event);
        let mut result = AgentResult {
            input: prompt.clone(),
            output: "".to_string(),
            context: context.clone()
        };
        loop {
            if context.current_step >= self.max_steps {
                result.output = "超过最大执行步数".to_string();
                result.context = context.clone();
                return Ok(result);
            }
            let messages = self.build_message(&context)?;
            let request = CreateChatCompletionRequestArgs::default()
                .messages(messages)
                .model(&self.model)
                .tools(self.tool_box.clone())
                .build()?;

            let response = (|| async {
                match timeout(
                    Duration::from_secs(context.tool_config.model_execute_timeout),
                    client.chat().create(request.clone()),
                )
                .await
                {
                    Ok(result) => result.map_err(Into::into),
                    Err(_) => Err(anyhow!(
                        "model request timed out after {}s",
                        context.tool_config.model_execute_timeout
                    )),
                }
            }).retry(ExponentialBuilder::new().with_max_times(context.tool_config.execute_retry_count)).await?;

            let message = response.choices.into_iter()
                .next().ok_or_else(|| anyhow!("no message choices"))?.message;

            if let Some(tool_calls) = message.tool_calls {
                self.add_tool_call(&tool_calls, &mut context);
                self.tool_execute(&tool_calls, &mut context, &mut permission).await;
                context.increment_step();
            } else {
                let output = message.content.ok_or_else(|| anyhow!("no content"))?;
                result.output = output;
                result.context = context.clone();
                return Ok(result);
            }
        }

    }

    pub fn add_tool_call(&self, tool_calls: &Vec<ChatCompletionMessageToolCalls>, context: &mut Context) {
        let mut content = vec![];
        for tool_call in tool_calls {
            if let ChatCompletionMessageToolCalls::Function(tool) = tool_call {
                // 参数解析失败也保留 ToolCall,保证 tool_call_id 在 assistant 消息里存在;
                // 具体结果/错误统一由 tool_execute 产生唯一一条 ToolCallResult
                let arguments = serde_json::from_str::<Value>(&tool.function.arguments)
                    .unwrap_or_else(|_| Value::String(tool.function.arguments.clone()));
                content.push(
                    ContentItem::ToolCall {
                        tool_call_id: tool.id.clone(),
                        name: tool.function.name.clone(),
                        arguments
                    }
                );
            }
        }
        let event = Event::new(
            context.execution_id.clone(),
            "assistant",
            content,
        );
        context.add_event(event);
    }

    pub async fn tool_execute(&self, tool_calls: &Vec<ChatCompletionMessageToolCalls>, context: &mut Context, permission: &mut Permission) {
        for tool_call in tool_calls {
            if let ChatCompletionMessageToolCalls::Function(tool) = tool_call {
                let name = tool.function.name.clone();
                let args = tool.function.arguments.clone();
                let span = span!(Level::INFO, "tool call", name = name, id = tool.id.clone());
                let tool_view = ToolView {
                    tool_call_id: tool.id.clone(),
                    name: name.clone(),
                    arguments: args.clone(),
                    model: self.model.clone()
                };

                let (status, content) = async {
                    match self.tool_box_map.get(&name) {
                        Some(tool) => {
                            tool.execute_with_timeout(args.as_str(), &tool_view, context, permission).await
                        },
                        None => {
                            error!("tool not found");
                            (ToolCallStatus::Failure, format!("找不到工具{}", name.clone()))
                        }
                    }
                }.instrument(span).await;

                let event = Event::new(
                    context.execution_id.clone(),
                    "tool_calls",
                    vec![
                        ContentItem::ToolCallResult {
                            tool_call_id: tool.id.clone(),
                            name,
                            status,
                            content,
                        }
                    ],
                );
                context.add_event(event);
            }
        }
    }

}
