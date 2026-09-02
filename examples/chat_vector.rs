use ai_agent::modals::web_search::Topic;
use ai_agent::state::TEXT_EMBEDDING_3_SMALL_MODEL;
use ai_agent::tools::vector::chunk::chunk_handle;
use ai_agent::tools::vector::search::vector_search;
use serde::Serialize;
use std::time::Duration;
use tavily::{SearchRequest, Tavily};
use tiktoken_rs::cl100k_base;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

#[derive(Debug, Serialize)]
struct TavilySearch {
    query: String,
    topic: String,
    max_result: i32,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    dotenvy::dotenv()?;
    let tvly_api_key = std::env::var("TVLY_API_KEY")?;

    let question = "2026年美加墨世界杯最佳射手";

    let tavily = Tavily::builder(&tvly_api_key)
        .timeout(Duration::from_secs(30))
        .max_retries(3)
        .build()?;

    let request = SearchRequest::new(&tvly_api_key, question.to_string())
        .topic("general")
        .max_results(10);

    let search_result = tavily.call(&request).await?;
    let contents = search_result
        .results
        .into_iter()
        .map(|data| data.content)
        .collect::<Vec<_>>()
        .join("\n\n");

    let core_bpe = cl100k_base()?;
    let token = core_bpe.encode_with_special_tokens(&contents).len();
    info!("token: {}", token);

    let chunks = chunk_handle(&contents, 100, 10);
    let vector_search_result = vector_search(question, &chunks, 5).await?;
    info!("vector_search_result: {:?}", vector_search_result);
    let vector_search_result_string = vector_search_result
        .into_iter()
        .map(|data| format!("{:?}", data))
        .collect::<Vec<_>>()
        .join("\n\n");
    let new_token = core_bpe
        .encode_with_special_tokens(&vector_search_result_string)
        .len();
    info!("new_token: {}", new_token);
    Ok(())
}
