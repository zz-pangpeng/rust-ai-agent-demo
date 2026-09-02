use ai_agent::llm::chat_stream::chat_stream;
use ai_agent::state::GPT_OSS_20B;

use futures::{StreamExt, pin_mut};
use tracing::{Level, error, info};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    dotenvy::dotenv()?;
    let response = chat_stream(
        GPT_OSS_20B,
        Some("你是资深前端开发工程师"),
        "从网页请求到浏览器渲染，完整流程解析并且在每个流程中详细介绍可做性能优化的内容",
    );

    pin_mut!(response);

    let mut output = "".to_string();
    while let Some(res) = response.next().await {
        match res {
            Ok(text) => {
                info!("当前收到的内容： {}", text);
                output.push_str(&text)
            }
            Err(e) => {
                error!("流式输出报错：{}", e);
            }
        }
    }

    info!("完整输出内容： {}", output);

    Ok(())
}
