use std::collections::HashMap;
use std::sync::OnceLock;
use tokio::sync::{oneshot, RwLock};
use serde_json::Value;
use async_trait::async_trait;
use tauri::Emitter;

use super::tools::{Tool, ToolDefinition, ToolResult};
use super::security::SecuritySandbox;

pub type PendingCallsMap = RwLock<HashMap<String, oneshot::Sender<ToolResult>>>;

/// 获取全局挂起的 oneshot 发送端映射
pub fn pending_calls() -> &'static PendingCallsMap {
    static INSTANCE: OnceLock<PendingCallsMap> = OnceLock::new();
    INSTANCE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// 前端工具代理。后端决定调用时，将调用信息派发至前端 Vue 逻辑并挂起当前执行线程，等待前端计算完成后回调返回。
pub struct FrontendProxyTool {
    definition: ToolDefinition,
    app_handle: tauri::AppHandle,
}

impl FrontendProxyTool {
    pub fn new(definition: ToolDefinition, app_handle: tauri::AppHandle) -> Self {
        Self {
            definition,
            app_handle,
        }
    }
}

#[async_trait]
impl Tool for FrontendProxyTool {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    async fn execute(&self, arguments: &Value, _sandbox: &SecuritySandbox) -> ToolResult {
        let call_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel::<ToolResult>();

        // 1. 存入全局挂起调用池中
        pending_calls().write().await.insert(call_id.clone(), tx);

        // 2. 向前端发出广播，带上参数和分配的 call_id
        if let Err(e) = self.app_handle.emit("ai:call-frontend-tool", serde_json::json!({
            "callId": call_id,
            "toolName": self.definition.name,
            "arguments": arguments
        })) {
            pending_calls().write().await.remove(&call_id);
            return ToolResult {
                success: false,
                output: "".into(),
                error: Some(format!("向前端派发工具调用失败: {}", e)),
            };
        }

        // 3. 挂起当前线程，等待前端 Vue 计算完毕并回调（加 30 秒超时）
        match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => {
                pending_calls().write().await.remove(&call_id);
                ToolResult {
                    success: false,
                    output: "".into(),
                    error: Some("前端工具通道意外断开".into()),
                }
            }
            Err(_) => {
                pending_calls().write().await.remove(&call_id);
                ToolResult {
                    success: false,
                    output: "".into(),
                    error: Some("前端工具计算执行超时（30 秒）".into()),
                }
            }
        }
    }
}
