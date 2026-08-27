use std::collections::{HashMap, VecDeque};
use std::time::Duration;
use tokio::io::{stdin, stdout, AsyncBufReadExt, AsyncWriteExt, BufReader, Lines, Stdin};
use tokio::sync::oneshot;
use tokio::sync::oneshot::Sender;
use tokio::time::timeout;
use tracing::info;

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, PartialEq)]
pub enum PermissionResult {
    GrantedOnce,
    GrantedAlways,
    Denied,
    Timeout,
    NoInput,
}

#[derive(Debug, PartialEq)]
enum PermissionQueueState {
    Empty,
    Working,
}

/// 队列中的一项:key 与它归属的工具绑定,避免不同工具的 key 互相套用授权结果
#[derive(Debug)]
struct PermissionEntry {
    key: String,
    tool_name: String,
}

pub struct Permission {
    queue: VecDeque<PermissionEntry>,
    state: PermissionQueueState,
    duration: Duration,
    lines: Lines<BufReader<Stdin>>,
    map: HashMap<String, PermissionResult>,
    waits: HashMap<String, Vec<Sender<PermissionResult>>>,
}

impl Permission {
    pub fn new(duration: Duration) -> Self {
        let reader = BufReader::new(stdin());
        let lines = reader.lines();
        Permission {
            queue: VecDeque::new(),
            state: PermissionQueueState::Empty,
            duration,
            lines,
            map: HashMap::new(),
            waits: HashMap::new(),
        }
    }

    pub async fn query(&mut self, key: String, tool_name: String) -> PermissionResult {
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
        if self.map.get(&tool_name) == Some(&PermissionResult::GrantedAlways) {
            return PermissionResult::GrantedAlways;
        }

        // 3) 入队,同一 key 只排一次
        if !self.queue.iter().any(|entry| entry.key == key) {
            self.queue.push_back(PermissionEntry {
                key: key.clone(),
                tool_name,
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
            if self.map.get(&entry.tool_name) == Some(&PermissionResult::GrantedAlways) {
                info!("用户已授权工具{}使用", entry.tool_name);
                self.sender(entry.key, PermissionResult::GrantedAlways);
                continue; // 继续消费队列,而不是 return
            }

            println!("是否允许执行工具{}： 允许一次/允许该工具所有操作/拒绝： y/a/n", entry.tool_name);
            stdout().flush().await.unwrap();

            let status = match timeout(self.duration, self.lines.next_line()).await {
                Ok(Ok(Some(line))) => match line.trim().to_ascii_lowercase().as_str() {
                    "y" => PermissionResult::GrantedOnce,
                    "a" => {
                        self.map
                            .insert(entry.tool_name.clone(), PermissionResult::GrantedAlways);
                        PermissionResult::GrantedAlways
                    }
                    "n" => PermissionResult::Denied,
                    _ => PermissionResult::NoInput,
                },
                _ => PermissionResult::Timeout,
            };

            self.map.insert(entry.key.clone(), status.clone());
            self.sender(entry.key, status);
        }

        self.state = PermissionQueueState::Empty;
    }
}
