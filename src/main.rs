mod llm;
mod state;
mod modals;
mod tools;
mod agent;
mod permission;

use tracing::{Level};
use tracing_subscriber::FmtSubscriber;
use crate::llm::chat_schema::chat_schema;
use crate::state::GEMMA;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    dotenvy::dotenv()?;
    //let response = chat_message(GPT_OSS_20B, Some("你是一个全能助手"), "新中国哪一年成立？").await?;
    let _response = chat_schema(GEMMA, Some("你是一位全能数学家"), "1x + 2x = 15, 求 x = ?").await?;

    Ok(())
}
