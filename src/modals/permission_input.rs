use crate::modals::tool::ToolView;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines, Stdin, stdin, stdout};

#[async_trait::async_trait]
pub trait PermissionInput: Send + Sync {
    async fn ask(&mut self, permission_entry: &PermissionEntry) -> PermissionResult;
    fn mode(&self) -> PermissionMode;
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub enum PermissionResult {
    GrantedOnce,
    GrantedAlways,
    Denied,
    Timeout,
    NoInput,
}
#[derive(Debug)]
pub struct PermissionEntry {
    pub key: String,
    pub tool_view: ToolView,
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub enum PermissionMode {
    #[default]
    #[serde(rename = "interactive")]
    Interactive,
    #[serde(rename = "auto_approve")]
    AutoApprove,
    #[serde(rename = "auto_deny")]
    AutoDeny,
}

pub struct StdinPermissionInput {
    pub lines: Lines<BufReader<Stdin>>,
}

impl StdinPermissionInput {
    pub fn new() -> Self {
        let reader = BufReader::new(stdin());
        let lines = reader.lines();
        StdinPermissionInput { lines }
    }
}

pub fn input_match(line: &str) -> PermissionResult {
    match line.trim().to_ascii_lowercase().as_str() {
        "y" => PermissionResult::GrantedOnce,
        "a" => PermissionResult::GrantedAlways,
        "n" => PermissionResult::Denied,
        _ => PermissionResult::NoInput,
    }
}
#[async_trait::async_trait]
impl PermissionInput for StdinPermissionInput {
    async fn ask(&mut self, permission_entry: &PermissionEntry) -> PermissionResult {
        println!(
            "是否允许执行工具{}： 允许一次/允许该工具所有操作/拒绝： y/a/n",
            permission_entry.tool_view.name
        );
        stdout().flush().await.unwrap();
        if let Ok(Some(line)) = self.lines.next_line().await {
            return input_match(line.as_str());
        };
        PermissionResult::Timeout
    }

    fn mode(&self) -> PermissionMode {
        PermissionMode::Interactive
    }
}

pub struct GrantedAlwaysPermissionInput;
#[async_trait::async_trait]
impl PermissionInput for GrantedAlwaysPermissionInput {
    async fn ask(&mut self, _permission_entry: &PermissionEntry) -> PermissionResult {
        PermissionResult::GrantedAlways
    }

    fn mode(&self) -> PermissionMode {
        PermissionMode::AutoApprove
    }
}

pub struct DeniedPermissionInput;
#[async_trait::async_trait]
impl PermissionInput for DeniedPermissionInput {
    async fn ask(&mut self, _permission_entry: &PermissionEntry) -> PermissionResult {
        PermissionResult::Denied
    }
    fn mode(&self) -> PermissionMode {
        PermissionMode::AutoDeny
    }
}
