use crate::modals::config::AgentConfig;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct ToolView {
    pub tool_call_id: String,
    pub name: String,
    pub arguments: String,
    pub model: String,
    pub config: AgentConfig,
}
