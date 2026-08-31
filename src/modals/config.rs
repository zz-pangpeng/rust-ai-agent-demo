use config::{Config, File};
use tracing::info;

#[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Clone)]
pub struct AgentConfig {
    #[serde(default = "agent_max_steps")]
    pub agent_max_steps: usize,
    #[serde(default = "agent_execute_timeout")]
    pub agent_execute_timeout: u64,
    #[serde(default = "execute_retry_count")]
    pub agent_execute_retry_count: usize,
    #[serde(default = "tool_execute_timeout")]
    pub tool_execute_timeout: u64,
    #[serde(default = "tool_callback_execute_timeout")]
    pub tool_callback_execute_timeout: u64,
    #[serde(default="permission_timeout")]
    pub permission_timeout: u64,
}

fn agent_max_steps() -> usize { 10 }
fn agent_execute_timeout() -> u64 { 30 }

fn tool_execute_timeout() -> u64 { 15 }

fn tool_callback_execute_timeout() -> u64 { 5 }
fn execute_retry_count() -> usize {
    3
}

fn permission_timeout() -> u64 {
    5
}

impl AgentConfig {
    pub fn new() -> AgentConfig {
        let run_mode = std::env::var("RUN_MODE").unwrap_or_else(|_| "development".to_string());

        let config = Config::builder()
            .add_source(File::with_name("config/default"))
            .add_source(File::with_name(&format!("config/{}", run_mode)).required(false))
            .build();
        let agent_config = match config {
            Ok(tool_config) => {
                tool_config.try_deserialize().unwrap_or(AgentConfig::default())
            },
            Err(_) => {
                AgentConfig::default()
            }
        };

        info!("{:?}", agent_config);
        agent_config
    }
}
