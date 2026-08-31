use async_openai::{Client};
use async_openai::config::OpenAIConfig;
use async_openai::error::OpenAIError;
use async_openai::types::chat::{CreateChatCompletionRequest, CreateChatCompletionResponse};

#[async_trait::async_trait]
pub trait ChatClient: Send + Sync {
    
    async fn chat_create(&self, req: CreateChatCompletionRequest) -> Result<CreateChatCompletionResponse, OpenAIError>;
}

pub struct RealChatClient {
    inner: Client<OpenAIConfig>
}

impl RealChatClient {
    pub fn new() -> Self {
        RealChatClient {
            inner: Client::new()
        }
    }
}
#[async_trait::async_trait]
impl ChatClient for RealChatClient {
    async fn chat_create(&self, req: CreateChatCompletionRequest) -> Result<CreateChatCompletionResponse, OpenAIError> {
        self.inner.chat().create(req).await
    }
}

