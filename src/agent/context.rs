use std::collections::HashMap;
use config::{Config, File};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::info;
use crate::agent::event::Event;
use crate::modals::config::ToolConfig;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Context {
    pub execution_id: String,
    pub event: Vec<Event>,
    pub current_step: usize,
    pub state: HashMap<String, Value>,
    pub result: Option<String>,
    pub tool_config: ToolConfig,
}


impl Context {
    pub fn new() -> Self {
        let run_mode = std::env::var("RUN_MODE").unwrap_or_else(|_| "development".to_string());

        let tool_config = Config::builder()
            .add_source(File::with_name("config/default"))
            .add_source(File::with_name(&format!("config/{}", run_mode)).required(false))
            .build();
        let tool_config = match tool_config {
            Ok(tool_config) => {
                tool_config.try_deserialize().unwrap_or(ToolConfig::default())
            },
            Err(_) => {
                ToolConfig::default()
            }
        };
        info!("{:?}", tool_config);
        
        Self {
            execution_id: uuid::Uuid::new_v4().to_string(),
            event: Vec::new(),
            current_step: 0,
            state: HashMap::new(),
            result: None,
            tool_config,
        }
    }

    pub fn add_event(&mut self, event: Event) {
        self.event.push(event);
    }

    pub fn increment_step(&mut self){
        self.current_step += 1;
    }

}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}
