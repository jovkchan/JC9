use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use chrono::Utc;
use super::types::{BlackboardEntry, BlackboardEntryType};

/// 共享内存黑板 - 遵循设计规约的数据存储与检索
pub struct SharedBlackboard {
    entries: Arc<RwLock<HashMap<String, BlackboardEntry>>>,
}

impl SharedBlackboard {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 写入/更新一个黑板条目
    pub async fn write(
        &self,
        entry_type: BlackboardEntryType,
        key: String,
        value: String,
        source_worker: String,
    ) -> String {
        // 强规约与数据结构化校验
        match entry_type {
            BlackboardEntryType::GlobalConfigPath => {
                if value.trim().is_empty() {
                    eprintln!("❌ [Blackboard] 拒绝写入：GlobalConfigPath 的值不能为空");
                    return String::new();
                }
            }
            BlackboardEntryType::EnvVariable => {
                if !value.contains('=') && serde_json::from_str::<serde_json::Value>(&value).is_err() {
                    eprintln!("⚠️ [Blackboard] 写入警告：EnvVariable 应符合 KEY=VALUE 键值对或 JSON 规约格式");
                }
            }
            _ => {}
        }

        let id = uuid::Uuid::new_v4().to_string();
        let entry = BlackboardEntry {
            id: id.clone(),
            entry_type,
            key: key.clone(),
            value,
            source_worker,
            timestamp: Utc::now(),
        };
        self.entries.write().await.insert(key, entry);
        id
    }

    /// 读取一个黑板条目
    pub async fn read(&self, key: &str) -> Option<BlackboardEntry> {
        self.entries.read().await.get(key).cloned()
    }

    /// 获取所有的黑板条目列表
    pub async fn get_all(&self) -> Vec<BlackboardEntry> {
        self.entries.read().await.values().cloned().collect()
    }

    /// 根据类型筛选黑板条目
    pub async fn get_by_type(&self, entry_type: BlackboardEntryType) -> Vec<BlackboardEntry> {
        self.entries
            .read()
            .await
            .values()
            .filter(|e| e.entry_type == entry_type)
            .cloned()
            .collect()
    }

    /// 移除某个条目
    pub async fn remove(&self, key: &str) -> Option<BlackboardEntry> {
        self.entries.write().await.remove(key)
    }

    /// 清空黑板
    pub async fn clear(&self) {
        self.entries.write().await.clear();
    }
}
