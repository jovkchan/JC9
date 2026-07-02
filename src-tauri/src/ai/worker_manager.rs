use std::sync::{Arc, Mutex};
use tokio::sync::{RwLock, Semaphore};
use std::collections::HashMap;
use std::time::Instant;
use chrono::Utc;
use std::path::PathBuf;
use tauri::Emitter;
use rusqlite::Connection;

use super::types::*;
use super::llm::LlmProvider;
use super::tools::{ToolRegistry, ToolDefinition};
use super::blackboard::SharedBlackboard;
use super::react_loop::ReActLoop;
use super::loop_breaker::LoopBreaker;
use super::approval::ApprovalQueue;
use super::workspace::WorkspaceManager;
use super::security::SecuritySandbox;
use super::host_detector::HostDetector;
use super::knowledge_base::KnowledgeBase;
use super::prompt_builder::PromptBuilder;
use super::repo_map::RepoMap;
use super::mcp_client::McpClient;
use super::tracer::{Tracer, TraceEventType};
use super::browser::{BrowserManager, BrowserNavigateTool, BrowserClickTool, BrowserTypeTool,
    BrowserGetHtmlTool, BrowserGetTextTool, BrowserScreenshotTool, BrowserCloseTool};

/// Worker 管理器 - 并发控制、Worker 生命周期与 COW 隔离环境调度
pub struct WorkerManager {
    workers: Arc<RwLock<HashMap<String, WorkerState>>>,
    semaphore: Arc<Semaphore>,
    provider: Arc<dyn LlmProvider>,
    blackboard: Arc<SharedBlackboard>,
    approval_queue: Arc<ApprovalQueue>,
    knowledge_base: Arc<KnowledgeBase>,
    workspace_root: Arc<tokio::sync::RwLock<PathBuf>>,
    cost_config: Arc<tokio::sync::RwLock<CostConfig>>,
    max_iterations: u32,
    app_handle: Option<tauri::AppHandle>,
    db_conn: Option<Arc<Mutex<Connection>>>,
    mcp_client: Arc<McpClient>,
    frontend_tools: Arc<RwLock<HashMap<String, ToolDefinition>>>,
    tracer: Arc<Tracer>,
    browser_manager: Arc<BrowserManager>,
}

impl WorkerManager {
    pub fn new(
        provider: Arc<dyn LlmProvider>,
        blackboard: Arc<SharedBlackboard>,
        approval_queue: Arc<ApprovalQueue>,
        knowledge_base: Arc<KnowledgeBase>,
        workspace_root: Arc<tokio::sync::RwLock<PathBuf>>,
        cost_config: Arc<tokio::sync::RwLock<CostConfig>>,
        max_concurrent: usize,
        max_iterations: u32,
        app_handle: Option<tauri::AppHandle>,
        db_conn: Option<Arc<Mutex<Connection>>>,
        mcp_client: Arc<McpClient>,
        tracer: Arc<Tracer>,
        browser_manager: Arc<BrowserManager>,
    ) -> Self {
        Self {
            workers: Arc::new(RwLock::new(HashMap::new())),
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            provider, blackboard, approval_queue, knowledge_base, workspace_root, cost_config, max_iterations,
            app_handle, db_conn, mcp_client,
            frontend_tools: Arc::new(RwLock::new(HashMap::new())),
            tracer,
            browser_manager,
        }
    }

    /// 注册前端工具定义
    pub async fn register_frontend_tool(&self, definition: ToolDefinition) {
        self.frontend_tools.write().await.insert(definition.name.clone(), definition);
    }

    /// 铺拉起一个并发 Worker 并绑定 COW 隔离开发空间
    pub async fn spawn_worker(&self, session_id: String, task: TaskNode, system_prompt: String) -> Result<String, String> {
        let sem_clone = self.semaphore.clone();
        let _permit = sem_clone.acquire_owned().await.map_err(|e| e.to_string())?;
        let worker_id = uuid::Uuid::new_v4().to_string();

        // 1. 初始化 COW 临时沙箱
        let current_root = self.workspace_root.read().await.clone();
        let workspace_mgr = Arc::new(WorkspaceManager::new(current_root.clone()));
        let temp_workspace = workspace_mgr.prepare_sandbox()?;
        println!("📦 [Worker] {} COW沙箱: {}", &worker_id[..8], temp_workspace.display());

        // 2. 为该子代理定制专有的沙箱边界和工具集以实现进程修改隔离
        let mut worker_sandbox = SecuritySandbox::new(temp_workspace.clone());
        // 允许读取原始工作区文件（写操作仍走 COW 沙箱）
        worker_sandbox.add_read_only_path(current_root);
        let worker_tools = Arc::new(ToolRegistry::new(worker_sandbox));

        // 将 MCP 工具注册到当前 Worker 的工具集
        self.mcp_client.bind_registry(worker_tools.clone()).await;

        // 注册已有的前端动态工具
        {
            let ft_list = self.frontend_tools.read().await;
            if let Some(ref handle) = self.app_handle {
                for (name, def) in ft_list.iter() {
                    let tool = Arc::new(super::frontend_tool::FrontendProxyTool::new(def.clone(), handle.clone())) as Arc<dyn super::tools::Tool>;
                    worker_tools.register(name.clone(), tool).await;
                }
            }
        }

        // 注册浏览器操控工具
        {
            let bm = self.browser_manager.clone();
            let tools: Vec<(String, Arc<dyn super::tools::Tool>)> = vec![
                ("browser_navigate".into(), Arc::new(BrowserNavigateTool::new(bm.clone())) as Arc<dyn super::tools::Tool>),
                ("browser_click".into(), Arc::new(BrowserClickTool::new(bm.clone()))),
                ("browser_type".into(), Arc::new(BrowserTypeTool::new(bm.clone()))),
                ("browser_get_html".into(), Arc::new(BrowserGetHtmlTool::new(bm.clone()))),
                ("browser_get_text".into(), Arc::new(BrowserGetTextTool::new(bm.clone()))),
                ("browser_screenshot".into(), Arc::new(BrowserScreenshotTool::new(bm.clone()))),
                ("browser_close".into(), Arc::new(BrowserCloseTool::new(bm))),
            ];
            for (name, tool) in tools {
                worker_tools.register(name, tool).await;
            }
        }

        // 3. 通过 PromptBuilder 构建结构化 System Prompt
        let host_env = HostDetector::new().detect();
        let host_prompt = HostDetector::new().generate_system_prompt(&host_env);

        let repo_map = RepoMap::new(self.workspace_root.clone());
        let repo_map_text = repo_map.generate().await;

        let tool_defs = worker_tools.get_definitions().await;
        let cost_config = self.cost_config.read().await.clone();

        let enriched_system_prompt = PromptBuilder::new()
            .with_host_prompt(host_prompt)
            .with_repo_map(repo_map_text)
            .with_tools(tool_defs)
            .with_cost_config(cost_config)
            .with_safety_rules(
                "## 安全约束\n\
                 - 禁止执行可能破坏系统的命令（rm -rf /、format 等）\n\
                 - 文件写操作必须先读取确认目标文件内容\n\
                 - 所有修改前自动备份原文件\n\
                 - 高风险操作需等待用户审批\n"
                    .into(),
            )
            .build(&system_prompt);

        let worker_state = WorkerState {
            id: worker_id.clone(), session_id: session_id.clone(), task_id: task.id.clone(),
            status: WorkerStatus::Thinking, current_thought: None,
            tool_call_count: 0, consecutive_errors: 0, last_error_hash: None,
            created_at: Utc::now(), last_active: Utc::now(), token_count: 0,
            cow_path: Some(temp_workspace.to_string_lossy().to_string()),
            history: Vec::new(),
            termination_reason: None,
        };
        self.workers.write().await.insert(worker_id.clone(), worker_state.clone());

        if let Some(ref handle) = self.app_handle {
            let _ = handle.emit("ai:worker-update", worker_state);
        }

        // 追踪：Worker 启动
        self.tracer.record(&session_id, Some(&worker_id), TraceEventType::WorkerSpawned, serde_json::json!({
            "worker_id": worker_id,
            "task_title": task.title,
            "task_id": task.id,
            "cow_path": temp_workspace.to_string_lossy(),
        })).await;

        let current_cost_config = self.cost_config.read().await.clone();
        let loop_breaker = Arc::new(LoopBreaker::new(worker_id.clone()));
        let session_id_for_loop = session_id.clone();
        let react_loop = ReActLoop::new(
            worker_id.clone(), session_id_for_loop, self.provider.clone(), worker_tools,
            self.blackboard.clone(), loop_breaker, self.approval_queue.clone(),
            current_cost_config, self.max_iterations,
            self.workers.clone(), self.app_handle.clone(),
            self.db_conn.clone(), self.tracer.clone(),
        );

        let workers_clone = self.workers.clone();
        let wid = worker_id.clone();
        let wid_log = wid.clone();
        let provider_for_summary = self.provider.clone();
        println!("🚀 [Worker] {} 启动 | task={}", &wid_log[..8], task.title);
        let blackboard_for_summary = self.blackboard.clone();
        let knowledge_base = self.knowledge_base.clone();
        let task_id_for_summary = task.id.clone();
        let task_title = task.title.clone();
        let app_handle_clone = self.app_handle.clone();
        let session_id_clone = session_id.clone();
        
        tokio::spawn(async move {
            let _permit = _permit;
            // 启动会话心跳（每 5 秒发射 ai:session-progress）
            let heartbeat_handle = {
                let workers = workers_clone.clone();
                let wid = wid.clone();
                let sid = session_id_clone.clone();
                let ah = app_handle_clone.clone();
                let start = Instant::now();
                tokio::spawn(async move {
                    loop {
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                        let should_stop = {
                            let map = workers.read().await;
                            match map.get(&wid) {
                                Some(w) if w.status == WorkerStatus::Thinking
                                    || w.status == WorkerStatus::CallingTool
                                    || w.status == WorkerStatus::Observing
                                    || w.status == WorkerStatus::WaitingApproval => {
                                    if let Some(ref h) = ah {
                                        let event = SessionProgressEvent {
                                            worker_id: wid.clone(),
                                            session_id: sid.clone(),
                                            status: format!("{:?}", w.status).to_lowercase(),
                                            iteration: (w.history.len()) as u32,
                                            tool_call_count: w.tool_call_count,
                                            total_tokens: w.token_count,
                                            cost_cny: 0.0, // cost_tracker 在 ReActLoop 里，简化处理
                                            elapsed_seconds: start.elapsed().as_secs(),
                                            timestamp: Utc::now().to_rfc3339(),
                                        };
                                        let _ = h.emit("ai:session-progress", event);
                                    }
                                    false // continue heartbeat
                                }
                                _ => true, // worker done/killed, stop heartbeat
                            }
                        };
                        if should_stop { break; }
                    }
                })
            };

            let result = react_loop.run(enriched_system_prompt, task.description).await;
            // 停止心跳
            heartbeat_handle.abort();

            match result {
                Ok(result) => {
                    let state = react_loop.get_state().await;
                    let mut workers = workers_clone.write().await;
                    if let Some(w) = workers.get_mut(&wid) {
                        w.status = WorkerStatus::Completed;
                        w.last_active = Utc::now();
                        w.history = state.history.clone();
                        w.termination_reason = state.termination_reason.clone();
                        if let Some(ref handle) = app_handle_clone {
                            let _ = handle.emit("ai:worker-update", w.clone());
                        }
                    }
                    println!("🏁 [Worker] {} 完成 | result_len={}", &wid[..8], result.len());

                    // 1. 运行轨迹摘要提取并沉淀到知识库草稿箱
                    if let Ok(summary) = super::summarizer::Summarizer::summarize_run(provider_for_summary, &state.history).await {
                        // 写入共享黑板供其他 Worker 参考
                        blackboard_for_summary.write(
                            BlackboardEntryType::IdentifiedBug,
                            format!("{}_takeaway", task_id_for_summary),
                            summary.clone(),
                            wid.clone()
                        ).await;

                        // 持久化到知识库草稿箱（前端轮询可见）
                        let draft_entry = KbEntry {
                            id: String::new(),
                            title: format!("[Takeaway] {}", task_title),
                            content: summary,
                            entry_type: KbEntryType::Takeaway,
                            tags: vec!["auto-generated".into(), "takeaway".into()],
                            source_session: Some(wid.clone()),
                            confidence: 0.5, // 初始置信度，后续 Step 5 会自动调整
                            is_draft: true,
                            created_at: Utc::now(),
                            updated_at: Utc::now(),
                            embedding: None,
                        };
                        knowledge_base.add_entry(draft_entry).await;
                    }

                    // 2. 任务成功完成。把隔离快照中的增量修改写入原始工作区
                    if let Err(e) = workspace_mgr.apply_to_original() {
                        println!("Worker {} COW 修改合并写回失败: {}", wid, e);
                        // 合并失败：标记 Worker 为 Failed，通知前端，保留临时工作区供用户手动恢复
                        if let Some(w) = workers.get_mut(&wid) {
                            w.status = WorkerStatus::Failed;
                            w.termination_reason = Some(format!(
                                "COW 修改合并写回失败: {}. 临时工作区已保留: {}",
                                e, temp_workspace.display()
                            ));
                            if let Some(ref handle) = app_handle_clone {
                                let _ = handle.emit("ai:worker-update", w.clone());
                            }
                        }
                        // 不执行 cleanup()，保留临时工作区供用户手动恢复
                    } else {
                        workspace_mgr.cleanup();
                    }
                }
                Err(e) => {
                    let state = react_loop.get_state().await;
                    let mut workers = workers_clone.write().await;
                    if let Some(w) = workers.get_mut(&wid) {
                        w.status = WorkerStatus::Failed;
                        w.last_active = Utc::now();
                        w.history = state.history.clone();
                        w.termination_reason = Some(e.clone());
                        if let Some(ref handle) = app_handle_clone {
                            let _ = handle.emit("ai:worker-update", w.clone());
                        }
                    }
                    println!("💀 [Worker] {} 失败: {}", &wid[..8], e);
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
            w.last_active = Utc::now();
            if let Some(ref handle) = self.app_handle {
                let _ = handle.emit("ai:worker-update", w.clone());
            }

            // 追踪：Worker 被杀死
            self.tracer.record(&w.session_id, Some(id), TraceEventType::WorkerKilled, serde_json::json!({
                "worker_id": id,
                "task_id": w.task_id,
                "tool_call_count": w.tool_call_count,
                "reason": "user_killed",
            })).await;

            true
        } else { false }
    }
}