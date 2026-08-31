use crate::modals::config::AgentConfig;

pub struct ToolView {
    pub tool_call_id: String,
    pub name: String,
    pub arguments: String,
    pub model: String,
    pub config: AgentConfig
}