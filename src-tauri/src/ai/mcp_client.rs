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
#[allow(dead_code)]
struct McpConnection {
    server_id: String,
    /// 消息 ID 计数器
    msg_id: u64,
    /// 待处理的请求（id → response sender）
    pending: HashMap<u64, tokio::sync::oneshot::Sender<Result<Value, String>>>,
    /// 进程句柄（stdio 模式）
    child: Option<tokio::process::Child>,
    /// stdin 写入端（stdio 模式）
    stdin: Option<Arc<tokio::sync::Mutex<tokio::process::ChildStdin>>>,
    /// SSE 基础 URL
    base_url: Option<String>,
    /// HTTP 客户端（SSE 模式）
    http_client: Option<reqwest::Client>,
    /// 关闭信号
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
}

/// MCP 客户端 — 支持 SSE 和 stdio 两种传输方式
pub struct McpClient {
    servers: Arc<RwLock<Vec<McpServerInfo>>>,
    connections: Arc<RwLock<HashMap<String, McpConnection>>>,
    tool_registry: Arc<tokio::sync::RwLock<Option<Arc<ToolRegistry>>>>,
}

impl McpClient {
    pub fn new() -> Self {
        Self {
            servers: Arc::new(RwLock::new(Vec::new())),
            connections: Arc::new(RwLock::new(HashMap::new())),
            tool_registry: Arc::new(tokio::sync::RwLock::new(None)),
        }
    }

    /// 绑定 ToolRegistry，使 MCP 工具自动注册到 Agent
    pub async fn bind_registry(&self, registry: Arc<ToolRegistry>) {
        *self.tool_registry.write().await = Some(registry);
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

        let reg = self.tool_registry.clone();
        tokio::spawn(async move {
            if let Err(e) = Self::run_sse_connection(&name_clone, &url_clone, &id, servers_clone, connections_clone, reg).await {
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
        let reg = self.tool_registry.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::run_stdio_connection(&name_clone, &command, &args, &id, servers_clone, connections_clone, reg).await {
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

    /// 健康检查 — 向服务器发送 ping
    pub async fn ping(&self, server_name: &str) -> Result<(), String> {
        let id = {
            let mut conns = self.connections.write().await;
            let conn = conns.get_mut(server_name).ok_or_else(|| format!("连接未找到: {}", server_name))?;
            conn.msg_id += 1;
            conn.msg_id
        };

        let (tx, rx) = tokio::sync::oneshot::channel::<Result<Value, String>>();
        {
            let mut conns = self.connections.write().await;
            if let Some(conn) = conns.get_mut(server_name) {
                conn.pending.insert(id, tx);
            }
        }

        let req = McpMessage::request("ping", json!({}), id);
        let req_json = req.to_json()?;

        let (transport, _sse_url) = {
            let servers = self.servers.read().await;
            let srv = servers.iter().find(|s| s.name == server_name)
                .ok_or_else(|| format!("服务器 '{}' 不存在", server_name))?;
            (srv.transport.clone(), srv.url.clone())
        };

        match transport {
            McpTransport::Stdio => {
                let conns = self.connections.read().await;
                if let Some(conn) = conns.get(server_name) {
                    if let Some(stdin) = &conn.stdin {
                        let mut w = stdin.lock().await;
                        w.write_all(req_json.as_bytes()).await.map_err(|e| format!("ping 写入失败: {}", e))?;
                        w.write_all(b"\n").await.ok();
                    }
                }
            }
            McpTransport::Sse => {
                let conns = self.connections.read().await;
                if let Some(conn) = conns.get(server_name) {
                    if let (Some(client), Some(url)) = (&conn.http_client, &conn.base_url) {
                        let _ = client.post(url).body(req_json).send().await;
                    }
                }
            }
        }

        match tokio::time::timeout(std::time::Duration::from_secs(10), rx).await {
            Ok(Ok(Ok(_))) => Ok(()),
            Ok(Ok(Err(e))) => Err(e),
            Ok(Err(_)) => Err("ping 通道关闭".into()),
            Err(_) => {
                let mut conns = self.connections.write().await;
                if let Some(conn) = conns.get_mut(server_name) {
                    conn.pending.remove(&id);
                }
                Err("ping 超时 (10s)".into())
            }
        }
    }

    /// 对所有已连接服务器执行健康检查，断开自动标记
    pub async fn health_check_all(&self) {
        let names: Vec<String> = {
            let servers = self.servers.read().await;
            servers.iter()
                .filter(|s| s.status == "connected")
                .map(|s| s.name.clone())
                .collect()
        };
        for name in &names {
            if self.ping(name).await.is_err() {
                Self::update_server_status(name, "error", Some("健康检查失败"), &self.servers).await;
            }
        }
    }

    /// 调用 MCP 工具（被 ToolRegistry 调用）
    pub async fn call_tool(&self, server_name: &str, tool_name: &str, arguments: Value) -> Result<String, String> {
        let msg_id = {
            let mut conns = self.connections.write().await;
            let conn = conns.get_mut(server_name).ok_or_else(|| format!("MCP 服务器 '{}' 未连接", server_name))?;
            conn.msg_id += 1;
            conn.msg_id
        };

        let (tx, rx) = tokio::sync::oneshot::channel::<Result<Value, String>>();

        {
            let mut conns = self.connections.write().await;
            let conn = conns.get_mut(server_name).ok_or_else(|| format!("连接已断开: {}", server_name))?;
            conn.pending.insert(msg_id, tx);
        }

        let call_params = McpCallToolParams {
            name: tool_name.to_string(),
            arguments: Some(arguments),
        };
        let req = McpMessage::request("tools/call", json!(call_params), msg_id);
        let req_json = req.to_json()?;

        // 查找服务器信息以确定传输方式
        let (transport, _sse_url) = {
            let servers = self.servers.read().await;
            let srv = servers.iter().find(|s| s.name == server_name)
                .ok_or_else(|| format!("服务器 '{}' 不存在", server_name))?;
            (srv.transport.clone(), srv.url.clone())
        };

        match transport {
            McpTransport::Stdio => {
                let conns = self.connections.read().await;
                if let Some(conn) = conns.get(server_name) {
                    if let Some(stdin) = &conn.stdin {
                        let mut w = stdin.lock().await;
                        w.write_all(req_json.as_bytes()).await.map_err(|e| format!("写入 stdin 失败: {}", e))?;
                        w.write_all(b"\n").await.ok();
                    } else {
                        return Err("stdio 未就绪".into());
                    }
                } else {
                    return Err("连接未找到".into());
                }
            }
            McpTransport::Sse => {
                let conns = self.connections.read().await;
                if let Some(conn) = conns.get(server_name) {
                    if let (Some(client), Some(url)) = (&conn.http_client, &conn.base_url) {
                        let resp = client.post(url)
                            .header("Content-Type", "application/json")
                            .body(req_json.clone())
                            .send()
                            .await
                            .map_err(|e| format!("SSE 请求失败: {}", e))?;
                        // SSE 响应通过事件流返回，通过 pending 通道等待
                        let status = resp.status();
                        if !status.is_success() {
                            let body = resp.text().await.unwrap_or_default();
                            return Err(format!("SSE 返回错误状态 {}: {}", status, body));
                        }
                    }
                }
            }
        }

        // 等待响应（超时 60 秒）
        match tokio::time::timeout(std::time::Duration::from_secs(60), rx).await {
            Ok(Ok(Ok(result))) => {
                Ok(serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()))
            }
            Ok(Ok(Err(e))) => Err(e),
            Ok(Err(_)) => Err("MCP 工具调用通道已关闭".into()),
            Err(_) => {
                // 超时，清理 pending
                let mut conns = self.connections.write().await;
                if let Some(conn) = conns.get_mut(server_name) {
                    conn.pending.remove(&msg_id);
                }
                Err("MCP 工具调用超时（60 秒）".into())
            }
        }
    }

    /// MCP 工具列表注入到 ToolRegistry
    pub async fn inject_tools(&self, registry: &ToolRegistry) {
        let servers = self.servers.read().await;
        let self_arc = Arc::new(McpClient {
            servers: self.servers.clone(),
            connections: self.connections.clone(),
            tool_registry: self.tool_registry.clone(),
        });
        for server in servers.iter() {
            for tool in &server.tools {
                let tool_name = format!("mcp_{}_{}", server.name, tool.name);
                let tool_def = tool.clone();
                let server_name = server.name.clone();
                let wrapper = McpToolWrapper {
                    server_name,
                    tool_def,
                    client: self_arc.clone(),
                };
                registry.register(tool_name, Arc::new(wrapper) as Arc<dyn super::tools::Tool>).await;
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

    async fn update_server_tools(name: &str, tools: Vec<McpTool>, servers: &Arc<RwLock<Vec<McpServerInfo>>>, registry: &Arc<tokio::sync::RwLock<Option<Arc<ToolRegistry>>>>) {
        {
            let mut srv = servers.write().await;
            if let Some(s) = srv.iter_mut().find(|s| s.name == name) {
                s.tools = tools.clone();
                s.status = "connected".into();
            }
        }
        // 自动注入到 ToolRegistry
        if let Some(ref reg) = *registry.read().await {
            let srv = servers.read().await;
            if srv.iter().any(|s| s.name == name) {
                let mcp_client_arc = Arc::new(McpClient {
                    servers: servers.clone(),
                    connections: Arc::new(RwLock::new(HashMap::new())),
                    tool_registry: registry.clone(),
                });
                for tool in &tools {
                    let tool_name = format!("mcp_{}_{}", name, tool.name);
                    let wrapper = McpToolWrapper {
                        server_name: name.to_string(),
                        tool_def: tool.clone(),
                        client: mcp_client_arc.clone(),
                    };
                    reg.register(tool_name, Arc::new(wrapper) as Arc<dyn super::tools::Tool>).await;
                }
                println!("  🔧 MCP [{}] 已注册 {} 个工具到 Agent", name, tools.len());
            }
        }
    }

    /// SSE 连接循环
    #[allow(unused_variables)]
    async fn run_sse_connection(
        name: &str, url: &str, id: &str,
        servers: Arc<RwLock<Vec<McpServerInfo>>>,
        connections: Arc<RwLock<HashMap<String, McpConnection>>>,
        registry: Arc<tokio::sync::RwLock<Option<Arc<ToolRegistry>>>>,
    ) -> Result<(), String> {
        let http_client = reqwest::Client::new();
        let base_url = url.to_string();

        // 创建连接记录
        let (shutdown_tx, _shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        {
            let mut conns = connections.write().await;
            conns.insert(name.to_string(), McpConnection {
                server_id: id.to_string(),
                msg_id: 0,
                pending: HashMap::new(),
                child: None,
                stdin: None,
                base_url: Some(base_url.clone()),
                http_client: Some(http_client.clone()),
                shutdown: Some(shutdown_tx),
            });
        }

        // 1. SSE 连接
        let resp = http_client.get(&base_url)
            .header("Accept", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .send()
            .await
            .map_err(|e| format!("SSE 连接失败: {}", e))?;

        // 2. 解析 SSE 流
        let stream = resp.bytes_stream();
        let mut buffer = String::new();

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
                            // 处理 initialized 通知
                            if msg.method.as_deref() == Some("initialized") {
                                let list_req = McpMessage::request("tools/list", json!({}), 1u64);
                                let _ = http_client.post(&base_url).json(&list_req).send().await;
                            }
                            // 处理 tools/list_changed 通知 — 重新拉取工具列表
                            if msg.method.as_deref() == Some("notifications/tools/list_changed") {
                                let list_req = McpMessage::request("tools/list", json!({}), 1u64);
                                let _ = http_client.post(&base_url).json(&list_req).send().await;
                            }
                            // 处理 tools/list 结果
                            if let Some(ref result) = msg.result {
                                if let Ok(list) = serde_json::from_value::<McpListToolsResult>(result.clone()) {
                                    Self::update_server_tools(name, list.tools, &servers, &registry).await;
                                }
                            }
                            // 路由到 pending 请求
                            if let Some(id) = msg.id {
                                if id > 1 {
                                    let mut conns = connections.write().await;
                                    if let Some(conn) = conns.get_mut(name) {
                                        if let Some(sender) = conn.pending.remove(&id) {
                                            if let Some(result) = msg.result {
                                                let _ = sender.send(Ok(result));
                                            } else if let Some(error) = msg.error {
                                                let _ = sender.send(Err(format!("MCP 错误 [{}]: {}", error.code, error.message)));
                                            } else {
                                                let _ = sender.send(Err("空响应".into()));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Self::update_server_status(name, "disconnected", None, &servers).await;
        let mut conns = connections.write().await;
        conns.remove(name);
        Ok(())
    }

    /// stdio 连接循环
    async fn run_stdio_connection(
        name: &str, command: &str, args: &[String], id: &str,
        servers: Arc<RwLock<Vec<McpServerInfo>>>,
        connections: Arc<RwLock<HashMap<String, McpConnection>>>,
        registry: Arc<tokio::sync::RwLock<Option<Arc<ToolRegistry>>>>,
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

        let stdin_arc = Arc::new(tokio::sync::Mutex::new(stdin));

        // 创建连接记录
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        {
            let mut conns = connections.write().await;
            conns.insert(name.to_string(), McpConnection {
                server_id: id.to_string(),
                msg_id: 0,
                pending: HashMap::new(),
                child: Some(child),
                stdin: Some(stdin_arc.clone()),
                base_url: None,
                http_client: None,
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
        {
            let mut write_stdin = stdin_arc.lock().await;
            write_stdin.write_all(init_msg.to_json()?.as_bytes()).await.map_err(|e| format!("写入 stdin 失败: {}", e))?;
            write_stdin.write_all(b"\n").await.ok();
        }

        // 读取响应
        let mut initialized = false;

        loop {
            tokio::select! {
                line = reader.next_line() => {
                    match line {
                        Ok(Some(text)) => {
                            if text.trim().is_empty() { continue; }
                            if let Ok(msg) = McpMessage::from_json(&text) {
                                if !initialized && msg.id == Some(1) {
                                    // 处理 initialize 响应
                                    if msg.result.is_some() {
                                        // 发送 initialized 通知
                                        let notif = McpMessage::notification("initialized", json!({}));
                                        {
                                            let mut w = stdin_arc.lock().await;
                                            w.write_all(notif.to_json()?.as_bytes()).await.ok();
                                            w.write_all(b"\n").await.ok();
                                        }
                                        initialized = true;

                                        // 请求工具列表
                                        let list_req = McpMessage::request("tools/list", json!({}), 2u64);
                                        {
                                            let mut w = stdin_arc.lock().await;
                                            w.write_all(list_req.to_json()?.as_bytes()).await.ok();
                                            w.write_all(b"\n").await.ok();
                                        }
                                    }
                                } else if msg.method.as_deref() == Some("notifications/tools/list_changed") {
                                    // 工具列表变更，重新拉取
                                    let list_req = McpMessage::request("tools/list", json!({}), 2u64);
                                    {
                                        let mut w = stdin_arc.lock().await;
                                        w.write_all(list_req.to_json()?.as_bytes()).await.ok();
                                        w.write_all(b"\n").await.ok();
                                    }
                                } else if msg.id == Some(2) {
                                    // tools/list 响应
                                    if let Some(ref result) = msg.result {
                                        if let Ok(list) = serde_json::from_value::<McpListToolsResult>(result.clone()) {
                                            Self::update_server_tools(name, list.tools, &servers, &registry).await;
                                        }
                                    }
                                } else if let Some(id) = msg.id {
                                    // 路由到 pending 请求
                                    let mut conns = connections.write().await;
                                    if let Some(conn) = conns.get_mut(name) {
                                        if let Some(sender) = conn.pending.remove(&id) {
                                            if let Some(result) = msg.result {
                                                let _ = sender.send(Ok(result));
                                            } else if let Some(error) = msg.error {
                                                let _ = sender.send(Err(format!("MCP 错误 [{}]: {}", error.code, error.message)));
                                            } else {
                                                let _ = sender.send(Err("空响应".into()));
                                            }
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
        {
            let mut conns = connections.write().await;
            if let Some(mut child_opt) = conns.get_mut(name).and_then(|c| c.child.take()) {
                let _ = child_opt.kill().await;
            }
            conns.remove(name);
        }

        Ok(())
    }
}

/// MCP 工具包装器 — 使 MCP 工具可被 ToolRegistry 调用
pub struct McpToolWrapper {
    server_name: String,
    tool_def: McpTool,
    client: Arc<McpClient>,
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

    async fn execute(&self, arguments: &Value, _sandbox: &super::security::SecuritySandbox) -> super::tools::ToolResult {
        match self.client.call_tool(&self.server_name, &self.tool_def.name, arguments.clone()).await {
            Ok(output) => super::tools::ToolResult {
                success: true,
                output,
                error: None,
            },
            Err(e) => super::tools::ToolResult {
                success: false,
                output: "".into(),
                error: Some(e),
            },
        }
    }
}
