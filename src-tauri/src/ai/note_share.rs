// ══════════════════════════════════════════════════════════════
// 笔记分享 REST API — 独立模块，不耦合 MCP Server
// GET  /api/notes/:id      → JSON
// GET  /api/notes/:id/html → HTML（模板渲染 + 侧边目录）
// ══════════════════════════════════════════════════════════════

use std::sync::Arc;
use axum::{
    extract::{State, Path},
    http::{StatusCode, header},
    response::Html,
    Json,
};
use serde_json::{json, Value};
use rusqlite::{params, Connection};
use tower_http::cors::CorsLayer;
use axum::Router;
use axum::routing::get;
use std::sync::atomic::{AtomicBool, Ordering};
use std::net::SocketAddr;
use serde::{Serialize, Deserialize};

// ═══════════════════════════════════════════════
// 笔记分享服务配置
// ═══════════════════════════════════════════════
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteShareConfig {
    pub port: u16,
    pub host: String,
}

impl Default for NoteShareConfig {
    fn default() -> Self {
        Self { port: 8899, host: "0.0.0.0".into() }
    }
}

// ═══════════════════════════════════════════════
// Upload 目录（与 lib.rs 中一致）
// ═══════════════════════════════════════════════
fn get_upload_base_dir() -> Result<std::path::PathBuf, String> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map_err(|_| "无法获取用户目录".to_string())?;
    Ok(std::path::PathBuf::from(home).join(".jc9").join("Upload"))
}

/// 根据扩展名推断 MIME 类型
fn mime_from_ext(ext: &str) -> &'static str {
    match ext.to_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

/// GET /api/files/:year/:filename — 静态文件服务（Upload 目录）  
pub async fn handle_serve_file(
    Path((year, filename)): Path<(String, String)>,
) -> Result<(StatusCode, axum::http::HeaderMap, Vec<u8>), StatusCode> {
    let base = get_upload_base_dir().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let file_path = base.join(&year).join(&filename);

    // 安全检查：防止路径穿越
    let canonical_base = base.canonicalize().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let canonical_file = file_path.canonicalize().map_err(|_| StatusCode::NOT_FOUND)?;
    if !canonical_file.starts_with(&canonical_base) {
        return Err(StatusCode::FORBIDDEN);
    }

    let data = tokio::fs::read(&file_path).await.map_err(|_| StatusCode::NOT_FOUND)?;

    let ext = std::path::Path::new(&filename)
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_default();
    let mime = mime_from_ext(&ext);

    let mut headers = axum::http::HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, axum::http::HeaderValue::from_str(mime).unwrap());
    Ok((StatusCode::OK, headers, data))
}

// ═══════════════════════════════════════════════
// 笔记分享服务状态
// ═══════════════════════════════════════════════
pub struct NoteShareState {
    pub db_conn: Option<Arc<std::sync::Mutex<Connection>>>,
}

pub struct NoteShareServer {
    pub running: Arc<AtomicBool>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    pub port: u16,
    pub host: String,
    db_conn: Option<Arc<std::sync::Mutex<Connection>>>,
}

impl NoteShareServer {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            shutdown_tx: None,
            port: 8899,
            host: "0.0.0.0".into(),
            db_conn: None,
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn get_config(&self) -> NoteShareConfig {
        NoteShareConfig { port: self.port, host: self.host.clone() }
    }

    pub fn update_config(&mut self, config: &NoteShareConfig) {
        self.port = config.port;
        self.host = config.host.clone();
    }

    pub fn set_db_conn(&mut self, db: Arc<std::sync::Mutex<Connection>>) {
        self.db_conn = Some(db);
    }

    pub async fn start(&mut self) -> Result<(), String> {
        if self.running.load(Ordering::SeqCst) {
            return Err("笔记分享服务已在运行中".into());
        }
        let db = self.db_conn.clone().ok_or("数据库未就绪".to_string())?;
        let host = &self.host;
        let start_port = self.port;

        // 尝试绑定端口：从配置端口开始，失败则 +1 重试，最多试 10 个
        let max_attempts = 10;
        let mut listener = None;
        let mut actual_port = start_port;

        for attempt in 0..max_attempts {
            let port = start_port + attempt;
            let addr: SocketAddr = format!("{}:{}", host, port)
                .parse().map_err(|e| format!("地址格式错误: {}", e))?;
            match tokio::net::TcpListener::bind(addr).await {
                Ok(l) => {
                    listener = Some(l);
                    actual_port = port;
                    break;
                }
                Err(e) => {
                    if attempt < max_attempts - 1 {
                        println!("⚠️  笔记分享端口 {} 被占用，尝试下一个端口... ({})", port, e);
                    } else {
                        return Err(format!("端口 {}-{} 全部被占用，请修改配置或释放端口", start_port, start_port + max_attempts - 1));
                    }
                }
            }
        }

        let listener = listener.unwrap();
        if actual_port != start_port {
            println!("📝 端口 {} 被占用，已自动切换到 {}", start_port, actual_port);
        }

        let shared_state = Arc::new(NoteShareState { db_conn: Some(db) });
        let running = self.running.clone();

        let app = Router::new()
            .route("/api/notes/:id", get(handle_get_note_json))
            .route("/api/notes/:id/html", get(handle_get_note_html))
            .route("/api/files/:year/:filename", get(handle_serve_file))
            .layer(CorsLayer::very_permissive())
            .with_state(shared_state);

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        self.shutdown_tx = Some(shutdown_tx);
        self.running.store(true, Ordering::SeqCst);
        println!("📝 笔记分享服务已启动: http://{}:{}/api/notes/:id", host, actual_port);

        tokio::spawn(async move {
            axum::serve(listener, app.into_make_service())
                .with_graceful_shutdown(async { let _ = shutdown_rx.await; })
                .await.ok();
            running.store(false, Ordering::SeqCst);
            println!("📝 笔记分享服务已停止");
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
}

/// GET /api/notes/:id — 返回笔记 JSON
pub async fn handle_get_note_json(
    State(state): State<Arc<NoteShareState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let db = state.db_conn.as_ref().ok_or((StatusCode::SERVICE_UNAVAILABLE, "数据库未就绪".into()))?;
    let conn = db.lock().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("数据库锁失败: {}", e)))?;

    let note = conn.query_row(
        "SELECT id, title, content, format, tags, group_id, visibility, is_pinned, created_at, updated_at FROM notes WHERE id=?1 AND is_deleted=0",
        params![id],
        |row: &rusqlite::Row<'_>| {
            Ok(json!({
                "id": row.get::<_,String>(0)?,
                "title": row.get::<_,String>(1)?,
                "content": row.get::<_,String>(2)?,
                "format": row.get::<_,String>(3)?,
                "tags": row.get::<_,String>(4)?,
                "groupId": row.get::<_,Option<String>>(5)?,
                "visibility": row.get::<_,String>(6)?,
                "isPinned": row.get::<_,bool>(7)?,
                "createdAt": row.get::<_,String>(8)?,
                "updatedAt": row.get::<_,String>(9)?,
            }))
        },
    ).map_err(|_| (StatusCode::NOT_FOUND, "笔记不存在".into()))?;

    Ok(Json(note))
}

// ── HTML 模板辅助 ──

/// 从 Markdown 文本提取标题生成侧边目录
fn extract_toc(markdown: &str) -> Vec<(usize, String, String)> {
    let mut toc = Vec::new();
    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            let level = trimmed.chars().take_while(|c| *c == '#').count();
            let text = trimmed[level..].trim().to_string();
            let id = slugify(&text);
            toc.push((level, text, id));
        }
    }
    toc
}

/// 将标题文本转为 URL 友好的 slug ID
fn slugify(text: &str) -> String {
    text.to_lowercase()
        .chars().filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '-' || *c == '_')
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}

/// 给渲染后的 HTML heading 标签添加 id 属性
fn add_heading_ids(html: &str) -> String {
    let mut seen = std::collections::HashMap::new();
    let mut result = html.to_string();
    // 逐级处理 h1-h6（不支持反向引用，所以分开匹配）
    for level in 1..=6 {
        let tag = format!("h{}", level);
        let pattern = format!(r#"<{}([^>]*)>(.*?)</{}>"#, tag, tag);
        if let Ok(re) = regex::Regex::new(&pattern) {
            result = re.replace_all(&result, |caps: &regex::Captures| {
                let attrs = &caps[1];
                let inner = &caps[2];
                let plain = regex::Regex::new(r"<[^>]+>").unwrap().replace_all(inner, "");
                let base_id = slugify(&plain);
                let count = seen.entry(base_id.clone()).or_insert(0);
                let id = if *count > 0 { format!("{}-{}", base_id, count) } else { base_id.clone() };
                *count += 1;
                format!("<{} id=\"{}\"{}>{}</{}>", tag, id, attrs, inner, tag)
            }).to_string();
        }
    }
    result
}

/// 将 HTML 中的 src="Upload/..." 映射为 src="/api/files/..."
fn remap_upload_srcs(html: &str) -> String {
    let re = regex::Regex::new(r#"src="(Upload/([^"]+))""#).unwrap();
    re.replace_all(html, r#"src="/api/files/$2""#).to_string()
}

/// 从 HTML 内容提取标题生成 TOC
fn extract_toc_from_html(html: &str) -> Vec<(usize, String, String)> {
    let mut toc = Vec::new();
    // Rust regex 不支持反向引用，逐级匹配 h1-h6
    for level in 1..=6 {
        let pattern = format!(r#"<h{}[^>]*>(.*?)</h{}>"#, level, level);
        if let Ok(re) = regex::Regex::new(&pattern) {
            for caps in re.captures_iter(html) {
                let text = regex::Regex::new(r"<[^>]+>")
                    .unwrap()
                    .replace_all(&caps[1], "")
                    .to_string();
                let id = slugify(&text);
                toc.push((level, text, id));
            }
        }
    }
    toc
}

/// 生成 TOC 的 HTML 树
fn render_toc_html(toc: &[(usize, String, String)]) -> String {
    if toc.is_empty() { return String::new(); }
    let mut html = String::from("<nav class=\"toc\"><ul>");
    let mut stack: Vec<usize> = vec![];
    for (level, text, id) in toc {
        let lvl = *level;
        while stack.last().map_or(false, |last| *last >= lvl) {
            html.push_str("</li></ul>");
            stack.pop();
        }
        if stack.last().map_or(true, |last| *last < lvl) {
            if !stack.is_empty() { html.push_str("<ul>"); }
            stack.push(lvl);
        }
        html.push_str(&format!("<li><a href=\"#{}\">{}</a>", id, text));
    }
    while !stack.is_empty() {
        html.push_str("</li></ul>");
        stack.pop();
    }
    html.push_str("</nav>");
    html
}

/// GET /api/notes/:id/html — 返回笔记 HTML（模板渲染 + 侧边目录）
pub async fn handle_get_note_html(
    State(state): State<Arc<NoteShareState>>,
    Path(id): Path<String>,
) -> Result<Html<String>, (StatusCode, String)> {
    let db = state.db_conn.as_ref().ok_or((StatusCode::SERVICE_UNAVAILABLE, "数据库未就绪".into()))?;
    let conn = db.lock().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("数据库锁失败: {}", e)))?;

    let (title, content, format, created, updated): (String, String, String, String, String) = conn.query_row(
        "SELECT title, content, COALESCE(format,'markdown'), created_at, updated_at FROM notes WHERE id=?1 AND is_deleted=0",
        params![id],
        |row: &rusqlite::Row<'_>| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
    ).map_err(|_| (StatusCode::NOT_FOUND, "笔记不存在".into()))?;

    let (html_body, toc_source) = if format == "html" {
        // HTML 格式：直接使用，但需要：
        // 1. 将 src="Upload/..." 映射为 src="/api/files/..."
        // 2. 提取标题生成 TOC
        let remapped = remap_upload_srcs(&content);
        let toc = extract_toc_from_html(&content);
        (remapped, toc)
    } else {
        // Markdown 格式：pulldown-cmark 渲染
        use pulldown_cmark::{Parser, Options, html};
        let mut opts = Options::empty();
        opts.insert(Options::ENABLE_TABLES);
        opts.insert(Options::ENABLE_TASKLISTS);
        opts.insert(Options::ENABLE_STRIKETHROUGH);
        let parser = Parser::new_ext(&content, opts);
        let mut body = String::new();
        html::push_html(&mut body, parser);
        let toc = extract_toc(&content);
        // Markdown 中也可能有 Upload/ 相对路径图片
        let body = remap_upload_srcs(&body);
        (body, toc)
    };

    let html_body = add_heading_ids(&html_body);
    let toc_html = render_toc_html(&toc_source);
    let has_toc = !toc_source.is_empty();
    let body_class = if has_toc { "" } else { "no-toc" };
    let sun_svg = r#"<svg viewBox="0 0 1024 1024" width="16" height="16" fill="currentColor"><path d="M512.000213 733.353497c-122.06857 0-221.353283-99.284713-221.353283-221.353284S389.931643 290.64693 512.000213 290.64693 733.353497 389.931643 733.353497 512.000213 634.026117 733.353497 512.000213 733.353497z m0-357.373767A136.148482 136.148482 0 0 0 375.97973 512.000213 136.148482 136.148482 0 0 0 512.000213 648.020697 136.148482 136.148482 0 0 0 648.020697 512.000213 136.148482 136.148482 0 0 0 512.000213 375.97973zM554.666613 171.735673A42.154403 42.154403 0 0 1 512.000213 213.335413c-23.551853 0-42.6664-18.645217-42.6664-41.59974V41.603153A42.154403 42.154403 0 0 1 512.000213 0.003413c23.551853 0 42.6664 18.645217 42.6664 41.59974v130.13252zM554.666613 982.397273A42.154403 42.154403 0 0 1 512.000213 1023.997013c-23.594519 0-42.6664-18.687883-42.6664-41.59974v-130.175186A42.111737 42.111737 0 0 1 512.000213 810.665013c23.551853 0 42.6664 18.60255 42.6664 41.59974v130.13252zM171.735673 469.333813c22.954523 0 41.59974 19.114547 41.59974 42.6664 0 23.594519-18.645217 42.6664-41.59974 42.6664H41.603153A42.154403 42.154403 0 0 1 0.003413 512.000213c0-23.551853 18.645217-42.6664 41.59974-42.6664h130.13252zM982.397273 469.333813c22.954523 0 41.59974 19.114547 41.59974 42.6664 0 23.594519-18.687883 42.6664-41.59974 42.6664h-130.175186A42.111737 42.111737 0 0 1 810.665013 512.000213c0-23.551853 18.60255-42.6664 41.59974-42.6664h130.13252zM241.239239 722.430898a42.06907 42.06907 0 0 1 59.562294 0.767995 42.111737 42.111737 0 0 1 0.767996 59.562295l-92.031425 92.074091a42.154403 42.154403 0 0 1-59.562295-0.853328 42.154403 42.154403 0 0 1-0.767995-59.562294l92.031425-91.988759zM814.462323 149.207814a42.154403 42.154403 0 0 1 59.562294 0.767995 42.154403 42.154403 0 0 1 0.767996 59.562295l-92.031425 92.031425a42.06907 42.06907 0 0 1-59.562295-0.767996 42.111737 42.111737 0 0 1-0.810661-59.562294l92.074091-92.031425zM241.239239 301.526862a42.19707 42.19707 0 0 0 59.604961-0.725329 42.111737 42.111737 0 0 0 0.767995-59.562294L209.538104 149.122481a42.154403 42.154403 0 0 0-59.562295 0.853328 42.111737 42.111737 0 0 0-0.767995 59.562295l92.031425 91.988758zM814.462323 874.792613a42.111737 42.111737 0 0 0 59.562294-0.810662 42.154403 42.154403 0 0 0 0.767996-59.562294l-92.031425-92.031425a42.06907 42.06907 0 0 0-59.562295 0.767995 42.111737 42.111737 0 0 0-0.810661 59.562294l92.074091 92.074092z"/></svg>"#;
    let moon_svg = r#"<svg viewBox="0 0 1024 1024" width="16" height="16" fill="currentColor"><path d="M644.5056 70.528C834.4064 127.488 972.8 303.5648 972.8 512c0 254.4896-206.3104 460.8-460.8 460.8-222.4128 0-408.0128-157.568-451.2768-367.1296A433.4848 433.4848 0 0 0 230.4 640c240.3584 0 435.2-194.8416 435.2-435.2 0-44.2112-6.5792-86.8608-18.8416-127.0528z"/></svg>"#;
    let layout_svg = r#"<svg viewBox="0 0 1024 1024" width="16" height="16" fill="currentColor"><path d="M716.8 194.56A112.64 112.64 0 0 1 829.44 307.2v409.6a112.64 112.64 0 0 1-112.64 112.64H307.2A112.64 112.64 0 0 1 194.56 716.8V307.2A112.64 112.64 0 0 1 307.2 194.56h409.6zM542.72 256v512H716.8l5.2224-0.256a51.2 51.2 0 0 0 45.7216-45.7216L768 716.8V307.2a51.2 51.2 0 0 0-45.9776-50.944L716.8 256h-174.08zM307.2 256a51.2 51.2 0 0 0-51.2 51.2v409.6l0.256 5.2224A51.2 51.2 0 0 0 307.2 768h174.08V256H307.2z"/></svg>"#;
    let page = format!(r#"<!DOCTYPE html>
<html lang="zh-CN" data-theme="light" data-layout="mode1">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1.0">
<title>{title} - JC9</title>
<style>
  * {{ margin:0; padding:0; box-sizing:border-box; }}

  /* ── 主题变量 ── */
  :root {{
    --bg-page: #f8f9fa;
    --bg-card: #fff;
    --bg-toc: #fff;
    --bg-code: #f4f5f6;
    --bg-code-inline: #f0f0f0;
    --bg-blockquote: #fafafa;
    --bg-hover: #f0f0f0;
    --text-primary: #1a1a1a;
    --text-secondary: #333;
    --text-muted: #666;
    --text-dim: #999;
    --text-toc: #555;
    --border-color: #eee;
    --border-strong: #e8e8e8;
    --border-table: #ddd;
    --link-color: #2563eb;
    --toolbar-bg: rgba(255,255,255,0.92);
    --toolbar-border: #ddd;
  }}
  [data-theme="dark"] {{
    --bg-page: #0d1117;
    --bg-card: #161b22;
    --bg-toc: #161b22;
    --bg-code: #1c2333;
    --bg-code-inline: #1c2333;
    --bg-blockquote: #1c2333;
    --bg-hover: #2a2f3a;
    --text-primary: #e6e6e6;
    --text-secondary: #c9d1d9;
    --text-muted: #8b949e;
    --text-dim: #6e7681;
    --text-toc: #8b949e;
    --border-color: #30363d;
    --border-strong: #30363d;
    --border-table: #30363d;
    --link-color: #58a6ff;
    --toolbar-bg: rgba(22,27,34,0.92);
    --toolbar-border: #30363d;
  }}

  body {{ font-family:-apple-system,BlinkMacSystemFont,'Segoe UI','Noto Sans SC',Roboto,sans-serif; color:var(--text-primary); background:var(--bg-page); line-height:1.7; font-size:15px; }}
  .layout {{ display:flex; min-height:100vh; transition:max-width 0.2s; position:relative; }}

  /* 模式1：居中 */
  [data-layout="mode1"] .layout {{ max-width:1200px; margin:0 auto; }}
  [data-layout="mode1"] .toc-sidebar {{ display:block; }}
  [data-layout="mode1"] .main-content {{ max-width:none; }}

  /* 模式2：目录靠左，内容自适应，不留白 */
  [data-layout="mode2"] .layout {{ max-width:none; margin:0; }}
  [data-layout="mode2"] .toc-sidebar {{ display:block; }}
  [data-layout="mode2"] .main-content {{ max-width:none; }}

  /* 无目录时隐藏侧栏 */
  .no-toc .toc-sidebar {{ display:none !important; }}
  [data-layout="mode2"].no-toc .layout {{ max-width:900px; margin:0 auto; }}

  .toc-sidebar {{ width:260px; min-width:260px; padding:32px 16px; position:sticky; top:0; height:100vh; overflow-y:auto; background:var(--bg-toc); border-right:1px solid var(--border-strong); }}
  .toc-sidebar .toc-title {{ font-size:11px; font-weight:600; color:var(--text-dim); text-transform:uppercase; letter-spacing:1px; margin-bottom:12px; }}
  .toc-sidebar ul {{ list-style:none; padding-left:0; }}
  .toc-sidebar li {{ margin:2px 0; }}
  .toc-sidebar a {{ display:block; padding:4px 8px; font-size:13px; color:var(--text-toc); text-decoration:none; border-radius:4px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }}
  .toc-sidebar a:hover {{ background:var(--bg-hover); color:var(--text-primary); }}
  .toc-sidebar ul ul {{ padding-left:14px; }}
  .toc-sidebar ul ul a {{ font-size:12px; }}
  .toc-sidebar::-webkit-scrollbar {{ width:4px; }}
  .toc-sidebar::-webkit-scrollbar-thumb {{ background:var(--border-strong); border-radius:2px; }}

  .main-content {{ flex:1; min-width:0; padding:40px 48px; background:var(--bg-card); }}
  .main-content h1 {{ font-size:1.8em; font-weight:700; margin-bottom:4px; }}
  .main-content .meta {{ font-size:12px; color:var(--text-dim); margin-bottom:28px; }}
  .main-content h2 {{ font-size:1.35em; font-weight:600; margin-top:32px; margin-bottom:12px; padding-bottom:6px; border-bottom:1px solid var(--border-color); }}
  .main-content h3 {{ font-size:1.15em; font-weight:600; margin-top:24px; margin-bottom:8px; color:var(--text-secondary); }}
  .main-content h4 {{ font-size:1.05em; font-weight:600; margin-top:20px; margin-bottom:6px; color:var(--text-secondary); }}
  .main-content p {{ margin-bottom:14px; color:var(--text-secondary); }}
  .main-content a {{ color:var(--link-color); text-decoration:none; }}
  .main-content a:hover {{ text-decoration:underline; }}
  .main-content ul,.main-content ol {{ margin-bottom:14px; padding-left:24px; }}
  .main-content li {{ margin-bottom:4px; }}
  .main-content pre {{ background:var(--bg-code); padding:14px 16px; border-radius:6px; overflow-x:auto; font-size:13px; line-height:1.5; margin-bottom:14px; border:1px solid var(--border-strong); }}
  .main-content code {{ font-family:'Cascadia Code','JetBrains Mono',Consolas,monospace; font-size:13px; background:var(--bg-code-inline); padding:2px 5px; border-radius:3px; }}
  .main-content pre code {{ background:transparent; padding:0; border-radius:0; }}
  .main-content blockquote {{ border-left:3px solid var(--border-strong); margin:14px 0; padding:4px 16px; color:var(--text-muted); background:var(--bg-blockquote); }}
  .main-content img {{ max-width:100%; border-radius:4px; margin:8px 0; }}
  .main-content table {{ border-collapse:collapse; width:100%; margin-bottom:14px; font-size:13px; }}
  .main-content th,.main-content td {{ border:1px solid var(--border-table); padding:8px 12px; }}
  .main-content th {{ background:var(--bg-page); font-weight:600; }}
  .main-content hr {{ border:none; border-top:1px solid var(--border-color); margin:24px 0; }}
  .footer {{ margin-top:40px; padding-top:16px; border-top:1px solid var(--border-color); font-size:11px; color:var(--text-dim); text-align:center; }}

  /* ── 浮动工具栏 ── */
  .toolbar {{ position:fixed; top:12px; right:12px; z-index:1000; display:flex; gap:4px; background:var(--toolbar-bg); border:1px solid var(--toolbar-border); border-radius:8px; padding:4px; backdrop-filter:blur(6px); box-shadow:0 2px 8px rgba(0,0,0,0.08); }}
  .toolbar button {{ width:32px; height:32px; border:none; border-radius:6px; background:transparent; color:var(--text-muted); cursor:pointer; font-size:15px; display:flex; align-items:center; justify-content:center; transition:all 0.15s; }}
  .toolbar button:hover {{ background:var(--bg-hover); color:var(--text-primary); }}
  .toolbar button svg {{ width:16px; height:16px; display:block; }}

  @media (max-width:800px) {{ .toc-sidebar {{ display:none !important; }} .main-content {{ padding:24px 20px; }} .toolbar {{ top:8px; right:8px; }} }}
</style>
</head>
<body class="{body_class}">
<div class="toolbar">
  <button id="btnTheme" title="切换明暗主题" onclick="toggleTheme()">{moon_svg}</button>
  <button id="btnLayout" title="切换布局" onclick="toggleLayout()">{layout_svg}</button>
</div>
<div class="layout">
  <aside class="toc-sidebar">
    <div class="toc-title">目录</div>
    {toc_html}
  </aside>
  <main class="main-content">
    <h1>{title}</h1>
    <div class="meta">创建于 {created} · 更新于 {updated}</div>
    <div id="content">{html_body}</div>
    <div class="footer">分享来自：<a href="https://JC9.BYCHY.COM">JC9</a> · © 2026 JC9</div>
  </main>
</div>
<script>
(function(){{
  // 从 localStorage 恢复偏好
  const savedTheme = localStorage.getItem('jc9_note_theme') || 'light';
  const savedLayout = localStorage.getItem('jc9_note_layout') || 'mode1';
  document.documentElement.setAttribute('data-theme', savedTheme);
  document.documentElement.setAttribute('data-layout', savedLayout);
  document.getElementById('btnTheme').innerHTML = savedTheme === 'dark' ? `{sun_svg}` : `{moon_svg}`;
  document.getElementById('btnLayout').innerHTML = `{layout_svg}`;
}})();
function toggleTheme() {{
  const html = document.documentElement;
  const theme = html.getAttribute('data-theme') === 'dark' ? 'light' : 'dark';
  html.setAttribute('data-theme', theme);
  localStorage.setItem('jc9_note_theme', theme);
  document.getElementById('btnTheme').innerHTML = theme === 'dark' ? `{sun_svg}` : `{moon_svg}`;
}}
function toggleLayout() {{
  const html = document.documentElement;
  const layout = html.getAttribute('data-layout') === 'mode2' ? 'mode1' : 'mode2';
  html.setAttribute('data-layout', layout);
  localStorage.setItem('jc9_note_layout', layout);
}}
</script>
</body>
</html>"#);
    Ok(Html(page))
}
