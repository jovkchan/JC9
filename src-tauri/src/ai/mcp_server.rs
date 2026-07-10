use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::net::SocketAddr;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tokio::net::TcpListener;
use tokio_stream::wrappers::ReceiverStream;
use tokio::sync::mpsc;
use axum::{
    extract::{State, Query},
    http::{StatusCode, HeaderMap},
    response::sse::{Event, Sse},
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};
use chrono::Utc;
use futures_util::stream::Stream;
use rusqlite::{params, Connection};

use super::mcp_types::*;
use super::knowledge_base::KnowledgeBase;
use super::types::*;

// ══════════════════════════════════════════════════════════════
// MCP Server 配置
// ══════════════════════════════════════════════════════════════

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpServerConfig {
    pub enabled: bool,
    pub port: u16,
    #[serde(alias = "apiKey")]
    pub api_key: String,
    pub host: String,
    #[serde(default)]
    #[serde(alias = "groupIds")]
    pub group_ids: Vec<String>,
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            port: 19799,
            api_key: uuid::Uuid::new_v4().to_string(),
            host: "127.0.0.1".into(),
            group_ids: vec![],
        }
    }
}

// ══════════════════════════════════════════════════════════════
// MCP Server — 暴露笔记操作给外部 AI Agent
// ══════════════════════════════════════════════════════════════

pub struct McpServer {
    config: Arc<RwLock<McpServerConfig>>,
    knowledge_base: Option<Arc<KnowledgeBase>>,
    db_conn: Option<Arc<std::sync::Mutex<Connection>>>,
    running: Arc<AtomicBool>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    sse_clients: Arc<RwLock<HashMap<String, mpsc::Sender<Result<Event, String>>>>>,
    pub group_ids_lock: Option<Arc<RwLock<Vec<String>>>>,
}

impl McpServer {
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(McpServerConfig::default())),
            knowledge_base: None,
            db_conn: None,
            running: Arc::new(AtomicBool::new(false)),
            shutdown_tx: None,
            sse_clients: Arc::new(RwLock::new(HashMap::new())),
            group_ids_lock: None,
        }
    }

    pub fn set_knowledge_base(&mut self, kb: Arc<KnowledgeBase>) {
        self.knowledge_base = Some(kb);
    }

    pub fn set_db_conn(&mut self, conn: Arc<std::sync::Mutex<Connection>>) {
        self.db_conn = Some(conn);
    }

    pub async fn update_config(&self, config: McpServerConfig) {
        *self.config.write().await = config;
    }

    pub async fn get_config(&self) -> McpServerConfig {
        self.config.read().await.clone()
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub async fn start(&mut self) -> Result<(), String> {
        if self.running.load(Ordering::SeqCst) {
            return Err("MCP Server 已在运行中".into());
        }
        let config = self.config.read().await.clone();
        if !config.enabled {
            return Err("MCP Server 未启用".into());
        }

        let host = config.host.clone();
        let start_port = config.port;
        let api_key = config.api_key.clone();
        let group_ids = config.group_ids.clone();

        // 尝试绑定端口：从配置端口开始，失败则 +1 重试，最多试 10 个
        let max_attempts = 10;
        let mut listener = None;
        let mut actual_port = start_port;

        for attempt in 0..max_attempts {
            let port = start_port + attempt;
            let addr: SocketAddr = format!("{}:{}", host, port)
                .parse().map_err(|e| format!("地址格式错误: {}", e))?;
            match TcpListener::bind(addr).await {
                Ok(l) => {
                    listener = Some(l);
                    actual_port = port;
                    break;
                }
                Err(e) => {
                    if attempt < max_attempts - 1 {
                        println!("⚠️  端口 {} 被占用，尝试下一个端口... ({})", port, e);
                    } else {
                        return Err(format!("端口 {}-{} 全部被占用，请修改配置或释放端口", start_port, start_port + max_attempts - 1));
                    }
                }
            }
        }

        let listener = listener.unwrap();

        // 如果实际端口与配置不同，只在内存中更新（不持久化，下次启动仍用原配置）
        if actual_port != start_port {
            self.config.write().await.port = actual_port;
            println!("🧠 端口 {} 被占用，已自动切换到 {}", start_port, actual_port);
            println!("   (配置文件中仍为 {}，下次启动仍尝试此端口)", start_port);
        }

        let kb = self.knowledge_base.clone();
        let db = self.db_conn.clone();
        let sse_clients = self.sse_clients.clone();
        let running = self.running.clone();

        let gids = Arc::new(RwLock::new(group_ids));
        self.group_ids_lock = Some(gids.clone());
        let shared_state = Arc::new(AppState {
            knowledge_base: kb,
            db_conn: db,
            sse_clients,
            api_key: Arc::new(api_key),
            group_ids: gids,
        });

        let app = Router::new()
            .route("/sse", get(handle_sse).post(handle_message))
            .route("/message", post(handle_message))
            .with_state(shared_state);

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        self.shutdown_tx = Some(shutdown_tx);
        self.running.store(true, Ordering::SeqCst);
        println!("🧠 JC9 MCP Server 已启动: http://{}:{}/sse", host, actual_port);

        tokio::spawn(async move {
            axum::serve(listener, app.into_make_service())
                .with_graceful_shutdown(async { let _ = shutdown_rx.await; })
                .await.ok();
            running.store(false, Ordering::SeqCst);
            println!("🧠 JC9 MCP Server 已停止");
        });
        Ok(())
    }

    pub async fn stop(&mut self) {
        if self.running.load(Ordering::SeqCst) {
            if let Some(tx) = self.shutdown_tx.take() {
                let _ = tx.send(());
            }
            self.running.store(false, Ordering::SeqCst);
        }
    }

    pub async fn restart(&mut self) -> Result<(), String> {
        self.stop().await;
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        self.start().await
    }
}

// ══════════════════════════════════════════════════════════════
// Axum 共享状态
// ══════════════════════════════════════════════════════════════

#[derive(Clone)]
struct AppState {
    knowledge_base: Option<Arc<KnowledgeBase>>,
    db_conn: Option<Arc<std::sync::Mutex<Connection>>>,
    sse_clients: Arc<RwLock<HashMap<String, mpsc::Sender<Result<Event, String>>>>>,
    api_key: Arc<String>,
    group_ids: Arc<RwLock<Vec<String>>>,
}

// ══════════════════════════════════════════════════════════════
// SSE 端点
// ══════════════════════════════════════════════════════════════

async fn handle_sse(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Sse<impl Stream<Item = Result<Event, String>>>, StatusCode> {
    // 支持 Header 认证和 URL 查询参数认证
    let auth_from_header = headers.get("authorization").and_then(|v| v.to_str().ok()).unwrap_or("");
    let auth_from_param = params.get("api_key").map(|s| format!("Bearer {}", s)).unwrap_or_default();
    let auth = if !auth_from_header.is_empty() { auth_from_header } else { &auth_from_param };
    if auth != format!("Bearer {}", state.api_key) && auth != state.api_key.as_str() {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let client_id = uuid::Uuid::new_v4().to_string();
    let (tx, rx) = mpsc::channel::<Result<Event, String>>(100);
    state.sse_clients.write().await.insert(client_id.clone(), tx.clone());
    let _ = tx.send(Ok(Event::default().event("endpoint").data("/message"))).await;
    let _ = tx.send(Ok(Event::default().event("initialized").data("{}"))).await;
    Ok(Sse::new(ReceiverStream::new(rx)))
}

// ══════════════════════════════════════════════════════════════
// Message 端点 — JSON-RPC
// ══════════════════════════════════════════════════════════════

async fn handle_message(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> (StatusCode, Json<Value>) {
    let msg: McpMessage = match serde_json::from_value(body) {
        Ok(m) => m,
        Err(e) => return (StatusCode::OK, Json(json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":format!("JSON 解析错误: {}", e)}}))),
    };

    let method = msg.method.as_deref().unwrap_or("");
    let msg_id = msg.id;

    let result = match method {
        "initialize" => Ok(json!({"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"jc9-mcp-server","version":"1.0.0"}})),
        "ping" => Ok(json!({})),
        "tools/list" => handle_tools_list().await,
        "tools/call" => handle_tools_call(&state, &msg).await,
        "notifications/initialized" => return (StatusCode::OK, Json(json!({"jsonrpc":"2.0"}))),
        _ => {
            if let Some(id) = msg_id {
                return (StatusCode::OK, Json(json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":format!("未知方法:{}",method)}})));
            }
            return (StatusCode::OK, Json(json!({"jsonrpc":"2.0"})));
        }
    };

    match result {
        Ok(response) => {
            if let Some(id) = msg_id {
                (StatusCode::OK, Json(json!({"jsonrpc":"2.0","id":id,"result":response})))
            } else {
                (StatusCode::OK, Json(json!({"jsonrpc":"2.0"})))
            }
        }
        Err(err_msg) => {
            if let Some(id) = msg_id {
                (StatusCode::OK, Json(json!({"jsonrpc":"2.0","id":id,"error":{"code":-32000,"message":err_msg}})))
            } else {
                (StatusCode::OK, Json(json!({"jsonrpc":"2.0"})))
            }
        }
    }
}

// ══════════════════════════════════════════════════════════════
// 工具列表 — 只暴露笔记操作
// ══════════════════════════════════════════════════════════════

async fn handle_tools_list() -> Result<Value, String> {
    Ok(json!({
        "tools": [
            {
                "name": "jc9_note_search",
                "description": "搜索笔记。先用向量语义搜索(sqlite-vec)理解查询意图，再结合关键词排序，返回最匹配的笔记列表(含ID/标题/内容预览/匹配分数)。典型用法：用户提问项目相关问题时先调用此工具搜索笔记，从结果中获取笔记ID后调用jc9_note_read读取全文。支持中文自然语言描述。",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": {"type":"string","description":"搜索关键词或自然语言描述，例如'项目怎么启动'、'数据库配置'"},
                        "limit": {"type":"integer","description":"可选：返回数量上限，默认5"},
                        "groupId": {"type":"string","description":"可选：按分组ID过滤，ID从jc9_note_groups获取"}
                    },
                    "required": ["query"]
                }
            },
            {
                "name": "jc9_note_read",
                "description": "读取指定笔记的完整内容（含标题和正文Markdown）。先用jc9_note_search或jc9_note_list找到笔记获取其ID，再用此工具读取全文。返回的content字段可直接作为回答依据。",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": {"type":"string","description":"笔记ID，从jc9_note_search或jc9_note_list的返回结果中获取"}
                    },
                    "required": ["id"]
                }
            },
            {
                "name": "jc9_note_create",
                "description": "创建新笔记。标题和内容为必填。创建后自动同步到知识库并生成向量嵌入，后续可通过jc9_note_search搜索到。返回新建笔记的ID。\n\n📝 笔记编写标准（务必遵守）：\n1. 结构先行：使用 # 标题分层（H1→H2→H3），每个 ## 章节围绕一个独立子主题，确保信息自解释\n2. 元数据驱动：正文以 YAML Frontmatter 开头声明 id/type/tags（如 type: Guide, tags: [Vue,性能]）\n3. 表达清晰：段落首句概括主旨；并列信息用 - 列表；技术内容用 ` 代码块包裹\n4. 关键词密度：关键术语保留高频词（如 sqlite-vec、混合检索），配合 FTS5 全文搜索提升命中率\n5. 标签：通过 tags 参数传入分类标签，格式 [\"架构\",\"配置\"]",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "title": {"type":"string","description":"笔记标题"},
                        "content": {"type":"string","description":"笔记正文内容（支持Markdown格式）"},
                        "groupId": {"type":"string","description":"可选：所属分组ID，从jc9_note_groups获取；留空则放入默认分组"},
                        "tags": {"type":"array","items":{"type":"string"},"description":"可选：标签列表，例如[\"架构\",\"配置\"]"}
                    },
                    "required": ["title","content"]
                }
            },
            {
                "name": "jc9_note_update_title",
                "description": "更新已有笔记的标题。需要笔记ID（从jc9_note_search或jc9_note_list获取）和新标题。",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": {"type":"string","description":"笔记ID，从jc9_note_search或jc9_note_list返回结果中获取"},
                        "title": {"type":"string","description":"新标题"}
                    },
                    "required": ["id","title"]
                }
            },
            {
                "name": "jc9_note_list",
                "description": "列出所有笔记（可按分组过滤）。返回笔记列表含ID/标题/内容预览/更新时间。可用于浏览全部笔记或获取特定分组的笔记列表。",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "groupId": {"type":"string","description":"可选：按分组ID过滤，只列出该分组下的笔记；ID从jc9_note_groups获取"},
                        "limit": {"type":"integer","description":"可选：返回数量上限，默认50"}
                    },
                    "required": []
                }
            },
            {
                "name": "jc9_note_groups",
                "description": "获取所有笔记分组列表。返回分组ID和名称，可用于其他工具（如jc9_note_list、jc9_note_create）的groupId参数。",
                "inputSchema": {"type":"object","properties":{},"required":[]}
            },
            {
                "name": "jc9_database_stats",
                "description": "获取数据库诊断统计：knowledge、embeddings、vec_embeddings、knowledge_fts 表的行数。用于确认向量索引是否正常工作。",
                "inputSchema": {"type":"object","properties":{},"required":[]}
            },
            {
                "name": "jc9_reindex",
                "description": "重建全部知识条目的向量嵌入。遍历 knowledge 表，逐条重新生成 n-gram 词袋向量并写入 embeddings 和 vec_embeddings 表。执行耗时与条目数成正比。",
                "inputSchema": {"type":"object","properties":{},"required":[]}
            }
        ]
    }))
}

// ══════════════════════════════════════════════════════════════
// 工具调用路由
// ══════════════════════════════════════════════════════════════

async fn handle_tools_call(state: &Arc<AppState>, msg: &McpMessage) -> Result<Value, String> {
    let params = msg.params.as_ref().ok_or("缺少参数")?;
    let tool_name = params["name"].as_str().ok_or("缺少工具名称")?;
    let args = params["arguments"].clone();
    let args = if args.is_null() { json!({}) } else { args };

    match tool_name {
        "jc9_note_search" => cmd_note_search(state, &args).await,
        "jc9_note_read" => cmd_note_read(state, &args).await,
        "jc9_note_create" => cmd_note_create(state, &args).await,
        "jc9_note_update_title" => cmd_note_update_title(state, &args).await,
        "jc9_note_list" => cmd_note_list(state, &args).await,
        "jc9_note_groups" => cmd_note_groups(state).await,
        "jc9_database_stats" => cmd_database_stats(state).await,
        "jc9_reindex" => cmd_reindex(state).await,
        _ => Err(format!("未知工具: {}", tool_name)),
    }
}

// ══════════════════════════════════════════════════════════════
// 辅助函数
// ══════════════════════════════════════════════════════════════

fn get_db(state: &Arc<AppState>) -> Result<Arc<std::sync::Mutex<Connection>>, String> {
    state.db_conn.clone().ok_or("数据库未初始化".into())
}

fn get_kb(state: &Arc<AppState>) -> Result<Arc<KnowledgeBase>, String> {
    state.knowledge_base.clone().ok_or("知识库未初始化".into())
}

/// 检查分组是否在白名单内（白名单为空则不限）
/// 支持继承：如果笔记分组的任一祖先在白名单中，也视为允许
fn group_allowed(whitelist: &[String], group_id: Option<&str>, conn: &Connection) -> bool {
    if whitelist.is_empty() { return true; }
    let mut current = match group_id {
        Some(gid) => gid.to_string(),
        None => return false,
    };
    for _ in 0..20 {
        if whitelist.iter().any(|id| id == &current) {
            return true;
        }
        let parent: Option<String> = conn.query_row(
            "SELECT parent_id FROM note_groups WHERE id = ?1",
            params![current],
            |row| row.get(0),
        ).ok().flatten();
        match parent {
            Some(p) => current = p,
            None => break,
        }
    }
    false
}

fn read_note_from_db(conn: &Connection, note_id: &str) -> Result<Option<Value>, String> {
    let sql = if note_id.len() < 32 {
        // 短 ID（如搜索预览的 8 位前缀）：用 LIKE 匹配
        "SELECT id,group_id,title,content,format,is_pinned,tags,visibility,created_at,updated_at,is_archived
         FROM notes WHERE id LIKE ?1 AND is_deleted=0 LIMIT 1"
    } else {
        "SELECT id,group_id,title,content,format,is_pinned,tags,visibility,created_at,updated_at,is_archived
         FROM notes WHERE id=?1 AND is_deleted=0"
    };
    let param = if note_id.len() < 32 {
        format!("{}%", note_id)
    } else {
        note_id.to_string()
    };
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

    match stmt.query_row(params![param], |row| {
        let tags_str: String = row.get::<_,String>(6).unwrap_or_default();
        let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
        Ok(json!({
            "id": row.get::<_,String>(0).unwrap_or_default(),
            "groupId": row.get::<_,Option<String>>(1).unwrap_or(None),
            "title": row.get::<_,String>(2).unwrap_or_default(),
            "content": row.get::<_,String>(3).unwrap_or_default(),
            "format": row.get::<_,String>(4).unwrap_or_default(),
            "isPinned": row.get::<_,i32>(5).unwrap_or(0)!=0,
            "tags": tags,
            "createdAt": row.get::<_,String>(8).unwrap_or_default(),
            "updatedAt": row.get::<_,String>(9).unwrap_or_default(),
            "isArchived": row.get::<_,i32>(10).unwrap_or(0)!=0,
        }))
    }) {
        Ok(note) => Ok(Some(note)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

fn list_notes_from_db(conn: &Connection, group_id: Option<&str>, limit: usize) -> Result<Vec<Value>, String> {
    let sql: &str;
    let mut stmt;
    if let Some(gid) = group_id {
        sql = "SELECT id,group_id,title,content,format,is_pinned,tags,created_at,updated_at
               FROM notes WHERE group_id=?1 AND is_deleted=0 ORDER BY updated_at DESC LIMIT ?2";
        stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(rusqlite::params![gid, limit as i64], |row| {
            note_row_to_json(row)
        }).map_err(|e| e.to_string())?;
        let mut notes = Vec::new();
        for r in rows { notes.push(r.map_err(|e| e.to_string())?); }
        Ok(notes)
    } else {
        sql = "SELECT id,group_id,title,content,format,is_pinned,tags,created_at,updated_at
               FROM notes WHERE is_deleted=0 ORDER BY updated_at DESC LIMIT ?1";
        stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(rusqlite::params![limit as i64], |row| {
            note_row_to_json(row)
        }).map_err(|e| e.to_string())?;
        let mut notes = Vec::new();
        for r in rows { notes.push(r.map_err(|e| e.to_string())?); }
        Ok(notes)
    }
}

fn note_row_to_json(row: &rusqlite::Row) -> rusqlite::Result<Value> {
    let tags_str: String = row.get::<_,String>(6).unwrap_or_default();
    let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
    let content: String = row.get::<_,String>(3).unwrap_or_default();
    Ok(json!({
        "id": row.get::<_,String>(0).unwrap_or_default(),
        "groupId": row.get::<_,Option<String>>(1).unwrap_or(None),
        "title": row.get::<_,String>(2).unwrap_or_default(),
        "contentPreview": content.chars().take(200).collect::<String>(),
        "tags": tags,
        "createdAt": row.get::<_,String>(7).unwrap_or_default(),
        "updatedAt": row.get::<_,String>(8).unwrap_or_default(),
    }))
}

fn list_groups_from_db(conn: &Connection) -> Result<Vec<Value>, String> {
    let mut stmt = conn.prepare("SELECT id,name,parent_id,sort_order FROM note_groups ORDER BY sort_order")
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        Ok(json!({"id":row.get::<_,String>(0).unwrap_or_default(),"name":row.get::<_,String>(1).unwrap_or_default()}))
    }).map_err(|e| e.to_string())?;
    let mut groups = Vec::new();
    for r in rows { groups.push(r.map_err(|e| e.to_string())?); }
    Ok(groups)
}

// ══════════════════════════════════════════════════════════════
// cmd_note_search — 向量语义搜索 + 关键词
// ══════════════════════════════════════════════════════════════

async fn cmd_note_search(state: &Arc<AppState>, args: &Value) -> Result<Value, String> {
    let kb = get_kb(state)?;
    let db = get_db(state)?;
    let query = args["query"].as_str().ok_or("缺少 query 参数")?;
    let limit = args["limit"].as_i64().unwrap_or(5) as usize;
    let group_id = args["groupId"].as_str().map(|s| s.to_string());
    let q = query.to_string();
    let l = limit;

    // 1. 向量语义搜索 + FTS5 关键词搜索 并行执行
    let (semantic, keyword) = tokio::join!(
        kb.semantic_search(&q, l * 2),
        kb.search(&q, l * 2)
    );

    // 2. RRF (Reciprocal Rank Fusion) 合并 — 比加权求和更鲁棒
    //    score = Σ 1/(k + rank_i)，k=60
    const RRF_K: f32 = 60.0;
    let mut rrf: std::collections::HashMap<String, (f32, &str)> = std::collections::HashMap::new();

    for (rank, (sid, _, _)) in semantic.iter().enumerate() {
        if let Some(nid) = sid.strip_prefix("note_") {
            let s = 1.0 / (RRF_K + rank as f32 + 1.0);
            rrf.entry(nid.to_string())
                .and_modify(|(sc, src)| { *sc += s; *src = "hybrid"; })
                .or_insert((s, "vector"));
        }
    }
    for (rank, entry) in keyword.iter().enumerate() {
        if let Some(nid) = entry.id.strip_prefix("note_") {
            let s = 1.0 / (RRF_K + rank as f32 + 1.0);
            rrf.entry(nid.to_string())
                .and_modify(|(sc, src)| { *sc += s; *src = "hybrid"; })
                .or_insert((s, "keyword"));
        }
    }

    // 按 RRF 分排序，归一化到 [0,1]
    let max_rrf = if rrf.is_empty() { 1.0 } else { rrf.values().map(|(s, _)| *s).fold(0.0f32, f32::max) };
    let mut ranked: Vec<(String, f32, &str)> = rrf.into_iter()
        .map(|(id, (score, src))| (id, if max_rrf > 0.0 { score / max_rrf } else { 0.0 }, src))
        .collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    ranked.truncate(l * 3);

    // 3. 先读取白名单（异步），再锁 DB
    let whitelist = state.group_ids.read().await.clone();
    let mut results: Vec<Value> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        for (nid, score, source) in &ranked {
            if seen.contains(nid) { continue; }
            if let Some(mut note) = read_note_from_db(&conn, nid)? {
                let note_gid = note["groupId"].as_str();
                let in_group = group_id.as_deref().map_or_else(
                    || group_allowed(&whitelist, note_gid, &conn),
                    |gid| note_gid == Some(gid)
                );
                if in_group && !note.get("isArchived").and_then(|v| v.as_bool()).unwrap_or(false) {
                    seen.insert(nid.clone());
                    note["score"] = json!(score);
                    note["matchSource"] = json!(source);
                    results.push(note);
                }
            }
        }
    }

    // 如果混合检索结果不足，回头多取一些向量结果补全
    if results.len() < l && semantic.len() > ranked.len() {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        for (sid, score, _) in &semantic {
            if results.len() >= l { break; }
            if let Some(nid) = sid.strip_prefix("note_") {
                if seen.contains(nid) { continue; }
                if let Some(mut note) = read_note_from_db(&conn, nid)? {
                    let note_gid = note["groupId"].as_str();
                    let in_group = group_id.as_deref().map_or_else(
                        || group_allowed(&whitelist, note_gid, &conn),
                        |gid| note_gid == Some(gid)
                    );
                    if in_group && !note.get("isArchived").and_then(|v| v.as_bool()).unwrap_or(false) {
                        seen.insert(nid.to_string());
                        note["score"] = json!(score);
                        note["matchSource"] = json!("vector_fallback");
                        results.push(note);
                    }
                }
            }
        }
    }

    results.truncate(l);

    if results.is_empty() {
        return Ok(json!({"content":[{"type":"text","text":"未找到匹配的笔记。请尝试其他关键词，或使用 jc9_note_create 创建新笔记。"}],"results":[],"total":0}));
    }

    let mut lines = vec![format!("搜索到 {} 条匹配笔记：\n", results.len())];
    for r in &results {
        let title = r["title"].as_str().unwrap_or("无标题");
        let nid = &r["id"].as_str().unwrap_or("")[..8.min(r["id"].as_str().unwrap_or("").len())];
        let score = r["score"].as_f64().unwrap_or(0.0);
        let source = r.get("matchSource").and_then(|s| s.as_str()).unwrap_or("?");
        let preview = r.get("content").and_then(|c| c.as_str()).map(|c| {
            let trimmed = c.replace('\n', " ").chars().take(100).collect::<String>();
            if trimmed.chars().count() == 100 { format!("{}...", trimmed.chars().take(97).collect::<String>()) } else { trimmed }
        }).unwrap_or_default();
        lines.push(format!("  [{nid}] {title} (匹配度:{:.2}, 来源:{source})", score));
        lines.push(format!("      {preview}"));
    }
    lines.push("\n使用 jc9_note_read 传入笔记ID 读取完整内容。".into());

    Ok(json!({"content":[{"type":"text","text":lines.join("\n")}],"results":results,"total":results.len()}))
}

// ══════════════════════════════════════════════════════════════
// cmd_note_read — 读笔记
// ══════════════════════════════════════════════════════════════

async fn cmd_note_read(state: &Arc<AppState>, args: &Value) -> Result<Value, String> {
    let db = get_db(state)?;
    let note_id = args["id"].as_str().ok_or("缺少 id 参数")?;
    let conn = db.lock().unwrap_or_else(|e| e.into_inner());
    let note = read_note_from_db(&conn, note_id)?.ok_or_else(|| format!("笔记不存在: {}", note_id))?;
    let title = note["title"].as_str().unwrap_or("");
    let content = note["content"].as_str().unwrap_or("");
    let text = format!("# {title}\n\n{content}\n\n---\nID: {note_id}");
    Ok(json!({"content":[{"type":"text","text":text}],"note":note}))
}

// ══════════════════════════════════════════════════════════════
// cmd_note_create — 新建笔记（自动同步知识库+向量）
// ══════════════════════════════════════════════════════════════

async fn cmd_note_create(state: &Arc<AppState>, args: &Value) -> Result<Value, String> {
    let db = get_db(state)?;
    let kb = get_kb(state)?;
    let title = args["title"].as_str().ok_or("缺少 title 参数")?;
    let content = args["content"].as_str().ok_or("缺少 content 参数")?;
    // 未指定 groupId 时取白名单第一个分组，白名单为空则不设分组
    let group_id = args["groupId"].as_str().map(|s| s.to_string()).or_else(|| state.group_ids.blocking_read().first().map(|s| s.clone()));
    let group_id = group_id.as_deref();
    let tags: Vec<String> = args["tags"].as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    let note_id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let tags_json = serde_json::to_string(&tags).unwrap_or_default();

    let conn = db.lock().unwrap_or_else(|e| e.into_inner());
    conn.execute(
        "INSERT INTO notes(id,user_id,group_id,title,content,format,is_pinned,tags,visibility,sort_order,version,is_deleted,is_archived,created_at,updated_at)
         VALUES(?1,'local',?2,?3,?4,'markdown',0,?5,'PRIVATE',0,1,0,0,?6,?6)",
        params![note_id, group_id, title, content, tags_json, now],
    ).map_err(|e| format!("创建笔记失败: {}", e))?;
    drop(conn);

    // 异步同步知识库
    let kb = kb.clone();
    let nid = note_id.clone();
    let t = title.to_string();
    let c = content.to_string();
    tokio::spawn(async move {
        kb.add_entry(KbEntry {
            id: format!("note_{}", nid), title: t, content: c,
            entry_type: KbEntryType::ConfigNote, tags,
            source_session: None, confidence: 0.0, is_draft: false,
            created_at: Utc::now(), updated_at: Utc::now(), embedding: None,
        }).await;
    });

    Ok(json!({"content":[{"type":"text","text":format!("✅ 笔记已创建，ID: {}",note_id)}],"id":note_id}))
}

// ══════════════════════════════════════════════════════════════
// cmd_note_update_title — 更新标题
// ══════════════════════════════════════════════════════════════

async fn cmd_note_update_title(state: &Arc<AppState>, args: &Value) -> Result<Value, String> {
    let db = get_db(state)?;
    let note_id = args["id"].as_str().ok_or("缺少 id 参数")?;
    let new_title = args["title"].as_str().ok_or("缺少 title 参数")?;
    let now = Utc::now().to_rfc3339();

    // 先读内容（释放锁），再更新标题
    let content = {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        read_note_from_db(&conn, note_id)?.and_then(|n| n["content"].as_str().map(String::from))
    };
    if content.is_none() {
        return Err("笔记不存在或已删除".into());
    }

    let conn = db.lock().unwrap_or_else(|e| e.into_inner());
    conn.execute(
        "UPDATE notes SET title=?1,updated_at=?2 WHERE id=?3 AND is_deleted=0",
        params![new_title, now, note_id],
    ).map_err(|e| format!("更新标题失败: {}", e))?;
    drop(conn);

    // 异步更新知识库
    let kb = get_kb(state)?;
    let nid = note_id.to_string();
    let nt = new_title.to_string();
    if let Some(c) = content {
        tokio::spawn(async move {
            kb.remove_entry(&format!("note_{}", nid)).await;
            kb.add_entry(KbEntry {
                id: format!("note_{}", nid), title: nt,
                content: c,
                entry_type: KbEntryType::ConfigNote,
                tags: vec![],
                source_session: None, confidence: 0.0, is_draft: false,
                created_at: Utc::now(), updated_at: Utc::now(), embedding: None,
            }).await;
        });
    }

    Ok(json!({"content":[{"type":"text","text":format!("✅ 标题已更新: {}",new_title)}]}))
}

// ══════════════════════════════════════════════════════════════
// cmd_note_list — 列表
// ══════════════════════════════════════════════════════════════

async fn cmd_note_list(state: &Arc<AppState>, args: &Value) -> Result<Value, String> {
    let db = get_db(state)?;
    let group_id = args["groupId"].as_str().map(|s| s.to_string());
    let limit = args["limit"].as_i64().unwrap_or(50) as usize;

    // 先读取白名单（异步），再锁 DB（同步），避免 MutexGuard 跨 await
    let whitelist = state.group_ids.read().await.clone();
    let conn = db.lock().unwrap_or_else(|e| e.into_inner());

    let notes = if group_id.is_some() {
        list_notes_from_db(&conn, group_id.as_deref(), limit)?
    } else if whitelist.is_empty() {
        list_notes_from_db(&conn, None::<&str>, limit)?
    } else {
        let mut all = Vec::new();
        for gid in &whitelist {
            all.append(&mut list_notes_from_db(&conn, Some(gid), limit)?);
        }
        all.sort_by(|a, b| b["updatedAt"].as_str().unwrap_or("").cmp(&a["updatedAt"].as_str().unwrap_or("")));
        all.truncate(limit);
        all
    };

    if notes.is_empty() {
        return Ok(json!({"content":[{"type":"text","text":"当前分组下没有笔记。请使用 jc9_note_create 创建新笔记，或使用 jc9_note_search 通过关键词搜索已有笔记。"}],"notes":[],"total":0}));
    }

    let mut lines = vec![format!("共 {} 条笔记：\n", notes.len())];
    for n in &notes {
        let title = n["title"].as_str().unwrap_or("无标题");
        let nid = &n["id"].as_str().unwrap_or("")[..8.min(n["id"].as_str().unwrap_or("").len())];
        let updated = &n["updatedAt"].as_str().unwrap_or("")[..10];
        lines.push(format!("  [{nid}] {title} ({updated})"));
    }
    lines.push("\n提示：使用 jc9_note_read 传入笔记ID 读取全文，或用 jc9_note_search 进行语义搜索。".into());

    Ok(json!({"content":[{"type":"text","text":lines.join("\n")}],"notes":notes,"total":notes.len()}))
}

// ══════════════════════════════════════════════════════════════
// cmd_note_groups — 分组列表
// ══════════════════════════════════════════════════════════════

async fn cmd_note_groups(state: &Arc<AppState>) -> Result<Value, String> {
    let db = get_db(state)?;
    let conn = db.lock().unwrap_or_else(|e| e.into_inner());
    let groups = list_groups_from_db(&conn)?;
    Ok(json!({"content":[{"type":"text","text":serde_json::to_string_pretty(&groups).unwrap_or_default()}],"groups":groups,"total":groups.len()}))
}

// ══════════════════════════════════════════════════════════════
// cmd_database_stats — 数据库诊断统计
// ══════════════════════════════════════════════════════════════

async fn cmd_database_stats(state: &Arc<AppState>) -> Result<Value, String> {
    let db = get_db(state)?;
    let conn = db.lock().unwrap_or_else(|e| e.into_inner());
    let count = |table: &str| -> i64 {
        conn.query_row(&format!("SELECT COUNT(*) FROM {}", table), [], |r| r.get(0)).unwrap_or(-1)
    };
    let note_count: i64 = conn.query_row("SELECT COUNT(*) FROM notes WHERE is_deleted=0", [], |r| r.get(0)).unwrap_or(0);
    let stats = json!({
        "knowledge": count("knowledge"),
        "embeddings": count("embeddings"),
        "vec_embeddings": count("vec_embeddings"),
        "knowledge_fts": count("knowledge_fts"),
        "notes": note_count
    });
    Ok(json!({
        "content": [{"type": "text", "text": serde_json::to_string_pretty(&stats).unwrap_or_default()}],
        "stats": stats
    }))
}

// ══════════════════════════════════════════════════════════════
// cmd_reindex — 重建全部向量嵌入
// ══════════════════════════════════════════════════════════════

async fn cmd_reindex(state: &Arc<AppState>) -> Result<Value, String> {
    let kb = get_kb(state)?;
    match kb.reindex_all().await {
        Ok(n) => Ok(json!({
            "content": [{"type": "text", "text": format!("✅ 重建完成: {} 条向量已生成", n)}],
            "indexed": n
        })),
        Err(e) => Err(format!("重建失败: {}", e)),
    }
}
