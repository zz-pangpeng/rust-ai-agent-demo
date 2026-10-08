use crate::agent::event::ToolCallStatus;
use crate::modals::permission_input::PermissionResult;
use crate::modals::tool::ToolView;
use crate::modals::web_search::{Topic, WebSearchRequest};
use crate::permission::Permission;
use crate::tools::tool::Tool;
use schemars::schema_for;
use serde_json::Value;
use std::time::Duration;
use tavily::{SearchRequest, Tavily};
use tracing::debug;

pub struct WebSearch;

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

    fn compress_query(&self, args: &str) -> Option<String> {
        match serde_json::from_str::<WebSearchRequest>(args) {
            Ok(request) => {
                Some(request.query)
            },
            Err(_) => {
                debug!("参数序列化失败");
                None
            }
        }
        
    }

    async fn execute(&mut self, args: &str) -> anyhow::Result<String> {
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

    async fn before_callback(
        &mut self,
        tool_view: &ToolView,
        permission: &Permission,
    ) -> Option<(ToolCallStatus, String)> {
        if tool_view.name != self.name() {
            return None;
        }
        println!("即将调用{}进行网络搜索", self.name());
        println!("搜索内容{}", tool_view.arguments.clone());

        let permission_result = permission.query(tool_view).await;
        match permission_result {
            PermissionResult::GrantedOnce | PermissionResult::GrantedAlways => None,
            PermissionResult::Denied => Some((ToolCallStatus::Denied, "用户拒绝".to_string())),
            PermissionResult::Timeout => {
                Some((ToolCallStatus::Failure, "等待用户授权超时".to_string()))
            }
            PermissionResult::NoInput => Some((
                ToolCallStatus::Failure,
                "用户输入无法识别，视为未授权".to_string(),
            )),
        }
    }

}
