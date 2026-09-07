use crate::modals::permission_input::{
    PermissionEntry, PermissionInput, PermissionMode, PermissionResult,
};
use crate::modals::tool::ToolView;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot::Sender;
use tokio::sync::{Mutex, oneshot};
use tokio::time::timeout;
use tracing::{error, info};

pub struct Permission {
    mode: PermissionMode,
    state: Arc<Mutex<PermissionState>>,
}

pub struct PermissionState {
    queue: VecDeque<PermissionEntry>,
    input: Box<dyn PermissionInput>,
    map: HashMap<String, PermissionResult>,
    waits: HashMap<String, Vec<Sender<PermissionResult>>>,
}

impl Permission {
    pub fn new(input: Box<dyn PermissionInput>) -> Self {
        Permission {
            mode: input.mode(),
            state: Arc::new(Mutex::new(PermissionState {
                queue: VecDeque::new(),
                input,
                map: HashMap::new(),
                waits: HashMap::new(),
            })),
        }
    }

    pub async fn reset(&self) -> &Self {
        let mut state = self.state.lock().await;
        state.queue.clear();
        state.waits.clear();
        state.map.clear();
        self
    }

    pub async fn query(&self, tool_view: &ToolView) -> PermissionResult {
        if self.mode == PermissionMode::AutoApprove {
            return PermissionResult::GrantedAlways;
        }
        if self.mode == PermissionMode::AutoDeny {
            return PermissionResult::Denied;
        }

        let mut permission_state = self.state.lock().await;

        let key = format!(
            "id: {}, tool name: {}, arguments: {}",
            tool_view.tool_call_id, tool_view.name, tool_view.arguments
        );
        // 1) key 级历史决定:Denied/GrantedAlways 直接复用;Once/Timeout/NoInput 失效后移除,重试时重新询问
        if let Some(result) = permission_state.map.get(&key) {
            match result {
                PermissionResult::Denied | PermissionResult::GrantedAlways => {
                    return result.clone();
                }
                _ => {
                    permission_state.map.remove(&key);
                }
            }
        }

        // 2) 工具级 always 授权,直接放行,无需排队
        if permission_state.map.get(&tool_view.name) == Some(&PermissionResult::GrantedAlways) {
            return PermissionResult::GrantedAlways;
        }

        // 3) 入队,同一 key 只排一次
        if !permission_state.queue.iter().any(|entry| entry.key == key) {
            permission_state.queue.push_back(PermissionEntry {
                key: key.clone(),
                tool_view: tool_view.clone(),
            });
        }

        // 4) 注册等待者;同一 key 可能有多个等待者,全部追加而不是覆盖
        let (tx, rx) = oneshot::channel();
        permission_state
            .waits
            .entry(key.clone())
            .or_default()
            .push(tx);

        drop(permission_state);

        // 5) 启动/继续消费队列(已有循环在跑时这里会直接返回,新 key 由那个循环处理)
        self.execute().await;

        // 6) 等待决策
        match rx.await {
            Ok(result) => match result {
                PermissionResult::Denied | PermissionResult::GrantedAlways => result,
                _ => {
                    let mut permission_state = self.state.lock().await;
                    permission_state.map.remove(&key);
                    result
                }
            },
            Err(_) => PermissionResult::Timeout,
        }
    }

    fn sender(&self, channel: Option<Vec<Sender<PermissionResult>>>, result: PermissionResult) {
        for tx in channel.unwrap_or_default() {
            let _ = tx.send(result.clone());
        }
    }

    async fn execute(&self) {
        let mut permission_state = self.state.lock().await;

        while let Some(entry) = permission_state.queue.pop_front() {
            // 按“这个 key 自己的工具”判断 always 授权,而不是按调用方工具
            if permission_state.map.get(&entry.tool_view.name)
                == Some(&PermissionResult::GrantedAlways)
            {
                info!("用户已授权工具{}使用", entry.tool_view.name);
                self.sender(
                    permission_state.waits.remove(&entry.key),
                    PermissionResult::GrantedAlways,
                );
                continue; // 继续消费队列,而不是 return
            }

            let duration = Duration::from_secs(entry.tool_view.config.permission_timeout);
            let result = match timeout(duration, permission_state.input.ask(&entry)).await {
                Ok(result) => {
                    if result == PermissionResult::GrantedAlways {
                        permission_state.map.insert(
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

            permission_state
                .map
                .insert(entry.key.clone(), result.clone());
            self.sender(permission_state.waits.remove(&entry.key), result);
        }
    }
}
