use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Topic {
    General,
    News,
    Finance,
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct WebSearchRequest {
    pub query: String,
    #[serde(default = "topic_default")]
    pub topic: Topic,
    #[serde(default = "max_result_default")]
    pub max_result: i32,
}

fn topic_default() -> Topic {
    Topic::News
}

fn max_result_default() -> i32 {
    10
}
