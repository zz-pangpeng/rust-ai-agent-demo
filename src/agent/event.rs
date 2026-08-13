use chrono::{Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum ContentItem {
    #[serde(rename = "message")]
    Message {
        role: String,
        content: String,
    },
    #[serde(rename = "tool_call")]
    ToolCall {
        tool_call_id: String,
        name: String,
        arguments: Value
    },
    #[serde(rename = "tool_result")]
    ToolCallResult {
        tool_call_id: String,
        name: String,
        status: ToolCallStatus,
        content: String,
    }
}
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallStatus {
    Success,
    Failure,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Event {
    pub id: String,
    /// 执行上下文中的id
    pub execution_id: String,
    pub content: Vec<ContentItem>,
    pub timestamp: i64,
    pub author: String,
}

impl Event {
    pub fn new(execution_id: impl Into<String>, author: impl Into<String>, content: Vec<ContentItem>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            execution_id: execution_id.into(),
            author: author.into(),
            content,
            timestamp: Utc::now().timestamp(),
        }
    }
}