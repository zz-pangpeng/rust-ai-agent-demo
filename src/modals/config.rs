use crate::modals::permission_input::PermissionMode;
use config::{Config, File};
use tracing::{error, info};

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
pub struct AgentConfig {
    #[serde(default = "agent_max_steps")]
    pub agent_max_steps: usize,
    #[serde(default = "agent_execute_timeout")]
    pub agent_execute_timeout: u64,
    #[serde(default = "agent_execute_retry_count")]
    pub agent_execute_retry_count: usize,
    #[serde(default = "tool_execute_timeout")]
    pub tool_execute_timeout: u64,
    #[serde(default = "tool_callback_execute_timeout")]
    pub tool_callback_execute_timeout: u64,
    #[serde(default = "permission_timeout")]
    pub permission_timeout: u64,
    #[serde(default = "permission_mode")]
    pub permission_mode: PermissionMode,
}

fn agent_max_steps() -> usize {
    10
}
fn agent_execute_timeout() -> u64 {
    30
}

fn tool_execute_timeout() -> u64 {
    15
}

fn tool_callback_execute_timeout() -> u64 {
    5
}
fn agent_execute_retry_count() -> usize {
    3
}

fn permission_timeout() -> u64 {
    5
}

fn permission_mode() -> PermissionMode {
    PermissionMode::default()
}

impl AgentConfig {
    pub fn new() -> AgentConfig {
        let run_mode = std::env::var("RUN_MODE").unwrap_or_else(|_| "development".to_string());

        let config = Config::builder()
            .add_source(File::with_name("config/default"))
            .add_source(File::with_name(&format!("config/{}", run_mode)).required(false))
            .build();
        let agent_config = match config {
            Ok(tool_config) => tool_config.try_deserialize().unwrap_or_else(|err| {
                error!("Failed to parse config: {}", err);
                AgentConfig::default()
            }),
            Err(_) => AgentConfig::default(),
        };

        info!("{:?}", agent_config);
        agent_config
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        AgentConfig {
            agent_max_steps: agent_max_steps(),
            agent_execute_timeout: agent_execute_timeout(),
            agent_execute_retry_count: agent_execute_retry_count(),
            tool_execute_timeout: tool_execute_timeout(),
            tool_callback_execute_timeout: tool_callback_execute_timeout(),
            permission_timeout: permission_timeout(),
            permission_mode: permission_mode(),
        }
    }
}
