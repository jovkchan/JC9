use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use std::collections::HashMap;
use chrono::Utc;
use std::path::PathBuf;

use super::types::*;
use super::llm::LlmProvider;
use super::tools::ToolRegistry;
use super::blackboard::SharedBlackboard;
use super::react_loop::ReActLoop;
use super::loop_breaker::LoopBreaker;
use super::approval::ApprovalQueue;
use super::workspace::WorkspaceManager;
use super::security::SecuritySandbox;

/// Worker 管理器 - 并发控制、Worker 生命周期与 COW 隔离环境调度
pub struct WorkerManager {
    workers: Arc<RwLock<HashMap<String, WorkerState>>>,
    semaphore: Arc<Semaphore>,
    provider: Arc<dyn LlmProvider>,
    blackboard: Arc<SharedBlackboard>,
    approval_queue: Arc<ApprovalQueue>,
    workspace_root: Arc<tokio::sync::RwLock<PathBuf>>,
    cost_config: Arc<tokio::sync::RwLock<CostConfig>>,
    max_iterations: u32,
}

impl WorkerManager {
    pub fn new(
        provider: Arc<dyn LlmProvider>,
        blackboard: Arc<SharedBlackboard>,
        approval_queue: Arc<ApprovalQueue>,
        workspace_root: Arc<tokio::sync::RwLock<PathBuf>>,
        cost_config: Arc<tokio::sync::RwLock<CostConfig>>,
        max_concurrent: usize,
        max_iterations: u32,
    ) -> Self {
        Self {
            workers: Arc::new(RwLock::new(HashMap::new())),
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            provider, blackboard, approval_queue, workspace_root, cost_config, max_iterations,
        }
    }

    /// 拉起一个并发 Worker 并绑定 COW 隔离开发空间
    pub async fn spawn_worker(&self, session_id: String, task: TaskNode, system_prompt: String) -> Result<String, String> {
        let _permit = self.semaphore.acquire().await.map_err(|e| e.to_string())?;
        let worker_id = uuid::Uuid::new_v4().to_string();

        // 1. 初始化 COW 临时沙箱
        let current_root = self.workspace_root.read().await.clone();
        let workspace_mgr = Arc::new(WorkspaceManager::new(current_root));
        let temp_workspace = workspace_mgr.prepare_sandbox()?;

        // 2. 为该子代理定制专有的沙箱边界和工具集以实现进程修改隔离
        let worker_sandbox = SecuritySandbox::new(temp_workspace.clone());
        let worker_tools = Arc::new(ToolRegistry::new(worker_sandbox));

        let worker_state = WorkerState {
            id: worker_id.clone(), session_id: session_id.clone(), task_id: task.id.clone(),
            status: WorkerStatus::Thinking, current_thought: None,
            tool_call_count: 0, consecutive_errors: 0, last_error_hash: None,
            created_at: Utc::now(), last_active: Utc::now(), token_count: 0,
            cow_path: Some(temp_workspace.to_string_lossy().to_string()),
        };
        self.workers.write().await.insert(worker_id.clone(), worker_state);

        let current_cost_config = self.cost_config.read().await.clone();
        let loop_breaker = Arc::new(LoopBreaker::new(worker_id.clone()));
        let react_loop = ReActLoop::new(
            worker_id.clone(), session_id, self.provider.clone(), worker_tools,
            self.blackboard.clone(), loop_breaker, self.approval_queue.clone(),
            current_cost_config, self.max_iterations,
        );

        let workers_clone = self.workers.clone();
        let wid = worker_id.clone();
        let provider_for_summary = self.provider.clone();
        let blackboard_for_summary = self.blackboard.clone();
        let task_id_for_summary = task.id.clone();
        
        tokio::spawn(async move {
            match react_loop.run(system_prompt, task.description).await {
                Ok(result) => {
                    let mut workers = workers_clone.write().await;
                    if let Some(w) = workers.get_mut(&wid) {
                        w.status = WorkerStatus::Completed;
                        w.last_active = Utc::now();
                    }
                    println!("Worker {} 完成: {}", wid, result);

                    // 1. 运行轨迹摘要提取
                    let state = react_loop.get_state().await;
                    if let Ok(summary) = super::summarizer::Summarizer::summarize_run(provider_for_summary, &state.history).await {
                        blackboard_for_summary.write(
                            BlackboardEntryType::IdentifiedBug,
                            format!("{}_takeaway", task_id_for_summary),
                            summary,
                            wid.clone()
                        ).await;
                    }

                    // 2. 任务成功完成。把隔离快照中的增量修改写入原始工作区
                    if let Err(e) = workspace_mgr.apply_to_original() {
                        println!("Worker {} COW 修改合并写回失败: {}", wid, e);
                    }
                    workspace_mgr.cleanup();
                }
                Err(e) => {
                    let mut workers = workers_clone.write().await;
                    if let Some(w) = workers.get_mut(&wid) {
                        w.status = WorkerStatus::Failed;
                        w.last_active = Utc::now();
                    }
                    println!("Worker {} 失败: {}", wid, e);
                    workspace_mgr.cleanup();
                }
            }
        });

        Ok(worker_id)
    }

    pub async fn get_worker(&self, id: &str) -> Option<WorkerState> {
        self.workers.read().await.get(id).cloned()
    }

    pub async fn list_workers(&self) -> Vec<WorkerState> {
        self.workers.read().await.values().cloned().collect()
    }

    pub async fn kill_worker(&self, id: &str) -> bool {
        let mut workers = self.workers.write().await;
        if let Some(w) = workers.get_mut(id) {
            w.status = WorkerStatus::Killed;
            true
        } else { false }
    }
}