use std::time::Duration;
use tavily::{SearchRequest, Tavily};
use tiktoken_rs::cl100k_base;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;
use ai_agent::modals::config::Vector;
use ai_agent::state::TEXT_EMBEDDING_3_SMALL_MODEL;
use ai_agent::token::estimate::estimate_tokens;
use ai_agent::vector::compress::compress;

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
    info!("estimate token: {}", estimate_tokens(&contents));

    let config = Vector {
        chunk_overlap: 20,
        chunk_size: 100,
        top_k: 3,
        model: TEXT_EMBEDDING_3_SMALL_MODEL.to_string(),
        execute_timeout: 5,
    };
    let vector_search_result_string = compress(&question, &contents, &config).await;
    let new_token = core_bpe
        .encode_with_special_tokens(&vector_search_result_string)
        .len();
    info!("new_token: {}", new_token);
    Ok(())
}
