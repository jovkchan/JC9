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
    provider: Arc<dyn LlmProvider>,
    blackboard: Arc<SharedBlackboard>,
    approval_queue: Arc<ApprovalQueue>,
    knowledge_base: Arc<KnowledgeBase>,
    worker_manager: Arc<WorkerManager>,
    sandbox: Arc<tokio::sync::RwLock<SecuritySandbox>>,
    host_detector: HostDetector,
    mcp_client: Arc<McpClient>,
    cost_config: Arc<tokio::sync::RwLock<CostConfig>>,
    workspace_root: Arc<tokio::sync::RwLock<PathBuf>>,
}

impl AgentManager {
    pub fn new(workspace_root: PathBuf) -> Self {
        let sandbox = Arc::new(tokio::sync::RwLock::new(SecuritySandbox::new(workspace_root.clone())));
        let blackboard = Arc::new(SharedBlackboard::new());
        let approval_queue = Arc::new(ApprovalQueue::new());
        let cost_config = Arc::new(tokio::sync::RwLock::new(CostConfig {
            input_cached_cost_per_m: 0.025,
            input_uncached_cost_per_m: 3.0,
            output_cost_per_m: 6.0,
            cost_limit: 5.0,
        }));
        let workspace_root_lock = Arc::new(tokio::sync::RwLock::new(workspace_root.clone()));

        // 默认使用本地 Mock 验证闭环，检测到环境变量则使用 OpenAi 
        let provider: Arc<dyn LlmProvider> = if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            let base_url = std::env::var("OPENAI_BASE_URL").ok();
            let model = std::env::var("OPENAI_MODEL").ok();
            Arc::new(OpenAiProvider::new(key, base_url, model))
        } else {
            Arc::new(MockLlmProvider::new())
        };

        let worker_manager = Arc::new(WorkerManager::new(
            provider.clone(),
            blackboard.clone(),
            approval_queue.clone(),
            workspace_root_lock.clone(),
            cost_config.clone(),
            4,   // 限制最大 4 并发 Worker
            15,  // 允许的最大迭代轮次
        ));

        Self {
            sessions: Arc::new(RwLock::new(Vec::new())),
            provider,
            blackboard,
            approval_queue,
            knowledge_base: Arc::new(KnowledgeBase::new(workspace_root.join("jc9_knowledge.db"))),
            worker_manager,
            sandbox,
            host_detector: HostDetector::new(),
            mcp_client: Arc::new(McpClient::new()),
            cost_config,
            workspace_root: workspace_root_lock,
        }
    }

    /// 获取所有会话
    pub async fn get_sessions(&self) -> Vec<AiSession> {
        self.sessions.read().await.clone()
    }

    /// 创建一个新会话
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
        self.sessions.write().await.push(session);
        id
    }

    /// 更新会话描述
    pub async fn update_session_task(&self, session_id: &str, task_desc: String) -> bool {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.iter_mut().find(|s| s.id == session_id) {
            session.task_description = task_desc;
            session.updated_at = Utc::now();
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

    pub fn worker_manager(&self) -> &Arc<WorkerManager> {
        &self.worker_manager
    }

    pub fn host_detector(&self) -> &HostDetector {
        &self.host_detector
    }

    pub fn mcp_client(&self) -> &Arc<McpClient> {
        &self.mcp_client
    }

    pub async fn plan_task(&self, session_id: String, request: String) -> Vec<TaskNode> {
        self.update_session_task(&session_id, request.clone()).await;
        super::planner::Planner::plan(self.provider.clone(), self.blackboard.clone(), session_id, request).await
    }

    pub fn workspace_root(&self) -> Arc<tokio::sync::RwLock<PathBuf>> {
        self.workspace_root.clone()
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
