use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::Utc;
use super::types::*;
use super::llm::{LlmProvider, LlmMessage};
use super::tools::ToolRegistry;
use super::blackboard::SharedBlackboard;
use super::loop_breaker::LoopBreaker;
use super::approval::ApprovalQueue;

/// ReAct 循环引擎 - Thought → Action → Observation
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
            cost_limit_usd: cost_config.cost_limit, // 预算限额，单位元
            is_circuit_broken: false,
            total_cost_cny: 0.0,
        }));
        Self {
            worker_id, session_id, provider, tools, blackboard, loop_breaker, approval_queue,
            state: Arc::new(RwLock::new(state)),
            cost_tracker,
            cost_config,
            max_iterations,
        }
    }

    pub async fn run(&self, system_prompt: String, user_message: String) -> Result<String, String> {
        let mut messages = vec![
            LlmMessage::system(system_prompt),
            LlmMessage::user(user_message),
        ];

        loop {
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

                if tracker.total_cost_cny >= tracker.cost_limit_usd {
                    tracker.is_circuit_broken = true;
                    let mut state = self.state.write().await;
                    state.is_terminated = true;
                    let err_reason = format!(
                        "【Token防爆熔断】当前会话消费已达 ¥{:.4}，超过预算上限 ¥{:.2}，强制熔断挂起！",
                        tracker.total_cost_cny, tracker.cost_limit_usd
                    );
                    state.termination_reason = Some(err_reason.clone());
                    return Err(err_reason);
                }
            }

            let thought = response.thought.clone().unwrap_or_default();
            let iteration = {
                let mut state = self.state.write().await;
                state.iteration += 1;
                state.iteration
            };

            if response.tool_calls.is_empty() {
                let step = ReActStep {
                    iteration, thought: thought.clone(), action: None,
                    observation: Some("任务完成".into()), timestamp: Utc::now(),
                };
                self.state.write().await.history.push(step);
                return Ok(response.content);
            }

            messages.push(LlmMessage::assistant(response.content.clone()));
            let mut observations = Vec::new();

            for tool_call in &response.tool_calls {
                let needs_approval = self.approval_queue.needs_approval(&tool_call.tool_name).await;
                if needs_approval {
                    let risk = self.tools.get_tool(&tool_call.tool_name).await
                        .map(|t| t.definition().risk_level).unwrap_or(RiskLevel::Medium);
                    
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
                        risk_level: risk, reason: format!("Worker {} 请求执行 {}", self.worker_id, tool_call.tool_name),
                        diff_preview, status: ApprovalStatus::Pending, created_at: Utc::now(), resolved_at: None,
                    };
                    let approved = self.approval_queue.request_approval(req).await;
                    if !approved {
                        observations.push(format!("工具 {} 被拒绝", tool_call.tool_name));
                        continue;
                    }
                }

                self.loop_breaker.record_tool_call().await;
                let result = self.tools.execute(&tool_call.tool_name, &tool_call.arguments).await;
                if !result.success {
                    self.loop_breaker.record_error(&result.error.clone().unwrap_or_default()).await;
                }
                let obs = if result.success { result.output } else { format!("错误: {}", result.error.unwrap_or_default()) };
                observations.push(format!("[{}] {}", tool_call.tool_name, obs));
                messages.push(LlmMessage::tool(format!("[{}] {}", tool_call.tool_name, obs)));
            }

            let step = ReActStep {
                iteration, thought, action: response.tool_calls.first().cloned(),
                observation: Some(observations.join("\n")), timestamp: Utc::now(),
            };
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