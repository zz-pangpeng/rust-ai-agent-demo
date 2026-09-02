use ai_agent::llm::chat_tool_calculator::chat_tool_calculator;
use ai_agent::state::GPT_OSS_20B;
use backon::{ExponentialBuilder, Retryable};
use chrono::Local;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    dotenvy::dotenv()?;
    let now = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let system = format!(
        r#"
            当前时间是： {now}
            你是一位全能助手，并且可以使用工具来完成任务。
            1, 当需要进行数学加减乘除计算，则使用工具calculator
            2, 你的数据是有截止日期的，超出你的日期时，则使用工具web_search进行搜索
            3, 如果需要查询费用，则使用工具expense mcp。例如：
             - select_expense
            4，如果工具可以回答，则调用工具并把结果以自然语言，简洁内容输出
            5，已完成任务为第一要求，不要刻意使用工具
        "#
    );

    let question = vec![
        "1 + 10等于多少",
        "2026年美加墨世界杯决赛是哪个国家对哪个国家，谁赢，谁进球",
        "查询并列出我在电脑花费了多少钱。如果每个月可以攒下500，需要多个月才能把过去所有花费在电脑上的钱补回来",
    ];
    let mut thread_handles = vec![];
    for prompt in question {
        let system = system.clone();
        thread_handles.push(tokio::spawn(async move {
            let op = || async { chat_tool_calculator(GPT_OSS_20B, system.as_str(), prompt).await };
            op.retry(ExponentialBuilder::default().with_max_times(3))
                .await
                .expect("TODO: panic message");
        }))
    }

    for handle in thread_handles {
        let _ = handle.await;
    }

    Ok(())
}
