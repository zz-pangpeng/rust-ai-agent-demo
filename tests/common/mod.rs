use async_openai::error::OpenAIError;
use async_openai::types::assistants::FunctionCall;
use async_openai::types::chat::{ChatChoice, ChatCompletionMessageToolCall, ChatCompletionMessageToolCalls, ChatCompletionRequestMessage, ChatCompletionRequestToolMessageContent, ChatCompletionRequestUserMessageContent, ChatCompletionResponseMessage, CreateChatCompletionRequest, CreateChatCompletionResponse};
use ai_agent::agent::runtime::Agent;
use ai_agent::modals::chat_client::ChatClient;
use ai_agent::state::TEST_MODEL;

pub fn get_default_response(content: Option<String>) -> CreateChatCompletionResponse {
    CreateChatCompletionResponse {
        id: "".to_string(),
        choices: vec![
            ChatChoice {
                index: 0,
                message: ChatCompletionResponseMessage {
                    content,
                    refusal: None,
                    tool_calls: None,
                    annotations: None,
                    role: Default::default(),
                    function_call: None,
                    audio: None,
                },
                finish_reason: None,
                logprobs: None,
            }
        ],
        created: 0,
        model: "".to_string(),
        service_tier: None,
        system_fingerprint: None,
        object: "".to_string(),
        usage: None,
    }
}

#[derive(PartialEq)]
pub enum ModeChatClientStatus {
    Timeout,
    NotOutput,
    Normal,
    ChoiceEmpty,
    OutMaxSteps,
    ToolNotFound,
    ToolTimeout,
    ToolExecuteFailure,
    ToolExecuteSuccess,
}

struct ModeChatClient {
    status: ModeChatClientStatus,
}

impl ModeChatClient {
    fn new(status: ModeChatClientStatus) -> ModeChatClient {
        ModeChatClient {
            status,
        }
    }
}


#[async_trait::async_trait]
impl ChatClient for ModeChatClient {
    async fn chat_create(&self, req: CreateChatCompletionRequest) -> Result<CreateChatCompletionResponse, OpenAIError> {
        let mut response = get_default_response(None);
        match self.status {
            ModeChatClientStatus::Timeout => {
                let () = std::future::pending().await;
            },
            ModeChatClientStatus::NotOutput => {
            },
            ModeChatClientStatus::ChoiceEmpty => {
               response.choices = Vec::new();
            }
            ModeChatClientStatus::Normal => {
                response.choices.first_mut().unwrap().message = ChatCompletionResponseMessage {
                    content: Some("hello rust agent".to_string()),
                    refusal: None,
                    tool_calls: None,
                    annotations: None,
                    role: Default::default(),
                    function_call: None,
                    audio: None,
                };
            }
            ModeChatClientStatus::OutMaxSteps => {
                response.choices.first_mut().unwrap().message = ChatCompletionResponseMessage {
                    content: None,
                    refusal: None,
                    tool_calls: Some(vec![
                        ChatCompletionMessageToolCalls::Function(
                            ChatCompletionMessageToolCall {
                                id: "123".to_string(),
                                function: FunctionCall {
                                    name: "search".to_string(),
                                    arguments: "".to_string(),
                                },
                            }
                        )
                    ]),
                    annotations: None,
                    role: Default::default(),
                    function_call: None,
                    audio: None,
                };
            },
            ModeChatClientStatus::ToolExecuteFailure |
            ModeChatClientStatus::ToolTimeout |
            ModeChatClientStatus::ToolExecuteSuccess |
            ModeChatClientStatus::ToolNotFound => {
                let last_message = req.messages.last().unwrap();
                match last_message {
                    ChatCompletionRequestMessage::User(data) => {
                        let arguments  = match &data.content {
                            ChatCompletionRequestUserMessageContent::Text(text) => text,
                            _ => &"".to_string()
                        };
                        let tool_name = if self.status == ModeChatClientStatus::ToolNotFound { "web_search" } else { "search" };
                        response.choices.first_mut().unwrap().message = ChatCompletionResponseMessage {
                            content: None,
                            refusal: None,
                            tool_calls: Some(vec![
                                ChatCompletionMessageToolCalls::Function(
                                    ChatCompletionMessageToolCall {
                                        id: "123".to_string(),
                                        function: FunctionCall {
                                            name: tool_name.to_string(),
                                            arguments: arguments.to_string(),
                                        },
                                    }
                                )
                            ]),
                            annotations: None,
                            role: Default::default(),
                            function_call: None,
                            audio: None,
                        };
                    },
                    ChatCompletionRequestMessage::Tool(data) => {
                        let content = match &data.content {
                            ChatCompletionRequestToolMessageContent::Text(text) => text,
                            _ => &"".to_string()
                        };
                        response = get_default_response(Some(content.clone()));
                    }
                    _ => {}
                }
            }
        }
        Ok(response)
    }
}

pub fn get_client(status: ModeChatClientStatus) -> Agent {
    let client = ModeChatClient::new(status);
    let mut agent = Agent::new(TEST_MODEL, Some(""));
    agent.bind_chat_client(Box::new(client));
    agent
}
