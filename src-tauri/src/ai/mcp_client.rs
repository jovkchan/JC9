use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// MCP 服务端信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerInfo {
    pub id: String,
    pub name: String,
    pub url: String,
    pub status: String, // "connected" | "disconnected" | "error"
    pub tools: Vec<serde_json::Value>,
}

/// MCP 客户端管理桩实现
pub struct McpClient {
    servers: Arc<RwLock<Vec<McpServerInfo>>>,
}

impl McpClient {
    pub fn new() -> Self {
        Self {
            servers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 连接并注册一个新的 MCP 服务器
    pub async fn connect(&self, name: String, url: String) -> Result<(), String> {
        let mut servers = self.servers.write().await;
        
        // 避免重复名字
        if let Some(s) = servers.iter_mut().find(|s| s.name == name) {
            s.url = url;
            s.status = "connected".into();
        } else {
            servers.push(McpServerInfo {
                id: uuid::Uuid::new_v4().to_string(),
                name,
                url,
                status: "connected".into(),
                tools: vec![],
            });
        }
        Ok(())
    }

    /// 获取所有注册的 MCP 服务器列表
    pub async fn list_servers(&self) -> Vec<McpServerInfo> {
        self.servers.read().await.clone()
    }
}
