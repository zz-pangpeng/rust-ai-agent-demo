use anyhow::anyhow;
use async_openai::{Client};
use async_openai::types::chat::{ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs};
use tracing::info;

pub async fn chat_message (model: &str, system: Option<&str>, prompt: &str) -> anyhow::Result<String> {
    let client = Client::new();

    let mut message = vec![];
    if let Some(system) = system {
        message.push(
            ChatCompletionRequestSystemMessageArgs::default()
                .content(system)
                .build()?
                .into()
        );
    }

    message.push(
        ChatCompletionRequestUserMessageArgs::default()
            .content(prompt)
            .build()?
            .into()
    );

    let request = CreateChatCompletionRequestArgs::default()
        .model(model)
        .messages(message)
        .max_tokens(1024_u16)
        .build()?;
    let response = client.chat().create(request).await?;
    
    let response = response.choices.into_iter().next().and_then(|choices| choices.message.content).ok_or_else(|| anyhow!("no message"))?;
    info!("{:?}", response);

    Ok(response)
}