use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use tokio::sync::RwLock;
use chrono::Utc;
use rusqlite::Connection;

use super::types::*;
use super::llm::{LlmMessage, LlmProvider, MockLlmProvider, OpenAiProvider};
use super::blackboard::SharedBlackboard;
use super::approval::ApprovalQueue;
use super::worker_manager::WorkerManager;
use super::knowledge_base::KnowledgeBase;
use super::security::SecuritySandbox;
use super::host_detector::HostDetector;
use super::mcp_client::McpClient;
use super::tracer::{Tracer, TraceEventType};
use super::browser::BrowserManager;

/// AI 状态与生命周期管理器 - 聚合所有 AI 核心子系统
pub struct AgentManager {
    sessions: Arc<RwLock<Vec<AiSession>>>,
    provider: Arc<tokio::sync::RwLock<Arc<dyn LlmProvider>>>,
    /// 是否配置了真实 LLM（否则为本地 Mock，AI 积木需提示未配置）
    configured: Arc<std::sync::atomic::AtomicBool>,
    blackboard: Arc<SharedBlackboard>,
    approval_queue: Arc<ApprovalQueue>,
    knowledge_base: Arc<KnowledgeBase>,
    worker_manager: Arc<tokio::sync::RwLock<Arc<WorkerManager>>>,
    sandbox: Arc<tokio::sync::RwLock<SecuritySandbox>>,
    host_detector: HostDetector,
    mcp_client: Arc<McpClient>,
    cost_config: Arc<tokio::sync::RwLock<CostConfig>>,
    workspace_root: Arc<tokio::sync::RwLock<PathBuf>>,
    app_handle: Option<tauri::AppHandle>,
    tracer: Arc<Tracer>,
    browser_manager: Arc<BrowserManager>,
    conn: Arc<Mutex<Connection>>,
}

impl AgentManager {
    pub fn new(workspace_root: PathBuf, conn: Arc<Mutex<Connection>>, app_handle: Option<tauri::AppHandle>) -> Self {
        let sandbox = Arc::new(tokio::sync::RwLock::new(SecuritySandbox::new(workspace_root.clone())));
        let blackboard = Arc::new(SharedBlackboard::new());
        let approval_queue = Arc::new(ApprovalQueue::new(app_handle.clone()));
        let cost_config = Arc::new(tokio::sync::RwLock::new(CostConfig {
            input_cached_cost_per_m: 0.025,
            input_uncached_cost_per_m: 2.0,
            output_cost_per_m: 4.0,
            cost_limit: 10.0,
            reasoning_effort: "high".into(),
        }));
        let workspace_root_lock = Arc::new(tokio::sync::RwLock::new(workspace_root.clone()));

        // 默认使用本地 Mock 验证闭环，检测到环境变量则使用 OpenAi 
        let configured = Arc::new(std::sync::atomic::AtomicBool::new(std::env::var("OPENAI_API_KEY").is_ok()));
        let provider_raw: Arc<dyn LlmProvider> = if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            let base_url = std::env::var("OPENAI_BASE_URL").ok();
            let model = std::env::var("OPENAI_MODEL").ok();
            let reasoning_effort = std::env::var("OPENAI_REASONING_EFFORT").ok();
            Arc::new(OpenAiProvider::new(key, base_url, model, reasoning_effort))
        } else {
            Arc::new(MockLlmProvider::new())
        };

        let knowledge_base = Arc::new(KnowledgeBase::new(conn.clone()));
        let tracer = Arc::new(Tracer::new(Some(conn.clone())));

        // 启动时从 SQLite 恢复历史会话
        let persisted_sessions = knowledge_base.load_sessions_blocking();

        let provider: Arc<tokio::sync::RwLock<Arc<dyn LlmProvider>>> = Arc::new(tokio::sync::RwLock::new(provider_raw.clone()));

        let browser_manager = Arc::new(BrowserManager::new(app_handle.clone()));
        let mcp_client = Arc::new(McpClient::new(app_handle.clone()));
        let worker_manager = Arc::new(tokio::sync::RwLock::new(Arc::new(WorkerManager::new(
            provider_raw,
            blackboard.clone(),
            approval_queue.clone(),
            knowledge_base.clone(),
            workspace_root_lock.clone(),
            cost_config.clone(),
            10,
            15,
            app_handle.clone(),
            Some(conn.clone()),
            mcp_client.clone(),
            tracer.clone(),
            browser_manager.clone(),
        ))));

        Self {
            sessions: Arc::new(RwLock::new(persisted_sessions)),
            provider,
            blackboard,
            approval_queue,
            knowledge_base,
            worker_manager,
            sandbox,
            host_detector: HostDetector::new(),
            mcp_client,
            cost_config,
            workspace_root: workspace_root_lock,
            app_handle,
            tracer,
            browser_manager,
            conn,
            configured,
        }
    }

    /// 获取所有会话
    pub async fn get_sessions(&self) -> Vec<AiSession> {
        self.sessions.read().await.clone()
    }

    /// 创建一个新会话（并持久化到 SQLite）
    pub async fn create_session(&self, title: String) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let session_title = title.clone();
        let session = AiSession {
            id: id.clone(),
            title,
            status: SessionStatus::Active,
            project_id: None,
            task_description: String::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            token_count: 0,
            cost_usd: 0.0,
        };
        self.knowledge_base.save_session(&session).await;
        self.sessions.write().await.push(session);

        // 追踪：会话创建
        self.tracer.record(&id, None, TraceEventType::SessionCreated, serde_json::json!({
            "title": session_title,
        })).await;

        id
    }

    /// 更新会话描述（并持久化）
    pub async fn update_session_task(&self, session_id: &str, task_desc: String) -> bool {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.iter_mut().find(|s| s.id == session_id) {
            session.task_description = task_desc;
            session.updated_at = Utc::now();
            let session_clone = session.clone();
            drop(sessions);
            self.knowledge_base.save_session(&session_clone).await;
            true
        } else {
            false
        }
    }

    /// 删除一个会话（从内存和 SQLite 中移除）
    pub async fn delete_session(&self, session_id: &str) -> bool {
        // 从内存中移除
        {
            let mut sessions = self.sessions.write().await;
            sessions.retain(|s| s.id != session_id);
        }
        // 从数据库删除
        self.knowledge_base.delete_session(session_id).await
    }

    pub fn blackboard(&self) -> &Arc<SharedBlackboard> {
        &self.blackboard
    }

    pub fn approval_queue(&self) -> &Arc<ApprovalQueue> {
        &self.approval_queue
    }

    pub fn knowledge_base(&self) -> &Arc<KnowledgeBase> {
        &self.knowledge_base
    }

    pub fn host_detector(&self) -> &HostDetector {
        &self.host_detector
    }

    pub fn mcp_client(&self) -> &Arc<McpClient> {
        &self.mcp_client
    }

    pub fn tracer(&self) -> &Arc<Tracer> {
        &self.tracer
    }

    pub fn browser_manager(&self) -> &Arc<BrowserManager> {
        &self.browser_manager
    }

    pub async fn plan_task(&self, session_id: String, request: String) -> Vec<TaskNode> {
        self.update_session_task(&session_id, request.clone()).await;
        let provider = self.provider.read().await.clone();
        let nodes = super::planner::Planner::plan(provider, self.blackboard.clone(), session_id.clone(), request).await;
        if let Some(ref handle) = self.app_handle {
            use tauri::Emitter;
            for node in &nodes {
                let _ = handle.emit("ai:task-update", node.clone());
            }
        }

        // 追踪：任务规划
        self.tracer.record(&session_id, None, TraceEventType::TaskPlanned, serde_json::json!({
            "task_count": nodes.len(),
            "tasks": nodes.iter().map(|n| serde_json::json!({
                "id": n.id,
                "title": n.title,
                "status": format!("{:?}", n.status),
            })).collect::<Vec<_>>(),
        })).await;

        nodes
    }

    pub fn workspace_root(&self) -> Arc<tokio::sync::RwLock<PathBuf>> {
        self.workspace_root.clone()
    }

    /// 是否已配置真实 LLM（false = 本地 Mock，AI 积木应提示未配置）
    pub fn is_configured(&self) -> bool {
        self.configured.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// AI 积木 / 通用生成：给定 system + user 提示，一次性返回生成文本（复用全局 provider）
    pub async fn generate_text(&self, system: &str, user: &str) -> Result<String, String> {
        let provider = self.provider.read().await.clone();
        let msgs = vec![
            LlmMessage::system(system.to_string()),
            LlmMessage::user(user.to_string()),
        ];
        let resp = provider.chat(&msgs, &[]).await.map_err(|e| format!("AI 调用失败: {}", e))?;
        let content = resp.content.trim().to_string();
        if content.is_empty() {
            return Err("AI 未返回内容".into());
        }
        Ok(content)
    }

    pub async fn set_reasoning_effort(&self, effort: String) {
        let eff = if effort.is_empty() || effort == "off" { None } else { Some(effort) };
        self.provider.read().await.set_reasoning_effort(eff).await;
    }

    /// 运行时从配置重新设置 LLM Provider
    pub async fn reconfigure_llm(&self, provider_name: &str, api_key: &str, base_url: &str, model: &str) {
        let new_provider: Arc<dyn LlmProvider> = Arc::new(OpenAiProvider::new(
            api_key.to_string(),
            Some(base_url.to_string()),
            Some(model.to_string()),
            None,
        ));
        let new_wm = Arc::new(WorkerManager::new(
            new_provider.clone(),
            self.blackboard.clone(),
            self.approval_queue.clone(),
            self.knowledge_base.clone(),
            self.workspace_root.clone(),
            self.cost_config.clone(),
            10, 15,
            self.app_handle.clone(),
            Some(self.conn.clone()),
            self.mcp_client.clone(),
            self.tracer.clone(),
            self.browser_manager.clone(),
        ));
        *self.provider.write().await = new_provider;
        *self.worker_manager.write().await = new_wm;
        self.configured.store(true, std::sync::atomic::Ordering::SeqCst);
        println!("🔄 [Agent] LLM 成功重新配置切换为 {}/{}", provider_name, model);
    }

    pub fn worker_manager(&self) -> &Arc<tokio::sync::RwLock<Arc<WorkerManager>>> {
        &self.worker_manager
    }

    pub fn sandbox(&self) -> &Arc<tokio::sync::RwLock<SecuritySandbox>> {
        &self.sandbox
    }

    pub fn cost_config(&self) -> &Arc<tokio::sync::RwLock<CostConfig>> {
        &self.cost_config
    }

    pub async fn update_workspace_root(&self, new_root: PathBuf) {
        // 同时获取两把写锁，避免竞态窗口导致其他线程读到不一致状态
        let mut root = self.workspace_root.write().await;
        let mut sandbox = self.sandbox.write().await;
        *root = new_root.clone();
        sandbox.update_workspace_root(new_root);
    }
}
