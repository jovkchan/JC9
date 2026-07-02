use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use chrono::Utc;
use std::collections::HashMap;
use std::time::Instant;
use tauri::Emitter;
use rusqlite::Connection;
use super::types::*;
use super::llm::{LlmProvider, LlmMessage};
use super::tools::{ToolRegistry, ToolResult};
use super::blackboard::SharedBlackboard;
use super::loop_breaker::LoopBreaker;
use super::approval::ApprovalQueue;
use super::guardrails::{Guardrails, GuardrailLevel};
use super::tracer::{Tracer, TraceEventType};

/// ReAct 循环引擎 - Thought → Action → Observation
/// 事件限频器 - 控制高频事件（thought/observation）的发射频率
struct EventRateLimiter {
    last_emit: Arc<RwLock<HashMap<String, Instant>>>,
    min_interval_ms: u64,
}

impl EventRateLimiter {
    fn new(min_interval_ms: u64) -> Self {
        Self { last_emit: Arc::new(RwLock::new(HashMap::new())), min_interval_ms }
    }

    async fn can_emit(&self, event_key: &str) -> bool {
        let now = Instant::now();
        let mut map = self.last_emit.write().await;
        if let Some(last) = map.get(event_key) {
            if now.duration_since(*last).as_millis() < self.min_interval_ms as u128 {
                return false;
            }
        }
        map.insert(event_key.to_string(), now);
        true
    }
}

pub struct ReActLoop {
    worker_id: String,
    session_id: String,
    provider: Arc<dyn LlmProvider>,
    tools: Arc<ToolRegistry>,
    #[allow(dead_code)]
    blackboard: Arc<SharedBlackboard>,
    loop_breaker: Arc<LoopBreaker>,
    approval_queue: Arc<ApprovalQueue>,
    state: Arc<RwLock<ReActState>>,
    cost_tracker: Arc<RwLock<TokenCostTracker>>,
    cost_config: CostConfig,
    max_iterations: u32,
    workers: Arc<RwLock<HashMap<String, WorkerState>>>,
    app_handle: Option<tauri::AppHandle>,
    db_conn: Option<Arc<Mutex<Connection>>>,
    #[allow(dead_code)]
    started_at: Instant,
    rate_limiter: Arc<EventRateLimiter>,
    tracer: Arc<Tracer>,
}

impl ReActLoop {
    pub fn new(
        worker_id: String,
        session_id: String,
        provider: Arc<dyn LlmProvider>,
        tools: Arc<ToolRegistry>,
        blackboard: Arc<SharedBlackboard>,
        loop_breaker: Arc<LoopBreaker>,
        approval_queue: Arc<ApprovalQueue>,
        cost_config: CostConfig,
        max_iterations: u32,
        workers: Arc<RwLock<HashMap<String, WorkerState>>>,
        app_handle: Option<tauri::AppHandle>,
        db_conn: Option<Arc<Mutex<Connection>>>,
        tracer: Arc<Tracer>,
    ) -> Self {
        let state = ReActState {
            worker_id: worker_id.clone(),
            iteration: 0,
            history: Vec::new(),
            is_terminated: false,
            termination_reason: None,
        };
        let cost_tracker = Arc::new(RwLock::new(TokenCostTracker {
            session_id: session_id.clone(),
            input_tokens: 0,
            output_tokens: 0,
            total_cost_usd: 0.0,
                cost_limit_cny: cost_config.cost_limit, // 预算限额，单位元（人民币）
            is_circuit_broken: false,
            total_cost_cny: 0.0,
        }));
        Self {
            worker_id, session_id, provider, tools, blackboard, loop_breaker, approval_queue,
            state: Arc::new(RwLock::new(state)),
            cost_tracker,
            cost_config,
            max_iterations,
            workers,
            app_handle,
            db_conn,
            started_at: Instant::now(),
            rate_limiter: Arc::new(EventRateLimiter::new(100)), // 100ms = 最多 10 次/秒
            tracer,
        }
    }

    async fn update_worker_state<F>(&self, update_fn: F)
    where F: FnOnce(&mut WorkerState) {
        let (history, term_reason) = {
            let state = self.state.read().await;
            (state.history.clone(), state.termination_reason.clone())
        };
        let mut workers = self.workers.write().await;
        if let Some(w) = workers.get_mut(&self.worker_id) {
            update_fn(w);
            w.last_active = Utc::now();
            w.history = history;
            w.termination_reason = term_reason;
            if let Some(ref handle) = self.app_handle {
                let _ = handle.emit("ai:worker-update", w.clone());
            }
        }
    }

    /// 发射 ai:thought 事件（限频：每 100ms 最多一次）
    async fn emit_thought_event(&self, thought: &str, iteration: u32) {
        if !self.rate_limiter.can_emit(&format!("thought:{}", self.worker_id)).await { return; }
        if let Some(ref handle) = self.app_handle {
            let event = ThoughtEvent {
                worker_id: self.worker_id.clone(),
                session_id: self.session_id.clone(),
                iteration,
                thought: thought.to_string(),
                timestamp: Utc::now().to_rfc3339(),
            };
            let _ = handle.emit("ai:thought", event);
        }
    }

    /// 发射 ai:observation 事件（限频：每 100ms 最多一次）
    async fn emit_observation_event(&self, tool_name: &str, success: bool, observation: &str, iteration: u32) {
        if !self.rate_limiter.can_emit(&format!("obs:{}", self.worker_id)).await { return; }
        if let Some(ref handle) = self.app_handle {
            let event = ObservationEvent {
                worker_id: self.worker_id.clone(),
                session_id: self.session_id.clone(),
                iteration,
                tool_name: tool_name.to_string(),
                success,
                observation: observation.to_string(),
                timestamp: Utc::now().to_rfc3339(),
            };
            let _ = handle.emit("ai:observation", event);
        }
    }

    /// 发射 ai:checkpoint 事件（无需限频）
    async fn emit_checkpoint_event(&self, thought: &str, has_action: bool, has_observation: bool, iteration: u32) {
        if let Some(ref handle) = self.app_handle {
            let event = CheckpointEvent {
                worker_id: self.worker_id.clone(),
                session_id: self.session_id.clone(),
                iteration,
                thought: thought.to_string(),
                has_action,
                has_observation,
                timestamp: Utc::now().to_rfc3339(),
            };
            let _ = handle.emit("ai:checkpoint", event);
        }
    }

    /// 发射 ai:cost-update 事件（无需限频）
    async fn emit_cost_update_event(&self) {
        if let Some(ref handle) = self.app_handle {
            let tracker = self.cost_tracker.read().await;
            let event = CostUpdateEvent {
                worker_id: self.worker_id.clone(),
                session_id: self.session_id.clone(),
                input_tokens: tracker.input_tokens,
                output_tokens: tracker.output_tokens,
                total_tokens: tracker.input_tokens + tracker.output_tokens,
                cost_cny: tracker.total_cost_cny,
                cost_usd: tracker.total_cost_usd,
                cost_limit_cny: tracker.cost_limit_cny,
                is_circuit_broken: tracker.is_circuit_broken,
                timestamp: Utc::now().to_rfc3339(),
            };
            let _ = handle.emit("ai:cost-update", event);
        }
    }

    /// 持久化当前迭代的 checkpoint 到 SQLite
    async fn save_checkpoint(&self, thought: &str, action: Option<&ToolCallRecord>, observation: Option<&str>) {
        if let Some(ref conn_arc) = self.db_conn {
            let iteration = self.state.read().await.iteration;
            let action_json = action.map(|a| serde_json::to_string(a).unwrap_or_default()).unwrap_or_default();
            let obs = observation.unwrap_or("");
            let ts = Utc::now().to_rfc3339();
            if let Ok(conn) = conn_arc.lock() {
                let id = format!("cp_{}_{}", self.worker_id, iteration);
                let _ = conn.execute(
                    "INSERT OR REPLACE INTO react_checkpoints (id, session_id, worker_id, iteration, thought, action, observation, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    rusqlite::params![id, self.session_id, self.worker_id, iteration, thought, action_json, obs, ts],
                );
            }
        }
    }

    pub async fn run(&self, system_prompt: String, user_message: String) -> Result<String, String> {
        let mut messages = vec![
            LlmMessage::system(system_prompt),
            LlmMessage::user(user_message),
        ];

        loop {
            {
                let workers = self.workers.read().await;
                if let Some(w) = workers.get(&self.worker_id) {
                    if w.status == WorkerStatus::Killed {
                        let mut state = self.state.write().await;
                        state.is_terminated = true;
                        state.termination_reason = Some("智能体已被用户终止。".into());
                        return Err("智能体已被用户终止。".into());
                    }
                }
            }

            if self.loop_breaker.is_tripped().await {
                let reason = self.loop_breaker.trip_reason().await;
                let mut state = self.state.write().await;
                state.is_terminated = true;
                state.termination_reason = reason.clone();
                return Err(reason.unwrap_or_else(|| "熔断器触发".into()));
            }

            {
                let state = self.state.read().await;
                if state.iteration >= self.max_iterations {
                    return Err(format!("达到最大迭代次数 {}", self.max_iterations));
                }
            }

            self.update_worker_state(|w| {
                w.status = WorkerStatus::Thinking;
            }).await;

            let tool_defs = self.tools.get_definitions().await;
            let response = self.provider.chat(&messages, &tool_defs).await?;

            // 统计 Token 成本与熔断
            let input_t = response.input_tokens;
            let output_t = response.output_tokens;
            let input_cost_cny = (input_t as f64) * self.cost_config.input_uncached_cost_per_m / 1_000_000.0;
            let output_cost_cny = (output_t as f64) * self.cost_config.output_cost_per_m / 1_000_000.0;
            let cost_cny = input_cost_cny + output_cost_cny;

            {
                let mut tracker = self.cost_tracker.write().await;
                tracker.input_tokens += input_t;
                tracker.output_tokens += output_t;
                tracker.total_cost_cny += cost_cny;
                tracker.total_cost_usd = tracker.total_cost_cny / 7.2;

                if tracker.total_cost_cny >= tracker.cost_limit_cny {
                    tracker.is_circuit_broken = true;
                    let mut state = self.state.write().await;
                    state.is_terminated = true;
                    let err_reason = format!(
                        "【Token防爆熔断】当前会话消费已达 ¥{:.4}，超过预算上限 ¥{:.2}，强制熔断挂起！",
                        tracker.total_cost_cny, tracker.cost_limit_cny
                    );
                    state.termination_reason = Some(err_reason.clone());
                    
                    self.update_worker_state(|w| {
                        w.status = WorkerStatus::Failed;
                    }).await;

                    return Err(err_reason);
                }
            }

            let thought = response.thought.clone().unwrap_or_default();
            let iteration = {
                let mut state = self.state.write().await;
                state.iteration += 1;
                state.iteration
            };

            self.update_worker_state(|w| {
                w.current_thought = Some(thought.clone());
                w.token_count += input_t + output_t;
            }).await;

            // 发射事件: ai:thought + ai:cost-update
            self.emit_thought_event(&thought, iteration).await;
            self.emit_cost_update_event().await;

            // 追踪：成本更新 + 思考内容
            {
                let tracker = self.cost_tracker.read().await;
                self.tracer.record(&self.session_id, Some(&self.worker_id), TraceEventType::CostUpdated, serde_json::json!({
                    "input_tokens": tracker.input_tokens,
                    "output_tokens": tracker.output_tokens,
                    "cost_cny": tracker.total_cost_cny,
                    "cost_usd": tracker.total_cost_usd,
                    "is_circuit_broken": tracker.is_circuit_broken,
                    "iteration": iteration,
                })).await;
            }
            if !thought.is_empty() {
                self.tracer.record(&self.session_id, Some(&self.worker_id), TraceEventType::Thought, serde_json::json!({
                    "thought": thought,
                    "iteration": iteration,
                })).await;
            }

            println!(
                "🔄 [ReAct] Worker={} iter={} | thought={:.80} | tool_calls={}",
                self.worker_id, iteration, thought, response.tool_calls.len()
            );

            if response.tool_calls.is_empty() {
                let step = ReActStep {
                    iteration, thought: thought.clone(), action: None,
                    observation: Some("任务完成".into()), timestamp: Utc::now(),
                };
                self.state.write().await.history.push(step);
                self.save_checkpoint(&thought, None, Some("任务完成")).await;
                self.emit_checkpoint_event(&thought, false, true, iteration).await;

                self.update_worker_state(|w| {
                    w.status = WorkerStatus::Completed;
                }).await;

                return Ok(response.content);
            }
            // DS thinking mode: 有 tool_calls 时必须回传 reasoning_content 和 tool_calls
            // （上方 is_empty() 检查已 return，此处 tool_calls 必非空）
            let reasoning = if thought.is_empty() { None } else { Some(thought.clone()) };
            messages.push(LlmMessage::assistant_with_tool_calls(
                response.content.clone(),
                reasoning,
                &response.tool_calls,
            ));
            let mut observations = Vec::new();

            for tool_call in &response.tool_calls {
                // 1. 拦截沙箱越界或白名单写入操作
                let mut is_out_of_bounds = false;
                let mut oob_error_msg = String::new();
                let mut path_str_val = String::new();

                let path_arg = tool_call.arguments["path"].as_str()
                    .or_else(|| tool_call.arguments["working_dir"].as_str())
                    .or_else(|| tool_call.arguments["workingDir"].as_str());

                if let Some(path_str) = path_arg {
                    path_str_val = path_str.to_string();
                    let sandbox = self.tools.sandbox();
                    if tool_call.tool_name == "write_file" || tool_call.tool_name == "patch_file" {
                        if let Err(e) = sandbox.validate_write_path(path_str) {
                            is_out_of_bounds = true;
                            oob_error_msg = e;
                        }
                    } else if tool_call.tool_name == "read_file" || tool_call.tool_name == "find_symbols" || tool_call.tool_name == "grep" || tool_call.tool_name == "run_command" {
                        if let Err(e) = sandbox.validate_read_path(path_str) {
                            is_out_of_bounds = true;
                            oob_error_msg = e;
                        }
                    }
                }

                let needs_approval = is_out_of_bounds || self.approval_queue.needs_approval(&tool_call.tool_name).await;
                if needs_approval {
                    let risk = if is_out_of_bounds {
                        RiskLevel::Critical
                    } else {
                        self.tools.get_tool(&tool_call.tool_name).await
                            .map(|t| t.definition().risk_level).unwrap_or(RiskLevel::Medium)
                    };

                    let reason = if is_out_of_bounds {
                        format!("【安全越界授权警告】Worker {} 请求读写项目外部敏感路径或修改只读配置文件: {}", self.worker_id, path_str_val)
                    } else {
                        format!("Worker {} 请求执行 {}", self.worker_id, tool_call.tool_name)
                    };
                    
                    // 智能生成 Diff 预览供审批看板渲染
                    let mut diff_preview = None;
                    if tool_call.tool_name == "patch_file" {
                        let target = tool_call.arguments["targetContent"].as_str()
                            .or_else(|| tool_call.arguments["target_content"].as_str())
                            .unwrap_or("");
                        let replacement = tool_call.arguments["replacementContent"].as_str()
                            .or_else(|| tool_call.arguments["replacement_content"].as_str())
                            .unwrap_or("");
                        diff_preview = Some(super::tools::generate_simple_diff(target, replacement));
                    } else if tool_call.tool_name == "write_file" {
                        let content = tool_call.arguments["content"].as_str().unwrap_or("");
                        diff_preview = Some(content.lines().map(|l| format!("+ {}", l)).collect::<Vec<String>>().join("\n"));
                    }

                    let req = ApprovalRequest {
                        id: uuid::Uuid::new_v4().to_string(),
                        worker_id: self.worker_id.clone(), session_id: self.session_id.clone(),
                        tool_name: tool_call.tool_name.clone(), arguments: tool_call.arguments.clone(),
                        risk_level: risk, reason,
                        diff_preview, status: ApprovalStatus::Pending, created_at: Utc::now(), resolved_at: None,
                    };

                    self.update_worker_state(|w| {
                        w.status = WorkerStatus::WaitingApproval;
                    }).await;

                    let approved = self.approval_queue.request_approval(req).await;
                    if !approved {
                        self.update_worker_state(|w| {
                            w.status = WorkerStatus::Thinking;
                        }).await;

                        let error_out = format!("错误: 越界读取/写入安全拦截限制。操作被拒绝: {}", oob_error_msg);
                        observations.push(format!("[{}] {}", tool_call.tool_name, error_out));
                        messages.push(LlmMessage::tool_with_id(
                            format!("[{}] {}", tool_call.tool_name, error_out),
                            tool_call.id.clone(),
                        ));
                        continue;
                    }
                }

                // ← Guardrails 预校验（在工具执行前拦截明显错误的参数）
                let guardrails = Guardrails::new(self.tools.sandbox());
                match guardrails.validate(&tool_call.tool_name, &tool_call.arguments).await {
                    GuardrailLevel::Error(msg) => {
                        let guardrail_err = format!("【Guardrails 拦截】{}", msg);
                        println!("  🛡️ [Guardrails] {} blocked: {}", tool_call.tool_name, msg);
                        observations.push(format!("[{}] {}", tool_call.tool_name, guardrail_err));
                        messages.push(LlmMessage::tool_with_id(
                            format!("[{}] {}", tool_call.tool_name, guardrail_err),
                            tool_call.id.clone(),
                        ));
                        self.update_worker_state(|w| {
                            w.consecutive_errors += 1;
                        }).await;
                        continue;
                    }
                    GuardrailLevel::Warning(msg) => {
                        println!("  🛡️ [Guardrails] {} warning: {}", tool_call.tool_name, msg);
                        // Warning 不阻塞，仅日志记录
                    }
                    GuardrailLevel::Critical(msg) => {
                        // Critical 应当已在审批阶段处理，此处作为兜底
                        let guardrail_err = format!("【Guardrails 严重拦截】{}", msg);
                        println!("  🛡️ [Guardrails] {} critical: {}", tool_call.tool_name, msg);
                        observations.push(format!("[{}] {}", tool_call.tool_name, guardrail_err));
                        messages.push(LlmMessage::tool_with_id(
                            format!("[{}] {}", tool_call.tool_name, guardrail_err),
                            tool_call.id.clone(),
                        ));
                        self.update_worker_state(|w| {
                            w.consecutive_errors += 1;
                        }).await;
                        continue;
                    }
                    GuardrailLevel::Pass => {}
                }

                // 准备执行工具
                self.update_worker_state(|w| {
                    w.status = WorkerStatus::CallingTool;
                }).await;

                // 追踪：工具调用开始
                self.tracer.record(&self.session_id, Some(&self.worker_id), TraceEventType::ToolInvocation, serde_json::json!({
                    "tool_name": tool_call.tool_name,
                    "arguments": tool_call.arguments,
                    "iteration": iteration,
                })).await;

                // 2. 检查工具次数限制并可能注入警告词
                let warning_needed = self.loop_breaker.record_tool_call().await;
                if warning_needed {
                    let alert_msg = LlmMessage::system("【系统中断告警】：你已经连续多次执行相同操作或工具调用次数过多，当前思路存在死循环风险。请立即停止当前的命令重试，重新梳理黑板上的共享信息，更换其他替代工具或重构你的解题策略！".to_string());
                    messages.push(alert_msg);
                    self.update_worker_state(|w| {
                        w.current_thought = Some("【系统自愈】连续调用工具超过10次，已强行注入反思引导词...".into());
                    }).await;
                }

                // 3. 执行工具（若属于用户授权的越界操作，直接在 react_loop 代理中文件系统反射执行）
                let mut proxy_executed = false;
                let mut proxy_success = false;
                let mut proxy_output = String::new();

                if is_out_of_bounds {
                    proxy_executed = true;
                    if tool_call.tool_name == "read_file" {
                        match std::fs::read_to_string(&path_str_val) {
                            Ok(content) => {
                                proxy_success = true;
                                proxy_output = content;
                            }
                            Err(e) => {
                                proxy_success = false;
                                proxy_output = format!("读取越界路径文件失败: {}", e);
                            }
                        }
                    } else if tool_call.tool_name == "write_file" {
                        let content = tool_call.arguments["content"].as_str().unwrap_or("");
                        let p = std::path::Path::new(&path_str_val);
                        if let Some(parent) = p.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        match std::fs::write(&path_str_val, content) {
                            Ok(_) => {
                                proxy_success = true;
                                proxy_output = "越界写入文件成功。".into();
                            }
                            Err(e) => {
                                proxy_success = false;
                                proxy_output = format!("越界写入文件失败: {}", e);
                            }
                        }
                    } else if tool_call.tool_name == "patch_file" {
                        let target_content = tool_call.arguments["targetContent"].as_str()
                            .or_else(|| tool_call.arguments["target_content"].as_str())
                            .unwrap_or("");
                        let replacement_content = tool_call.arguments["replacementContent"].as_str()
                            .or_else(|| tool_call.arguments["replacement_content"].as_str())
                            .unwrap_or("");
                        match std::fs::read_to_string(&path_str_val) {
                            Ok(original) => {
                                let count = original.matches(target_content).count();
                                if count == 1 {
                                    let modified = original.replacen(target_content, replacement_content, 1);
                                    match std::fs::write(&path_str_val, &modified) {
                                        Ok(_) => {
                                            proxy_success = true;
                                            proxy_output = format!("越界修改成功！\n{}", super::tools::generate_simple_diff(target_content, replacement_content));
                                        }
                                        Err(e) => {
                                            proxy_success = false;
                                            proxy_output = format!("越界修改写回失败: {}", e);
                                        }
                                    }
                                } else {
                                    proxy_success = false;
                                    proxy_output = format!("修改失败：目标内容在文件中非唯一或找不到（匹配到 {} 处）。", count);
                                }
                            }
                            Err(e) => {
                                proxy_success = false;
                                proxy_output = format!("读取越界文件修改失败: {}", e);
                            }
                        }
                    } else if tool_call.tool_name == "run_command" {
                        let command_str = tool_call.arguments["command"].as_str().unwrap_or("");
                        let sandbox = self.tools.sandbox();
                        if !sandbox.validate_command(command_str) {
                            proxy_success = false;
                            proxy_output = format!("【安全拦截】命令 '{}' 不在安全白名单中或包含黑名单模式，拒绝执行。", command_str);
                        } else {
                            let mut cmd = if cfg!(target_os = "windows") {
                                let mut c = std::process::Command::new("powershell");
                                c.args(["-NoProfile", "-Command", command_str]);
                                c
                            } else {
                                let mut c = std::process::Command::new("sh");
                                c.args(["-c", command_str]);
                                c
                            };
                            cmd.current_dir(&path_str_val);
                            match cmd.output() {
                                Ok(output) => {
                                    proxy_success = output.status.success();
                                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                                    let mut combined = stdout;
                                    if !stderr.is_empty() {
                                        combined.push_str("\n【标准错误输出】:\n");
                                        combined.push_str(&stderr);
                                    }
                                    proxy_output = combined;
                                }
                                Err(e) => {
                                    proxy_success = false;
                                    proxy_output = format!("越界工作目录命令执行失败: {}", e);
                                }
                            }
                        }
                    } else {
                        // 兜底退回
                        proxy_executed = false;
                    }
                }

                let result = if proxy_executed {
                    ToolResult {
                        success: proxy_success,
                        output: if proxy_success { proxy_output.clone() } else { "".into() },
                        error: if !proxy_success { Some(proxy_output.clone()) } else { None },
                    }
                } else {
                    self.tools.execute(&tool_call.tool_name, &tool_call.arguments).await
                };

                // 追踪：工具调用结果
                self.tracer.record(&self.session_id, Some(&self.worker_id), TraceEventType::ToolResult, serde_json::json!({
                    "tool_name": tool_call.tool_name,
                    "success": result.success,
                    "output_length": result.output.len(),
                    "has_error": result.error.is_some(),
                    "iteration": iteration,
                })).await;

                // 更新 Worker 状态为 Observing 并记录调用数与连续错误数
                self.update_worker_state(|w| {
                    w.status = WorkerStatus::Observing;
                    w.tool_call_count += 1;
                    if !result.success {
                        w.consecutive_errors += 1;
                    } else {
                        w.consecutive_errors = 0;
                    }
                }).await;

                if !result.success {
                    let err_str = result.error.clone().unwrap_or_default();
                    let warning_needed = self.loop_breaker.record_error(&err_str).await;
                    if warning_needed {
                        let alert_msg = LlmMessage::system("【系统中断告警】：你已经连续多次产生相同报错，当前思路存在死循环风险。请立即停止当前的命令重试，重新梳理黑板上的共享信息，更换其他替代工具或重构你的解题策略！".to_string());
                        messages.push(alert_msg);
                        self.update_worker_state(|w| {
                            w.current_thought = Some("【系统自愈】检测到连续报错，已强行注入反思引导词...".into());
                        }).await;
                    }
                }

                println!(
                    "🔧 [ReAct] tool={} success={} | output={:.100}",
                    tool_call.tool_name, result.success,
                    if result.success { &result.output } else { result.error.as_deref().unwrap_or("") }
                );
                let obs = if result.success { result.output.clone() } else { format!("错误: {}", result.error.clone().unwrap_or_default()) };
                observations.push(format!("[{}] {}", tool_call.tool_name, obs));
                messages.push(LlmMessage::tool_with_id(
                    format!("[{}] {}", tool_call.tool_name, obs),
                    tool_call.id.clone(),
                ));

                // 发射事件: ai:observation
                self.emit_observation_event(&tool_call.tool_name, result.success, &obs, iteration).await;
            }

            let step = ReActStep {
                iteration, thought, action: response.tool_calls.first().cloned(),
                observation: Some(observations.join("\n")), timestamp: Utc::now(),
            };
            let obs_joined = observations.join("\n");
            self.save_checkpoint(&step.thought, step.action.as_ref(), Some(&obs_joined)).await;
            self.emit_checkpoint_event(&step.thought, step.action.is_some(), !obs_joined.is_empty(), iteration).await;
            self.state.write().await.history.push(step);
        }
    }

    pub async fn get_state(&self) -> ReActState { self.state.read().await.clone() }

    pub async fn cost_tracker(&self) -> TokenCostTracker { self.cost_tracker.read().await.clone() }

    pub async fn terminate(&self, reason: String) {
        let mut state = self.state.write().await;
        state.is_terminated = true;
        state.termination_reason = Some(reason);
    }
}