use async_openai::Client;
use async_openai::types::chat::{ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs};
use futures::{Stream, StreamExt};
use async_stream::stream;

pub fn chat_stream(model: &str, system: Option<&str>, prompt: &str) -> impl Stream<Item = anyhow::Result<String>> {
    stream! {
        let client = Client::new();
    let mut messages = vec![];

    if let Some(system) = system {
        messages.push(
            ChatCompletionRequestSystemMessageArgs::default()
                .content(system)
                .build()?
                .into()
        );
    }

    messages.push(
        ChatCompletionRequestUserMessageArgs::default()
            .content(prompt)
            .build()?
            .into()
    );

    let request = CreateChatCompletionRequestArgs::default()
        .model(model)
        .messages(messages)
        .max_tokens(1024_u16)
        .stream(true)
        .build()?;
        let mut stream = client.chat().create_stream(request).await?;

        while let Some(response_result) = stream.next().await {
            match response_result {
                Ok(chunk) => {
                    if let Some(choice) = chunk.choices.first()
                        && let Some(new_text) = &choice.delta.content {
                            yield Ok(new_text.clone())
                        }
                }
                Err(err) => yield Err(err.into())
            }
        }
    }
}