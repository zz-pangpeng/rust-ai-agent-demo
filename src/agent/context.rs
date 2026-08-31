use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::agent::event::Event;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Context {
    pub execution_id: String,
    pub event: Vec<Event>,
    pub current_step: usize,
    pub state: HashMap<String, Value>,
    pub result: Option<String>,
}


impl Context {
    pub fn new() -> Self {
        
        Self {
            execution_id: uuid::Uuid::new_v4().to_string(),
            event: Vec::new(),
            current_step: 0,
            state: HashMap::new(),
            result: None,
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
