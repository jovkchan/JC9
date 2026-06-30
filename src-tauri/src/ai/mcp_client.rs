use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::process::Stdio;
use tokio::sync::RwLock;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Command as TokioCommand};
use super::mcp_types::*;
use super::tools::{ToolRegistry, ToolDefinition};
use super::types::RiskLevel;

/// MCP 传输方式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum McpTransport {
    Sse,
    Stdio,
}

/// MCP 服务端信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerInfo {
    pub id: String,
    pub name: String,
    pub transport: McpTransport,
    pub url: Option<String>,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub status: String, // "connected" | "disconnected" | "error" | "connecting"
    pub tools: Vec<McpTool>,
    pub error_message: Option<String>,
}

/// 内部 MCP 连接状态
struct McpConnection {
    server_id: String,
    /// 消息 ID 计数器
    msg_id: u64,
    /// 待处理的请求（id → response sender）
    pending: HashMap<u64, tokio::sync::oneshot::Sender<Result<Value, String>>>,
    /// 进程句柄（stdio 模式）
    child: Option<tokio::process::Child>,
    /// 关闭信号
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
}

/// MCP 客户端 — 支持 SSE 和 stdio 两种传输方式
pub struct McpClient {
    servers: Arc<RwLock<Vec<McpServerInfo>>>,
    connections: Arc<RwLock<HashMap<String, McpConnection>>>,
}

impl McpClient {
    pub fn new() -> Self {
        Self {
            servers: Arc::new(RwLock::new(Vec::new())),
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 通过 SSE 连接 MCP 服务器
    pub async fn connect(&self, name: String, url: String) -> Result<(), String> {
        let id = uuid::Uuid::new_v4().to_string();
        let server_info = McpServerInfo {
            id: id.clone(),
            name: name.clone(),
            transport: McpTransport::Sse,
            url: Some(url.clone()),
            command: None,
            args: None,
            status: "connecting".into(),
            tools: vec![],
            error_message: None,
        };

        {
            let mut servers = self.servers.write().await;
            if let Some(existing) = servers.iter_mut().find(|s| s.name == name) {
                existing.status = "connecting".into();
                existing.error_message = None;
            } else {
                servers.push(server_info);
            }
        }

        // 启动 SSE 连接任务
        let servers_clone = self.servers.clone();
        let servers_clone2 = self.servers.clone();
        let connections_clone = self.connections.clone();
        let name_clone = name.clone();
        let url_clone = url.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::run_sse_connection(&name_clone, &url_clone, &id, servers_clone, connections_clone).await {
                Self::update_server_status(&name_clone, "error", Some(&e), &servers_clone2).await;
            }
        });

        Ok(())
    }

    /// 通过 stdio 连接本地 MCP 服务器
    pub async fn connect_stdio(&self, name: String, command: String, args: Vec<String>) -> Result<(), String> {
        let id = uuid::Uuid::new_v4().to_string();
        let server_info = McpServerInfo {
            id: id.clone(),
            name: name.clone(),
            transport: McpTransport::Stdio,
            url: None,
            command: Some(command.clone()),
            args: Some(args.clone()),
            status: "connecting".into(),
            tools: vec![],
            error_message: None,
        };

        {
            let mut servers = self.servers.write().await;
            if let Some(existing) = servers.iter_mut().find(|s| s.name == name) {
                existing.status = "connecting".into();
                existing.error_message = None;
            } else {
                servers.push(server_info);
            }
        }

        let servers_clone = self.servers.clone();
        let servers_clone2 = self.servers.clone();
        let connections_clone = self.connections.clone();
        let name_clone = name.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::run_stdio_connection(&name_clone, &command, &args, &id, servers_clone, connections_clone).await {
                Self::update_server_status(&name_clone, "error", Some(&e), &servers_clone2).await;
            }
        });

        Ok(())
    }

    /// 获取所有注册的 MCP 服务器列表
    pub async fn list_servers(&self) -> Vec<McpServerInfo> {
        self.servers.read().await.clone()
    }

    /// 断开 MCP 服务器
    pub async fn disconnect(&self, name: &str) {
        Self::update_server_status(name, "disconnected", None, &self.servers).await;
        let mut connections = self.connections.write().await;
        connections.remove(name);
    }

    /// 调用 MCP 工具（被 ToolRegistry 调用）
    pub async fn call_tool(&self, _server_name: &str, _tool_name: &str, _arguments: Value) -> Result<String, String> {
        // TODO: 完善请求/响应匹配机制
        Err("MCP 工具调用尚在实现中".into())
    }

    /// MCP 工具列表注入到 ToolRegistry
    pub async fn inject_tools(&self, registry: &ToolRegistry) {
        let servers = self.servers.read().await;
        for server in servers.iter() {
            for tool in &server.tools {
                let tool_name = format!("mcp_{}_{}", server.name, tool.name);
                let tool_def = tool.clone();
                let server_name = server.name.clone();
                let self_arc = Arc::new(McpToolWrapper {
                    server_name: server_name.clone(),
                    tool_def: tool_def.clone(),
                    client: self.servers.clone(),
                });
                registry.register(tool_name, self_arc as Arc<dyn super::tools::Tool>).await;
            }
        }
    }

    // ── 内部辅助方法 ──

    async fn update_server_status(name: &str, status: &str, error: Option<&str>, servers: &Arc<RwLock<Vec<McpServerInfo>>>) {
        let mut srv = servers.write().await;
        if let Some(s) = srv.iter_mut().find(|s| s.name == name) {
            s.status = status.into();
            s.error_message = error.map(|e| e.into());
        }
    }

    async fn update_server_tools(name: &str, tools: Vec<McpTool>, servers: &Arc<RwLock<Vec<McpServerInfo>>>) {
        let mut srv = servers.write().await;
        if let Some(s) = srv.iter_mut().find(|s| s.name == name) {
            s.tools = tools;
            s.status = "connected".into();
        }
    }

    /// SSE 连接循环
    #[allow(unused_variables)]
    async fn run_sse_connection(
        name: &str, url: &str, id: &str,
        servers: Arc<RwLock<Vec<McpServerInfo>>>,
        connections: Arc<RwLock<HashMap<String, McpConnection>>>,
    ) -> Result<(), String> {
        let client = reqwest::Client::new();

        // 1. SSE 连接
        let resp = client.get(url)
            .header("Accept", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .send()
            .await
            .map_err(|e| format!("SSE 连接失败: {}", e))?;

        // 2. 解析 SSE 流
        let stream = resp.bytes_stream();
        let mut buffer = String::new();
        let _current_event = String::new();

        use futures_util::StreamExt;
        let mut stream = Box::pin(stream);

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("SSE 流读取失败: {}", e))?;
            let chunk_str = String::from_utf8_lossy(&chunk);
            buffer.push_str(&chunk_str);

            // 解析 SSE 事件 (event:\ndata:\n\n)
            while let Some(pos) = buffer.find("\n\n") {
                let event_block = buffer[..pos].to_string();
                buffer = buffer[pos + 2..].to_string();

                for line in event_block.lines() {
                    if let Some(data) = line.strip_prefix("data: ") {
                        // 解析 JSON-RPC 消息
                        if let Ok(msg) = McpMessage::from_json(data) {
                            // 处理初始化响应
                            if msg.method.as_deref() == Some("initialized") {
                                // 开始工具列表获取
                                let list_req = McpMessage::request("tools/list", json!({}), 1);
                                // Send via HTTP POST
                                let _ = client.post(url).json(&list_req).send().await;
                            }
                            // 处理 tools/list 结果
                            if let Some(ref result) = msg.result {
                                if let Ok(list) = serde_json::from_value::<McpListToolsResult>(result.clone()) {
                                    Self::update_server_tools(name, list.tools, &servers).await;
                                }
                            }
                        }
                    }
                    if let Some(event) = line.strip_prefix("event: ") {
                        let _ = event;
                    }
                }
            }
        }

        Ok(())
    }

    /// stdio 连接循环
    async fn run_stdio_connection(
        name: &str, command: &str, args: &[String], id: &str,
        servers: Arc<RwLock<Vec<McpServerInfo>>>,
        connections: Arc<RwLock<HashMap<String, McpConnection>>>,
    ) -> Result<(), String> {
        let mut child = TokioCommand::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("启动 MCP 进程失败: {}", e))?;

        let stdin = child.stdin.take().ok_or("无法获取 stdin")?;
        let stdout = child.stdout.take().ok_or("无法获取 stdout")?;
        let mut reader = BufReader::new(stdout).lines();

        // 创建连接记录
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        {
            let mut conns = connections.write().await;
            conns.insert(name.to_string(), McpConnection {
                server_id: id.to_string(),
                msg_id: 0,
                pending: HashMap::new(),
                child: None,
                shutdown: Some(shutdown_tx),
            });
        }

        // 发送 initialize 请求
        let init_params = McpInitializeParams {
            protocol_version: "2024-11-05".into(),
            capabilities: ClientCapabilities {
                tools: Some(ToolCapabilities { list_changed: true }),
                resources: None,
            },
            client_info: ClientInfo {
                name: "jc9".into(),
                version: "1.0.0".into(),
            },
        };

        let init_msg = McpMessage::request("initialize", json!(init_params), 1);
        let mut write_stdin = stdin;
        write_stdin.write_all(init_msg.to_json()?.as_bytes()).await.map_err(|e| format!("写入 stdin 失败: {}", e))?;
        write_stdin.write_all(b"\n").await.ok();

        // 读取响应
        let mut msg_id: u64 = 1;
        let mut initialized = false;

        loop {
            tokio::select! {
                line = reader.next_line() => {
                    match line {
                        Ok(Some(text)) => {
                            if text.trim().is_empty() { continue; }
                            if let Ok(msg) = McpMessage::from_json(&text) {
                                if !initialized {
                                    // 处理 initialize 响应
                                    if msg.id == Some(1) && msg.result.is_some() {
                                        // 发送 initialized 通知
                                        let notif = McpMessage::notification("initialized", json!({}));
                                        write_stdin.write_all(notif.to_json()?.as_bytes()).await.ok();
                                        write_stdin.write_all(b"\n").await.ok();
                                        initialized = true;

                                        // 请求工具列表
                                        msg_id += 1;
                                        let list_req = McpMessage::request("tools/list", json!({}), msg_id);
                                        write_stdin.write_all(list_req.to_json()?.as_bytes()).await.ok();
                                        write_stdin.write_all(b"\n").await.ok();
                                    }
                                } else if msg.id == Some(msg_id) {
                                    if let Some(ref result) = msg.result {
                                        if let Ok(list) = serde_json::from_value::<McpListToolsResult>(result.clone()) {
                                            Self::update_server_tools(name, list.tools, &servers).await;
                                        }
                                    }
                                }
                            }
                        }
                        Ok(None) => break,
                        Err(e) => {
                            Self::update_server_status(name, "error", Some(&format!("stdio 读取错误: {}", e)), &servers).await;
                            break;
                        }
                    }
                }
                _ = &mut shutdown_rx => {
                    break;
                }
            }
        }

        // 清理
        let _ = child.kill().await;
        {
            let mut conns = connections.write().await;
            conns.remove(name);
        }

        Ok(())
    }
}

/// MCP 工具包装器 — 使 MCP 工具可被 ToolRegistry 调用
pub struct McpToolWrapper {
    server_name: String,
    tool_def: McpTool,
    client: Arc<RwLock<Vec<McpServerInfo>>>,
}

#[async_trait::async_trait]
impl super::tools::Tool for McpToolWrapper {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: format!("mcp_{}_{}", self.server_name, self.tool_def.name),
            description: format!("[MCP/{}] {}", self.server_name, self.tool_def.description),
            parameters: self.tool_def.input_schema.clone(),
            risk_level: RiskLevel::Medium,
        }
    }

    async fn execute(&self, _arguments: &Value, _sandbox: &super::security::SecuritySandbox) -> super::tools::ToolResult {
        // 实际调用会通过 McpClient.call_tool 路由
        // 当前为桩实现
        super::tools::ToolResult {
            success: false,
            output: "".into(),
            error: Some("MCP 工具调用尚在实现中".into()),
        }
    }
}
