use std::sync::Arc;
use tokio::sync::RwLock;
use sha2::{Sha256, Digest};
use super::types::LoopBreakerState;

/// 外部强制死循环熔断器
pub struct LoopBreaker {
    state: Arc<RwLock<LoopBreakerState>>,
}

impl LoopBreaker {
    pub fn new(worker_id: String) -> Self {
        Self {
            state: Arc::new(RwLock::new(LoopBreakerState {
                worker_id,
                tool_call_count: 0,
                consecutive_errors: 0,
                error_hashes: Vec::new(),
                warning_injected: false,
                is_tripped: false,
                trip_reason: None,
            })),
        }
    }

    pub async fn is_tripped(&self) -> bool {
        self.state.read().await.is_tripped
    }

    pub async fn trip_reason(&self) -> Option<String> {
        self.state.read().await.trip_reason.clone()
    }

    pub async fn record_tool_call(&self) {
        let mut state = self.state.write().await;
        state.tool_call_count += 1;
        
        // 连续调用 Tool 超过 15 次触发熔断
        if state.tool_call_count > 15 {
            state.is_tripped = true;
            state.trip_reason = Some(format!(
                "【死循环熔断】Worker {} 累计调用工具次数达 {} 次，超过上限 15 次，可能存在逻辑死循环，系统已强制熔断。",
                state.worker_id, state.tool_call_count
            ));
        }
    }

    pub async fn record_error(&self, error: &str) {
        let mut state = self.state.write().await;
        state.consecutive_errors += 1;

        // 计算错误内容的 sha256 哈希值
        let mut hasher = Sha256::new();
        hasher.update(error.as_bytes());
        let hash_result = hasher.finalize();
        let err_hash = hex::encode(hash_result);
        
        state.error_hashes.push(err_hash.clone());

        // 检查连续 3 次产生高度一致的报错
        let len = state.error_hashes.len();
        if len >= 3 {
            let last_three = &state.error_hashes[len - 3..];
            if last_three[0] == last_three[1] && last_three[1] == last_three[2] {
                state.is_tripped = true;
                state.trip_reason = Some(format!(
                    "【死循环熔断】检测到 Worker {} 连续 3 次产生相同错误，判断已陷于死循环：{}",
                    state.worker_id, error
                ));
                return;
            }
        }

        // 如果连续报错超过 5 次，也认为应该触发熔断
        if state.consecutive_errors >= 5 {
            state.is_tripped = true;
            state.trip_reason = Some(format!(
                "【熔断】Worker {} 连续执行报错 {} 次，强制熔断挂起。",
                state.worker_id, state.consecutive_errors
            ));
        }
    }

    pub async fn reset_consecutive_errors(&self) {
        let mut state = self.state.write().await;
        state.consecutive_errors = 0;
    }

    pub async fn get_state(&self) -> LoopBreakerState {
        self.state.read().await.clone()
    }
}
