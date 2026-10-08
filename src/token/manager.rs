use tiktoken_rs::{cl100k_base, CoreBPE};

pub struct TokenManager {
    system_budget: u64,         // 系统
    tool_budget: u64,           // 工具定义
    summary_budget: u64,        // 总结文本
    history_budget: u64,        // 近期未压缩事件
    reserve_budget: u64,        // 预留给模型输出的空间
    max_tokens: u64,            // 最大token
    core_bpe: CoreBPE
}

impl TokenManager {
    pub fn new(reserve_budget: u64, max_tokens: u64) -> TokenManager {
        let core_bpe = cl100k_base().expect("cl100l_base init error");
        TokenManager {
            system_budget: 0,
            tool_budget: 0,
            summary_budget: 0,
            history_budget: 0,
            reserve_budget,
            max_tokens,
            core_bpe,
        }
    }

    pub fn update_system_budget(&mut self, prompt: Option<&str>) -> &mut Self {
        match prompt {
            None => self.system_budget = 0,
            Some(prompt) => {
                let token = self.core_bpe.encode_with_special_tokens(prompt).len() as u64;
                self.system_budget = token;
            }
        }
        self
    }

    pub fn check_remaining_token(&self) -> bool {
        self.system_budget + self.tool_budget + self.summary_budget + self.history_budget + self.reserve_budget < self.max_tokens
    }
    
    pub fn calculate_token(&self,text: &str) -> usize {
        self.core_bpe.encode_with_special_tokens(text).len()
    }
}
