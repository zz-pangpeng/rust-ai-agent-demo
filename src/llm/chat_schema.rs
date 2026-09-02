use crate::modals::math_solution::MathSolution;
use async_openai::Client;
use async_openai::types::chat::{
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
    CreateChatCompletionRequestArgs, ResponseFormat,
};
use async_openai::types::responses::ResponseFormatJsonSchema;
use tracing::info;

pub async fn chat_schema(model: &str, system: Option<&str>, prompt: &str) -> anyhow::Result<()> {
    let client = Client::new();

    let mut message = vec![];

    if let Some(system) = system {
        message.push(
            ChatCompletionRequestSystemMessageArgs::default()
                .content(system)
                .build()?
                .into(),
        );
    }

    message.push(
        ChatCompletionRequestUserMessageArgs::default()
            .content(prompt)
            .build()?
            .into(),
    );
    let schema = schemars::schema_for!(MathSolution);
    let schema_json = schema.as_value().clone();
    let response_format = ResponseFormat::JsonSchema {
        json_schema: ResponseFormatJsonSchema {
            description: Some("A math question solution steps".into()),
            name: "math_solution".to_string(),
            schema: schema_json,
            strict: Some(true),
        },
    };

    let request = CreateChatCompletionRequestArgs::default()
        .model(model)
        .max_tokens(1024_u16)
        .messages(message)
        .response_format(response_format)
        .build()?;

    let response = client.chat().create(request).await?;

    let result: MathSolution = response
        .choices
        .into_iter()
        .next()
        .and_then(|c| c.message.content)
        .ok_or_else(|| anyhow::anyhow!("No response choices"))
        .and_then(|c| serde_json::from_str(&c).map_err(Into::into))?;

    info!("{:#?}", result);

    Ok(())
}
