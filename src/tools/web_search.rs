use std::io::Write;
use std::time::Duration;
use schemars::schema_for;
use serde_json::Value;
use tavily::{SearchRequest, Tavily};
use tracing::{error, info};
use crate::modals::tool::ToolView;
use crate::modals::web_search::{Topic, WebSearchRequest};
use crate::tools::tool::Tool;
use crate::tools::vector::chunk::chunk_handle;
use crate::tools::vector::search::vector_search;

pub struct WebSearch;

const COMPRESS_THRESHOLD: usize = 500;
const CHUNK_SIZE: usize = 100;
const CHUNK_OVERLAP: usize = 50;
const TOP_K: usize = 3;

#[async_trait::async_trait]
impl Tool for WebSearch {
    fn name(&self) -> &'static str {
        "web_search"
    }

    fn description(&self) -> &'static str {
        "网络搜索大模型未知的信息"
    }

    fn parameters(&self) -> Value {
        serde_json::to_value(schema_for!(WebSearchRequest)).expect("to serialize parameters")
    }

    async fn execute(&self, args: &str) -> anyhow::Result<String> {
        let tvly_api_key = std::env::var("TVLY_API_KEY")?;
        let args: WebSearchRequest = serde_json::from_str(args)?;
        let tavily = Tavily::builder(&tvly_api_key)
            .timeout(Duration::from_secs(30))
            .max_retries(3)
            .build()?;
        let topic = match args.topic {
            Topic::General => "general",
            Topic::News => "news",
            Topic::Finance => "finance",
        };
        let request = SearchRequest::new(&tvly_api_key, args.query)
            .topic(topic)
            .max_results(args.max_result);

        let result = tavily.call(&request).await?;

        Ok(format!("{:?}", result))
    }

    async fn before_callback(&self, tool_view: &ToolView) -> Option<String> {
        if tool_view.name != self.name() {
            return None;
        }
        println!("即将调用{}进行网络搜索，是否允许", self.name());
        println!("搜索内容{}", tool_view.arguments.clone());

        let result = tokio::task::spawn_blocking(move || {
            println!("是否执行？（y/n）");
            std::io::stdout().flush().unwrap();
            let mut input = "".to_string();
            std::io::stdin().read_line(&mut input).unwrap();
            input.trim().eq_ignore_ascii_case("y")

        }).await.unwrap_or(false);
        if result {
            return None;
        }
        Some(format!("用户拒绝使用{}工具执行", self.name()))
    }

    async fn after_callback(&self, tool_view: &ToolView, result: String) -> String {
        let chars = result.chars().collect::<Vec<char>>();
        info!("chars: {:?}", chars.len());
        if chars.len() < COMPRESS_THRESHOLD {
            return result;
        }
        let request = serde_json::from_str::<WebSearchRequest>(tool_view.arguments.as_str());
        match request {
            Err(_) => {
                result
            },
            Ok(request) => {
                info!("当前搜索内容过大，现进行向量搜索");
                let text = request.query;
                let chunks = chunk_handle(result.as_str(), CHUNK_SIZE, CHUNK_OVERLAP);
                let vector_result = vector_search(&text, &chunks, TOP_K ).await;
                match vector_result {
                    Err(e) => {
                        error!("向量搜索失败： {:?}", e);
                        result
                    },
                    Ok(vector_result) => {
                        let new_result = vector_result.iter().map(|data| data.content.clone())
                            .collect::<Vec<String>>().join("\n\n");
                        new_result
                    }
                }
            }
        }
    }
}