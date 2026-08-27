#[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Clone)]
pub struct ToolConfig {
    #[serde(default = "all_execute_timeout")]
    pub all_execute_timeout: u64,
    #[serde(default = "step_execute_timeout")]
    pub step_execute_timeout: u64,
    #[serde(default = "execute_retry_count")]
    pub execute_retry_count: usize,
    #[serde(default="permission_timeout")]
    pub permission_timeout: u64,
    #[serde(default = "model_execute_timeout")]
    pub model_execute_timeout: u64
}

fn all_execute_timeout() -> u64 {
    30
}
fn step_execute_timeout() -> u64 {
    10
}
fn execute_retry_count() -> usize {
    3
}

fn permission_timeout() -> u64 {
    5
}

fn model_execute_timeout() -> u64 {
    60
}
