use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json::{json, Value};
use async_trait::async_trait;
use tauri::{Manager, WebviewWindow, webview::PageLoadEvent};
use url::Url;

use super::tools::{Tool, ToolDefinition, ToolResult};
use super::security::SecuritySandbox;
use super::types::RiskLevel;

type WryWindow = WebviewWindow<tauri::Wry>;

/// 浏览器会话状态
#[derive(Debug, Clone)]
enum BrowserWindowState {
    Closed,
    Open { label: String, url: String },
}

/// 浏览器管理器 — Agent 控制一个可见的浏览器窗口
///
/// 每次页面加载完成后自动通过 on_page_load 重新注入浮动地址栏。
pub struct BrowserManager {
    state: Arc<RwLock<BrowserWindowState>>,
    app_handle: Option<tauri::AppHandle>,
}

impl BrowserManager {
    pub fn new(app_handle: Option<tauri::AppHandle>) -> Self {
        Self {
            state: Arc::new(RwLock::new(BrowserWindowState::Closed)),
            app_handle,
        }
    }

    fn get_window(&self, handle: &tauri::AppHandle, label: &str) -> Option<WryWindow> {
        handle.get_webview_window(label)
    }

    /// 生成地址栏注入脚本 — 浮动覆盖层，每次页面加载后自动注入
    fn address_bar_js(current_url: &str) -> String {
        let safe = current_url
            .replace('\\', "\\\\").replace('\'', "\\'")
            .replace('\n', "\\n");
        format!(r#"(function(){{
  if(document.getElementById('__jc_bar'))return;
  var h=36,b=document.createElement('div');
  b.id='__jc_bar';
  b.style.cssText='display:flex;align-items:center;gap:5px;padding:4px 8px;background:#1a1a2e;border-bottom:1px solid #333;font-size:13px;position:fixed;top:0;left:0;right:0;z-index:2147483647;font-family:sans-serif;height:'+h+'px;box-shadow:0 2px 8px rgba(0,0,0,.5)';
  b.innerHTML='<span style="color:#888;font-size:11px;flex-shrink:0">🌐</span>'+
    '<input id="__jc_u" value="{safe}" style="flex:1;background:#0d0d1a;border:1px solid #333;border-radius:4px;padding:4px 8px;color:#e0e0e0;font-size:13px;outline:none;font-family:monospace" onfocus="this.select()" onkeydown="if(event.key==\'Enter\')n()"/>'+
    '<button style="background:#4a6cf7;color:#fff;border:none;border-radius:4px;padding:4px 12px;cursor:pointer;font-size:12px;flex-shrink:0" onclick="n()">前往</button>'+
    '<button style="background:transparent;color:#888;border:none;cursor:pointer;font-size:14px;padding:2px 4px;flex-shrink:0" onclick="var x=document.getElementById(\'__jc_bar\');if(x)x.style.display=x.style.display==\'none\'?\'flex\':\'none\'" title="隐藏">🔼</button>';
  document.body.prepend(b);
  var p=document.createElement('div');p.style.height=(h+4)+'px';document.body.prepend(p);
  window.__jc_n=function(){{var i=document.getElementById('__jc_u');if(!i)return;var u=i.value.trim();if(!u.startsWith('http'))u='https://'+u;window.location.href=u;i.value=u}};
  window.n=n;
  function n(){{window.__jc_n()}}
  if(window.__jc_poll)clearInterval(window.__jc_poll);
  window.__jc_poll=setInterval(function(){{if(!document.getElementById('__jc_bar')){{location.reload()}}}},3000);
}})();
"#)
    }

    /// 导航到指定 URL
    pub async fn navigate(&self, url_str: &str) -> Result<String, String> {
        let label = "jc9-browser-agent";
        let is_new = {
            let state = self.state.read().await;
            matches!(*state, BrowserWindowState::Closed)
        };

        if is_new {
            if let Some(ref handle) = self.app_handle {
                let parsed = Url::parse(url_str).map_err(|e| format!("URL 解析失败: {}", e))?;
                let window: WryWindow = tauri::WebviewWindowBuilder::new(
                    handle, label,
                    tauri::WebviewUrl::External(parsed),
                )
                .title("JC9 浏览器助手")
                .inner_size(1100.0, 800.0)
                .resizable(true)
                .on_page_load(move |w, payload| {
                    if payload.event() == PageLoadEvent::Finished {
                        let current = payload.url().as_str();
                        let js = Self::address_bar_js(current);
                        let _ = w.eval(&js);
                    }
                })
                .build()
                .map_err(|e| format!("创建浏览器窗口失败: {}", e))?;

                let _ = window.set_focus();
                *self.state.write().await = BrowserWindowState::Open {
                    label: label.to_string(),
                    url: url_str.to_string(),
                };
                Ok(format!("已打开浏览器窗口并导航到 {}", url_str))
            } else {
                Err("AppHandle 不可用".into())
            }
        } else {
            let current_state = self.state.read().await;
            if let BrowserWindowState::Open { label: ref lbl, .. } = *current_state {
                if let Some(ref handle) = self.app_handle {
                    if let Some(window) = self.get_window(handle, lbl) {
                        let safe = url_str.replace('\\', "\\\\").replace('\'', "\\'");
                        let _ = Self::eval_on_window(&window, &format!(
                            "window.location.href='{}';", safe
                        ));
                        let _ = window.set_focus();
                        let lbl_c = lbl.clone();
                        drop(current_state);
                        *self.state.write().await = BrowserWindowState::Open {
                            label: lbl_c, url: url_str.to_string(),
                        };
                        return Ok(format!("已导航到 {}", url_str));
                    }
                }
            }
            Err("窗口状态异常".into())
        }
    }

    /// 点击元素
    pub async fn click(&self, selector: &str) -> Result<String, String> {
        let sel = selector.replace('\\', "\\\\").replace('\'', "\\'");
        self.exec_js(&format!(
            r#"(function(){{var el=document.querySelector('{}');if(!el)return'not found: {}';el.click();return'clicked'}})()"#,
            sel, selector
        )).await
    }

    /// 输入文本
    pub async fn type_text(&self, selector: &str, text: &str) -> Result<String, String> {
        let sel = selector.replace('\\', "\\\\").replace('\'', "\\'");
        let txt = text.replace('\\', "\\\\").replace('\'', "\\'").replace('\n', "\\n");
        self.exec_js(&format!(
            r#"(function(){{var el=document.querySelector('{}');if(!el)return'not found: {}';el.value='{}';el.dispatchEvent(new Event('input',{{bubbles:true}}));el.dispatchEvent(new Event('change',{{bubbles:true}}));return'typed'}})()"#,
            sel, selector, txt
        )).await
    }

    /// 获取页面 HTML
    pub async fn get_html(&self) -> Result<String, String> {
        self.exec_js("document.documentElement.outerHTML").await
    }

    /// 获取页面可见文本
    pub async fn get_text(&self) -> Result<String, String> {
        self.exec_js("document.body.innerText").await
    }

    /// 获取页面标题
    pub async fn get_title(&self) -> Result<String, String> {
        self.exec_js("document.title").await
    }

    /// 截图
    pub async fn screenshot(&self) -> Result<String, String> {
        let title = self.get_title().await.unwrap_or_default();
        let url = {
            let state = self.state.read().await;
            match &*state {
                BrowserWindowState::Open { url, .. } => url.clone(),
                _ => "unknown".into(),
            }
        };
        let text = self.get_text().await.unwrap_or_default();
        let preview = if text.len() > 500 { format!("{}...", &text[..500]) } else { text };
        Ok(format!("【浏览器截图】\n标题: {}\nURL: {}\n页面内容:\n{}", title, url, preview))
    }

    /// 关闭浏览器窗口
    pub async fn close(&self) -> Result<String, String> {
        let state = self.state.read().await;
        match &*state {
            BrowserWindowState::Open { label, .. } => {
                if let Some(ref handle) = self.app_handle {
                    if let Some(window) = self.get_window(handle, label) {
                        let _: Result<(), _> = window.close();
                    }
                }
                drop(state);
                *self.state.write().await = BrowserWindowState::Closed;
                Ok("浏览器窗口已关闭".into())
            }
            BrowserWindowState::Closed => Ok("浏览器窗口已经是关闭状态".into()),
        }
    }

    fn eval_on_window(win: &WryWindow, js: &str) -> Result<(), String> {
        win.eval(js).map_err(|e| format!("浏览器 JS 执行失败: {}", e))
    }

    async fn exec_js(&self, js: &str) -> Result<String, String> {
        let state = self.state.read().await;
        match &*state {
            BrowserWindowState::Closed => Err("浏览器窗口未打开".into()),
            BrowserWindowState::Open { label, .. } => {
                if let Some(ref handle) = self.app_handle {
                    if let Some(window) = self.get_window(handle, label) {
                        let req_id = uuid::Uuid::new_v4().to_string();
                        let marker_val = format!("JC9_VAL_{}:", req_id);
                        let marker_err = format!("JC9_ERR_{}:", req_id);

                        let js_code = format!(
                            r#"(function(){{
                                var old = document.title;
                                try {{
                                    var res = (function(){{ return ({}); }})();
                                    var res_str = typeof res === 'string' ? res : JSON.stringify(res);
                                    document.title = "{}" + res_str;
                                }} catch(e) {{
                                    document.title = "{}" + e.toString();
                                }}
                                setTimeout(function() {{
                                    if (document.title.indexOf("{}") === 0 || document.title.indexOf("{}") === 0) {{
                                        document.title = old;
                                    }}
                                }}, 200);
                            }})();"#,
                            js, marker_val, marker_err, marker_val, marker_err
                        );

                        window.eval(&js_code).map_err(|e| format!("浏览器 JS 执行失败: {}", e))?;

                        let start_time = std::time::Instant::now();
                        loop {
                            if start_time.elapsed().as_secs() > 5 {
                                return Err("浏览器 JS 执行超时 (5s)".into());
                            }
                            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                            let current_title = window.title().unwrap_or_default();
                            if current_title.starts_with(&marker_val) {
                                return Ok(current_title[marker_val.len()..].to_string());
                            }
                            if current_title.starts_with(&marker_err) {
                                return Err(current_title[marker_err.len()..].to_string());
                            }
                        }
                    } else {
                        Err("浏览器窗口已关闭".into())
                    }
                } else {
                    Err("AppHandle 不可用".into())
                }
            }
        }
    }
}

/// Browser 工具包装器 — 实现 Tool trait 供 Agent 调用
pub struct BrowserNavigateTool {
    manager: Arc<BrowserManager>,
}

impl BrowserNavigateTool {
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for BrowserNavigateTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "browser_navigate".into(),
            description: "[Browser] 打开浏览器窗口并导航到指定 URL。弹出一个可见的浏览器窗口，用户可以看到操控过程。".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "要导航到的完整 URL（需包含协议，如 https://）"
                    }
                },
                "required": ["url"]
            }),
            risk_level: RiskLevel::Medium,
        }
    }

    async fn execute(&self, arguments: &Value, _sandbox: &SecuritySandbox) -> ToolResult {
        let url = arguments.get("url").and_then(|v| v.as_str()).unwrap_or("");
        if url.is_empty() {
            return ToolResult { success: false, output: "".into(), error: Some("缺少 url 参数".into()) };
        }
        match self.manager.navigate(url).await {
            Ok(output) => ToolResult { success: true, output, error: None },
            Err(e) => ToolResult { success: false, output: "".into(), error: Some(e) },
        }
    }
}

pub struct BrowserClickTool {
    manager: Arc<BrowserManager>,
}

impl BrowserClickTool {
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for BrowserClickTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "browser_click".into(),
            description: "[Browser] 在浏览器页面中点击指定 CSS 选择器的元素。用户可以看到点击动画。".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS 选择器，如 '#submit-btn'、'.nav-link'、'button:contains(登录)'"
                    }
                },
                "required": ["selector"]
            }),
            risk_level: RiskLevel::Medium,
        }
    }

    async fn execute(&self, arguments: &Value, _sandbox: &SecuritySandbox) -> ToolResult {
        let selector = arguments.get("selector").and_then(|v| v.as_str()).unwrap_or("");
        if selector.is_empty() {
            return ToolResult { success: false, output: "".into(), error: Some("缺少 selector 参数".into()) };
        }
        match self.manager.click(selector).await {
            Ok(output) => ToolResult { success: true, output, error: None },
            Err(e) => ToolResult { success: false, output: "".into(), error: Some(e) },
        }
    }
}

pub struct BrowserTypeTool {
    manager: Arc<BrowserManager>,
}

impl BrowserTypeTool {
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for BrowserTypeTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "browser_type".into(),
            description: "[Browser] 在浏览器页面的指定输入框中输入文本。用户可以看到输入过程。".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "输入框的 CSS 选择器，如 '#search-input'、'input[name=\"q\"]'"
                    },
                    "text": {
                        "type": "string",
                        "description": "要输入的文本内容"
                    }
                },
                "required": ["selector", "text"]
            }),
            risk_level: RiskLevel::Medium,
        }
    }

    async fn execute(&self, arguments: &Value, _sandbox: &SecuritySandbox) -> ToolResult {
        let selector = arguments.get("selector").and_then(|v| v.as_str()).unwrap_or("");
        let text = arguments.get("text").and_then(|v| v.as_str()).unwrap_or("");
        if selector.is_empty() {
            return ToolResult { success: false, output: "".into(), error: Some("缺少 selector 参数".into()) };
        }
        match self.manager.type_text(selector, text).await {
            Ok(output) => ToolResult { success: true, output, error: None },
            Err(e) => ToolResult { success: false, output: "".into(), error: Some(e) },
        }
    }
}

pub struct BrowserGetHtmlTool {
    manager: Arc<BrowserManager>,
}

impl BrowserGetHtmlTool {
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for BrowserGetHtmlTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "browser_get_html".into(),
            description: "[Browser] 获取浏览器当前页面的完整 HTML 内容。用于分析页面结构和提取数据。".into(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            risk_level: RiskLevel::Low,
        }
    }

    async fn execute(&self, _arguments: &Value, _sandbox: &SecuritySandbox) -> ToolResult {
        match self.manager.get_html().await {
            Ok(output) => ToolResult { success: true, output, error: None },
            Err(e) => ToolResult { success: false, output: "".into(), error: Some(e) },
        }
    }
}

pub struct BrowserGetTextTool {
    manager: Arc<BrowserManager>,
}

impl BrowserGetTextTool {
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for BrowserGetTextTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "browser_get_text".into(),
            description: "[Browser] 获取浏览器当前页面的可见文本内容。比 get_html 更简洁，适合快速了解页面内容。".into(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            risk_level: RiskLevel::Low,
        }
    }

    async fn execute(&self, _arguments: &Value, _sandbox: &SecuritySandbox) -> ToolResult {
        match self.manager.get_text().await {
            Ok(output) => ToolResult { success: true, output, error: None },
            Err(e) => ToolResult { success: false, output: "".into(), error: Some(e) },
        }
    }
}

pub struct BrowserScreenshotTool {
    manager: Arc<BrowserManager>,
}

impl BrowserScreenshotTool {
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for BrowserScreenshotTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "browser_screenshot".into(),
            description: "[Browser] 获取浏览器当前页面的快照（文字描述 + 页面标题和 URL）。用于了解页面当前渲染状态。".into(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            risk_level: RiskLevel::Low,
        }
    }

    async fn execute(&self, _arguments: &Value, _sandbox: &SecuritySandbox) -> ToolResult {
        match self.manager.screenshot().await {
            Ok(output) => ToolResult { success: true, output, error: None },
            Err(e) => ToolResult { success: false, output: "".into(), error: Some(e) },
        }
    }
}

pub struct BrowserCloseTool {
    manager: Arc<BrowserManager>,
}

impl BrowserCloseTool {
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for BrowserCloseTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "browser_close".into(),
            description: "[Browser] 关闭浏览器窗口。".into(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            risk_level: RiskLevel::Low,
        }
    }

    async fn execute(&self, _arguments: &Value, _sandbox: &SecuritySandbox) -> ToolResult {
        match self.manager.close().await {
            Ok(output) => ToolResult { success: true, output, error: None },
            Err(e) => ToolResult { success: false, output: "".into(), error: Some(e) },
        }
    }
}
