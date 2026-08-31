use chrono::Local;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use ai_agent::agent::runtime::Agent;
use ai_agent::state::DEEPSEEK_V4_FLASH;
use ai_agent::tools::tool::get_tools;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    dotenvy::dotenv()?;
    let now = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let system = format!(
        r#"y
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

    let mut agent = Agent::new(DEEPSEEK_V4_FLASH, Some(system));
    let (tools, tools_map) = get_tools().await?;
    agent.bind_tool_calls(tools, tools_map).set_max_steps(10);

    let question = r#"
        - 1234 * 4321等于多少
        - 2026年美加墨世界杯最佳射手
    "#.to_string();


    let result = agent.run(question).await?;

    info!("input: {}", result.input);
    info!("output: {}", result.output);
    info!("execute step: {}", result.context.current_step);

    Ok(())
}
