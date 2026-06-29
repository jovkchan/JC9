use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::RwLock;
use chrono::Utc;

use super::types::*;
use super::llm::{LlmProvider, MockLlmProvider, OpenAiProvider};
use super::blackboard::SharedBlackboard;
use super::approval::ApprovalQueue;
use super::worker_manager::WorkerManager;
use super::knowledge_base::KnowledgeBase;
use super::security::SecuritySandbox;
use super::host_detector::HostDetector;
use super::mcp_client::McpClient;

/// AI 状态与生命周期管理器 - 聚合所有 AI 核心子系统
pub struct AgentManager {
    sessions: Arc<RwLock<Vec<AiSession>>>,
    provider: Arc<tokio::sync::RwLock<Arc<dyn LlmProvider>>>,
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
}

impl AgentManager {
    pub fn new(workspace_root: PathBuf, db_path: PathBuf, app_handle: Option<tauri::AppHandle>) -> Self {
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
        let provider_raw: Arc<dyn LlmProvider> = if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            let base_url = std::env::var("OPENAI_BASE_URL").ok();
            let model = std::env::var("OPENAI_MODEL").ok();
            let reasoning_effort = std::env::var("OPENAI_REASONING_EFFORT").ok();
            Arc::new(OpenAiProvider::new(key, base_url, model, reasoning_effort))
        } else {
            Arc::new(MockLlmProvider::new())
        };

        let knowledge_base = Arc::new(KnowledgeBase::new(db_path.clone()));

        // 启动时从 SQLite 恢复历史会话
        let persisted_sessions = knowledge_base.load_sessions_blocking();

        let provider: Arc<tokio::sync::RwLock<Arc<dyn LlmProvider>>> = Arc::new(tokio::sync::RwLock::new(provider_raw.clone()));

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
            mcp_client: Arc::new(McpClient::new()),
            cost_config,
            workspace_root: workspace_root_lock,
            app_handle,
        }
    }

    /// 获取所有会话
    pub async fn get_sessions(&self) -> Vec<AiSession> {
        self.sessions.read().await.clone()
    }

    /// 创建一个新会话（并持久化到 SQLite）
    pub async fn create_session(&self, title: String) -> String {
        let id = uuid::Uuid::new_v4().to_string();
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

    pub async fn plan_task(&self, session_id: String, request: String) -> Vec<TaskNode> {
        self.update_session_task(&session_id, request.clone()).await;
        let provider = self.provider.read().await.clone();
        let nodes = super::planner::Planner::plan(provider, self.blackboard.clone(), session_id, request).await;
        if let Some(ref handle) = self.app_handle {
            use tauri::Emitter;
            for node in &nodes {
                let _ = handle.emit("ai:task-update", node.clone());
            }
        }
        nodes
    }

    pub fn workspace_root(&self) -> Arc<tokio::sync::RwLock<PathBuf>> {
        self.workspace_root.clone()
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
        ));
        *self.provider.write().await = new_provider;
        *self.worker_manager.write().await = new_wm;
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
        {
            let mut root = self.workspace_root.write().await;
            *root = new_root.clone();
        }
        {
            let mut sandbox = self.sandbox.write().await;
            sandbox.update_workspace_root(new_root);
        }
    }
}
