use async_openai::Client;
use async_openai::types::chat::{ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs};
use async_openai::types::chat::ResponseFormat::JsonObject;
use tracing::info;
use crate::state::DEEPSEEK_V4_FLASH;
use crate::modals::math_solution::MathSolution;

pub async fn chat_json_output_ds(prompt: &str) -> anyhow::Result<String> {
    let client = Client::new();
    let system = get_json_output_schema();
    let message = vec![
        ChatCompletionRequestSystemMessageArgs::default()
            .content(system)
            .build()?
            .into(),
        ChatCompletionRequestUserMessageArgs::default()
            .content(prompt)
            .build()?
            .into()
    ];

    let request = CreateChatCompletionRequestArgs::default()
        .model(DEEPSEEK_V4_FLASH)
        .messages(message)
        .response_format(JsonObject)
        .build()?
        .into();

    let response = client.chat().create(request).await?;

    let result: MathSolution = response.choices.into_iter().next().and_then(|c| c.message.content)
        .ok_or_else(|| anyhow::anyhow!("no message found"))
        .and_then(|c| serde_json::from_str(&c).map_err(Into::into))?;

    info!("result: {:?}", result);

    Ok("".to_string())
}

fn get_json_output_schema() -> String {
    let schema = schemars::schema_for!(MathSolution);
    let schema_json = serde_json::to_string_pretty(&schema).unwrap();
    format!(r#"
    The user will provide some exam text. Please parse the "question" and "answer" and output them in JSON format.

    EXAMPLE INPUT:
    Which is the highest mountain in the world? Mount Everest.

    EXAMPLE JSON OUTPUT:
    {schema_json}


    "#)
}