use ai_agent::llm::chat_json_output_ds::chat_json_output_ds;


use tracing::{Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    dotenvy::dotenv()?;
    let _response = chat_json_output_ds("1x + 2x = 15, 求x=?").await?;

    Ok(())
}
