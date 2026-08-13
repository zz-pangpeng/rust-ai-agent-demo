use ai_agent::llm::chat::chat_message;
use ai_agent::state::GPT_OSS_20B;

use tracing::{Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    dotenvy::dotenv()?;
    let _response = chat_message(GPT_OSS_20B, Some("你是一个全能助手"), "深圳市有哪些好玩的景点").await?;

    Ok(())
}
