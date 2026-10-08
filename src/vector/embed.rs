use async_openai::Client;
use async_openai::config::OpenAIConfig;
use async_openai::types::embeddings::{CreateEmbeddingRequestArgs, EmbeddingInput};
use backon::{ExponentialBuilder, Retryable};

pub async fn embed(text: &[String], model: &str) -> anyhow::Result<Vec<Vec<f32>>> {
    if text.is_empty() {
        return Ok(Vec::new());
    }
    let api_key = std::env::var("EMBED_API_KEY")?;
    let base_url = std::env::var("EMBED_BASE_URL")?;
    let config = OpenAIConfig::default()
        .with_api_key(api_key)
        .with_api_base(base_url);
    let client = Client::with_config(config);
    let request = CreateEmbeddingRequestArgs::default()
        .model(model)
        .input(EmbeddingInput::StringArray(text.to_owned()))
        .build()?;
    let response = (|| async { client.embeddings().create(request.clone()).await })
        .retry(ExponentialBuilder::new().with_max_times(3))
        .await?;

    let mut data = response.data;
    data.sort_by_key(|a| a.index);

    let result = data
        .into_iter()
        .map(|data| data.embedding)
        .collect::<Vec<Vec<f32>>>();

    Ok(result)
}
