use crate::modals::permission_input::{
    PermissionEntry, PermissionInput, PermissionMode, PermissionResult,
};
use crate::modals::tool::ToolView;
use std::collections::{HashMap, VecDeque};
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::sync::oneshot::Sender;
use tokio::time::timeout;
use tracing::{error, info};

#[derive(Debug, PartialEq)]
enum PermissionQueueState {
    Empty,
    Working,
}

pub struct Permission {
    queue: VecDeque<PermissionEntry>,
    state: PermissionQueueState,
    input: Box<dyn PermissionInput>,
    map: HashMap<String, PermissionResult>,
    waits: HashMap<String, Vec<Sender<PermissionResult>>>,
}

impl Permission {
    pub fn new(input: Box<dyn PermissionInput>) -> Self {
        Permission {
            queue: VecDeque::new(),
            state: PermissionQueueState::Empty,
            input,
            map: HashMap::new(),
            waits: HashMap::new(),
        }
    }

    pub fn reset(&mut self) -> &mut Self {
        self.state = PermissionQueueState::Empty;
        self.queue = VecDeque::new();
        self.map = HashMap::new();
        self.waits = HashMap::new();
        self
    }

    pub async fn query(&mut self, tool_view: &ToolView) -> PermissionResult {
        if self.input.mode() == PermissionMode::AutoApprove {
            return PermissionResult::GrantedAlways;
        }
        if self.input.mode() == PermissionMode::AutoDeny {
            return PermissionResult::Denied;
        }

        let key = format!(
            "id: {}, tool name: {}, arguments: {}",
            tool_view.tool_call_id, tool_view.name, tool_view.arguments
        );
        // 1) key 级历史决定:Denied/GrantedAlways 直接复用;Once/Timeout/NoInput 失效后移除,重试时重新询问
        if let Some(result) = self.map.get(&key) {
            match result {
                PermissionResult::Denied | PermissionResult::GrantedAlways => {
                    return result.clone();
                }
                _ => {
                    self.map.remove(&key);
                }
            }
        }

        // 2) 工具级 always 授权,直接放行,无需排队
        if self.map.get(&tool_view.name) == Some(&PermissionResult::GrantedAlways) {
            return PermissionResult::GrantedAlways;
        }

        // 3) 入队,同一 key 只排一次
        if !self.queue.iter().any(|entry| entry.key == key) {
            self.queue.push_back(PermissionEntry {
                key: key.clone(),
                tool_view: tool_view.clone(),
            });
        }

        // 4) 注册等待者;同一 key 可能有多个等待者,全部追加而不是覆盖
        let (tx, rx) = oneshot::channel();
        self.waits.entry(key.clone()).or_default().push(tx);

        // 5) 启动/继续消费队列(已有循环在跑时这里会直接返回,新 key 由那个循环处理)
        self.execute().await;

        // 6) 等待决策
        match rx.await {
            Ok(result) => match result {
                PermissionResult::Denied | PermissionResult::GrantedAlways => result,
                _ => {
                    self.map.remove(&key);
                    result
                }
            },
            Err(_) => PermissionResult::Timeout,
        }
    }

    fn sender(&mut self, key: String, result: PermissionResult) {
        if let Some(senders) = self.waits.remove(&key) {
            for tx in senders {
                let _ = tx.send(result.clone());
            }
        }
    }

    async fn execute(&mut self) {
        // 防重入:已有循环在消费队列时,新入队的 key 会在那个循环里被处理
        if self.state == PermissionQueueState::Working || self.queue.is_empty() {
            return;
        }
        self.state = PermissionQueueState::Working;

        while let Some(entry) = self.queue.pop_front() {
            // 按“这个 key 自己的工具”判断 always 授权,而不是按调用方工具
            if self.map.get(&entry.tool_view.name) == Some(&PermissionResult::GrantedAlways) {
                info!("用户已授权工具{}使用", entry.tool_view.name);
                self.sender(entry.key, PermissionResult::GrantedAlways);
                continue; // 继续消费队列,而不是 return
            }

            let duration = Duration::from_secs(entry.tool_view.config.permission_timeout);
            let result = match timeout(duration, self.input.ask(&entry)).await {
                Ok(result) => {
                    if result == PermissionResult::GrantedAlways {
                        self.map.insert(
                            entry.tool_view.name.clone(),
                            PermissionResult::GrantedAlways,
                        );
                    }
                    result
                }
                Err(_) => {
                    error!(
                        "tool permission ask timeout; tool name: {}, arguments: {}",
                        entry.tool_view.name, entry.tool_view.arguments
                    );
                    PermissionResult::Timeout
                }
            };

            self.map.insert(entry.key.clone(), result.clone());
            self.sender(entry.key, result);
        }

        self.state = PermissionQueueState::Empty;
    }
}
