use ai_agent::llm::chat_schema::chat_schema;
use ai_agent::state::GEMMA;

use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    dotenvy::dotenv()?;
    let _response = chat_schema(GEMMA, Some("你是一位数学家"), "1x + 2x = 15, 求x=?").await?;

    Ok(())
}
