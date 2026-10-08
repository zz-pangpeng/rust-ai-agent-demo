use crate::modals::permission_input::PermissionMode;
use config::{Config, File};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct Agent {
    pub max_rounds: usize,
    pub execute_timeout: u64,
    pub execute_retry_count: usize,
}

impl Default for Agent {
    fn default() -> Self {
        Agent {
            max_rounds: 10,
            execute_timeout: 180,
            execute_retry_count: 3,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct Tool {
    pub execute_timeout: u64,
    pub callback_execute_timeout: u64,
    pub result_max_token: usize
}

impl Default for Tool {
    fn default() -> Self {
        Tool {
            execute_timeout: 30,
            callback_execute_timeout: 10,
            result_max_token: 1000
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct Permission {
    pub execute_timeout: u64,
    pub mode: PermissionMode,
}

impl Default for Permission {
    fn default() -> Self {
        Permission {
            execute_timeout: 5,
            mode: PermissionMode::default(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct Compression {
    pub enabled: bool,
    pub trigger_threshold_ratio: f64,
    pub deep_compress_threshold: f64,
    pub max_summary_tokens: u16,
    pub max_in_memory_events: u16,
    pub min_compression_interval: usize,
    pub keep_recent_rounds: usize,
    pub archive_path: String,
    pub preserve_user_messages: bool,
    pub preserve_errors: bool,
}

impl Default for Compression {
    fn default() -> Self {
        Compression {
            enabled: true,                          // 开启压缩
            trigger_threshold_ratio: 0.7,           // 使用 70% 时触发
            max_summary_tokens: 4000,               // 总结最大 token 数
            max_in_memory_events: 200,              // 内存中最大事件数
            min_compression_interval: 3,            // 最小压缩间隔（轮次）
            keep_recent_rounds: 3,                  // 保留最近 N 轮不压缩
            archive_path: "./archives".to_string(), // 归档文件路径
            deep_compress_threshold: 0.9,           // 使用 90% 时触发深度压缩
            preserve_user_messages: true,           // 保留所有用户消息
            preserve_errors: true,                  // 保留所有错误信息
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct Vector {
    pub chunk_size: usize,
    pub chunk_overlap: usize,
    pub top_k: usize,
    pub model: String,
    pub execute_timeout: u64
}

impl Default for Vector {
    fn default() -> Self {
        Vector {
            chunk_size: 100,
            chunk_overlap: 20,
            top_k: 5,
            model: "openai/text-embedding-3-small".to_string(),
            execute_timeout: 5,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct SystemConfig {
    #[serde(default)]
    pub agent: Agent,
    #[serde(default)]
    pub tool: Tool,
    #[serde(default)]
    pub permission: Permission,
    #[serde(default)]
    pub compression: Compression,
    #[serde(default)]
    pub vector: Vector,
}

impl SystemConfig {
    pub fn new() -> SystemConfig {
        let run_mode = std::env::var("RUN_MODE").unwrap_or_else(|_| "development".to_string());

        let config = Config::builder()
            .add_source(File::with_name("config/default"))
            .add_source(File::with_name(&format!("config/{}", run_mode)).required(false))
            .build();
        let agent_config = match config {
            Ok(tool_config) => tool_config.try_deserialize().unwrap_or_else(|err| {
                error!("Failed to parse config: {}", err);
                SystemConfig::default()
            }),
            Err(_) => SystemConfig::default(),
        };

        info!("{:?}", agent_config);
        agent_config
    }
}

impl Default for SystemConfig {
    fn default() -> Self {
        SystemConfig {
            agent: Agent::default(),
            tool: Tool::default(),
            permission: Permission::default(),
            compression: Compression::default(),
            vector: Vector::default(),
        }
    }
}
