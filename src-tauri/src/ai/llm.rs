use async_trait::async_trait;
use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::types::*;
use super::tools::ToolDefinition;

/// LLM 消息格式
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmMessage {
    pub role: MessageRole,
    pub content: String,
    pub reasoning_content: Option<String>,  // DS thinking mode 思维链
    pub tool_call_id: Option<String>,       // 工具调用的ID，回传给API
    pub tool_calls: Vec<ToolCallRecord>,
}

impl LlmMessage {
    pub fn system(content: String) -> Self {
        Self { role: MessageRole::System, content, reasoning_content: None, tool_call_id: None, tool_calls: vec![] }
    }
    pub fn user(content: String) -> Self {
        Self { role: MessageRole::User, content, reasoning_content: None, tool_call_id: None, tool_calls: vec![] }
    }
    pub fn assistant(content: String) -> Self {
        Self { role: MessageRole::Assistant, content, reasoning_content: None, tool_call_id: None, tool_calls: vec![] }
    }
    pub fn assistant_with_tool_calls(content: String, reasoning: Option<String>, calls: &[ToolCallRecord]) -> Self {
        Self {
            role: MessageRole::Assistant,
            content,
            reasoning_content: reasoning,
            tool_call_id: None,
            tool_calls: calls.to_vec(),
        }
    }
    pub fn tool(content: String) -> Self {
        Self { role: MessageRole::Tool, content, reasoning_content: None, tool_call_id: None, tool_calls: vec![] }
    }
    pub fn tool_with_id(content: String, call_id: String) -> Self {
        Self { role: MessageRole::Tool, content, reasoning_content: None, tool_call_id: Some(call_id), tool_calls: vec![] }
    }
}

/// LLM 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmResponse {
    pub content: String,
    pub thought: Option<String>,
    pub tool_calls: Vec<ToolCallRecord>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub finish_reason: String,
}

/// LLM Provider trait - 抽象不同的大模型服务商
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn chat(&self, messages: &[LlmMessage], tools: &[ToolDefinition]) -> Result<LlmResponse, String>;
    fn name(&self) -> &str;
    fn estimate_cost(&self, input_tokens: u64, output_tokens: u64) -> f64;
    /// 运行时切换思维强度（DS thinking mode），默认空实现
    async fn set_reasoning_effort(&self, _effort: Option<String>) {}
}

/// Mock LLM Provider - 用于本地闭环逻辑验证
pub struct MockLlmProvider {
    call_count: Arc<Mutex<u32>>,
}

impl MockLlmProvider {
    pub fn new() -> Self {
        Self { call_count: Arc::new(Mutex::new(0)) }
    }

    fn generate_thought(&self, messages: &[LlmMessage], iteration: u32) -> String {
        let last_user = messages.iter().rev().find(|m| m.role == MessageRole::User);
        let user_text = last_user.map(|m| m.content.as_str()).unwrap_or("未知任务");

        if iteration == 0 {
            format!(
                "我需要分析用户的任务：「{}」。首先我应该读取相关文件来了解项目结构，然后制定执行计划。",
                user_text
            )
        } else if iteration < 3 {
            format!(
                "根据之前的观察结果，我需要继续推进任务。当前是第 {} 轮迭代，让我检查当前状态并决定下一步操作。",
                iteration + 1
            )
        } else {
            format!(
                "已经进行了 {} 轮迭代，让我汇总当前进展并准备给出最终结果。", iteration + 1
            )
        }
    }

    fn generate_tool_call(&self, _messages: &[LlmMessage], tools: &[ToolDefinition], iteration: u32) -> Option<ToolCallRecord> {
        if tools.is_empty() {
            return None;
        }

        // 根据迭代轮次选择不同工具
        let tool = if iteration == 0 {
            // 第一轮：尝试读取文件
            tools.iter().find(|t| t.name.contains("read") || t.name.contains("grep"))
                .or_else(|| tools.first())
        } else if iteration < 3 {
            // 中间轮：尝试执行或修改
            tools.iter().find(|t| t.name.contains("patch") || t.name.contains("run"))
                .or_else(|| tools.first())
        } else {
            // 后期：不再调用工具，准备总结
            return None;
        }?;

        let args = if tool.name.contains("read") || tool.name.contains("grep") {
            serde_json::json!({
                "path": "src/main.ts",
                "query": "function"
            })
        } else if tool.name.contains("patch") {
            serde_json::json!({
                "path": "src/main.ts",
                "content": "// patched by mock agent"
            })
        } else if tool.name.contains("run") {
            serde_json::json!({
                "command": "npm run build",
                "working_dir": "."
            })
        } else {
            serde_json::json!({})
        };

        Some(ToolCallRecord {
            id: uuid::Uuid::new_v4().to_string(),
            tool_name: tool.name.clone(),
            arguments: args,
            result: None,
            status: ToolCallStatus::Pending,
            timestamp: chrono::Utc::now(),
        })
    }
}

#[async_trait]
impl LlmProvider for MockLlmProvider {
    async fn chat(&self, messages: &[LlmMessage], tools: &[ToolDefinition]) -> Result<LlmResponse, String> {
        let mut count = self.call_count.lock().await;
        *count += 1;
        let iteration = *count - 1;

        // 模拟网络延迟
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let thought = self.generate_thought(messages, iteration);
        let tool_call = self.generate_tool_call(messages, tools, iteration);
        let has_tool_call = tool_call.is_some();

        let content = if has_tool_call {
            format!("{}\n\n我将调用工具来继续执行任务。", thought)
        } else {
            format!("{}\n\n任务已完成。根据我的分析，项目结构清晰，代码质量良好。", thought)
        };

        let input_tokens = messages.iter().map(|m| m.content.len() as u64 / 4).sum();
        let output_tokens = content.len() as u64 / 4;

        Ok(LlmResponse {
            content,
            thought: Some(thought),
            tool_calls: tool_call.into_iter().collect(),
            input_tokens,
            output_tokens,
            finish_reason: if has_tool_call { "tool_calls".into() } else { "stop".into() },
        })
    }

    fn name(&self) -> &str { "mock" }

    fn estimate_cost(&self, _input: u64, _output: u64) -> f64 { 0.0 }
}

/// OpenAI 兼容 Provider（完整支持 DeepSeek thinking mode + 运行时切换强度）
pub struct OpenAiProvider {
    api_key: Secret<String>,
    base_url: String,
    model: String,
    reasoning_effort: Arc<tokio::sync::RwLock<Option<String>>>,
    client: reqwest::Client,
}

impl std::fmt::Debug for OpenAiProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAiProvider")
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("api_key", &"[REDACTED]")
            .finish()
    }
}

impl OpenAiProvider {
    pub fn new(api_key: String, base_url: Option<String>, model: Option<String>, reasoning_effort_override: Option<String>) -> Self {
        let model = model.unwrap_or_else(|| "deepseek-v4-pro".into());
        let base_url = base_url.unwrap_or_else(|| "https://api.deepseek.com".into());

        let reasoning_effort = reasoning_effort_override
            .filter(|e| !e.is_empty() && e != "off")
            .or_else(|| {
                if model.contains("deepseek") || model.contains("o1") || model.contains("o3") {
                    Some("high".to_string())
                } else {
                    None
                }
            });

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        Self {
            api_key: Secret::new(api_key),
            base_url,
            model,
            reasoning_effort: Arc::new(tokio::sync::RwLock::new(reasoning_effort)),
            client,
        }
    }

    /// 运行时切换思维强度（即时生效）
    pub async fn set_reasoning_effort(&self, effort: Option<String>) {
        *self.reasoning_effort.write().await = effort;
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn chat(&self, messages: &[LlmMessage], tools: &[ToolDefinition]) -> Result<LlmResponse, String> {
        let url = format!("{}/chat/completions", self.base_url);

        let messages_json: Vec<serde_json::Value> = messages.iter().map(|m| {
            let role = match m.role {
                MessageRole::System => "system",
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::Tool => "tool",
            };
            let mut msg = serde_json::json!({ "role": role, "content": m.content });

            // DS thinking mode: 有 tool_calls 的 assistant 消息必须回传 reasoning_content
            if m.role == MessageRole::Assistant {
                if let Some(ref rc) = m.reasoning_content {
                    if !rc.is_empty() {
                        msg["reasoning_content"] = serde_json::json!(rc);
                    }
                }
                // 有 tool_calls 时必须带上
                if !m.tool_calls.is_empty() {
                    msg["tool_calls"] = serde_json::json!(m.tool_calls.iter().map(|tc| {
                        serde_json::json!({
                            "id": tc.id,
                            "type": "function",
                            "function": {
                                "name": tc.tool_name,
                                "arguments": serde_json::to_string(&tc.arguments).unwrap_or_default(),
                            }
                        })
                    }).collect::<Vec<_>>());
                }
            }

            // tool 消息必须带 tool_call_id
            if m.role == MessageRole::Tool {
                if let Some(ref tcid) = m.tool_call_id {
                    msg["tool_call_id"] = serde_json::json!(tcid);
                }
            }

            msg
        }).collect();

        let tools_json: Vec<serde_json::Value> = tools.iter().map(|t| {
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.parameters,
                }
            })
        }).collect();

        // 无工具时不携带 tools/tool_choice：部分模型（如 vLLM/GLM）拒绝空数组
        let mut body = serde_json::json!({
            "model": self.model,
            "messages": messages_json,
        });
        if !tools.is_empty() {
            body["tools"] = serde_json::json!(tools_json);
            body["tool_choice"] = serde_json::json!("auto");
        }

        // 针对支持思维的推理模型（如 DeepSeek 思考模型、OpenAI o1/o3 等）加入 reasoning_effort
        let is_reasoning_model = self.model.contains("deepseek") || self.model.contains("o1") || self.model.contains("o3");
        let current_effort = self.reasoning_effort.read().await.clone();
        
        if is_reasoning_model {
            if let Some(ref effort) = current_effort {
                body["reasoning_effort"] = serde_json::json!(effort);
                if self.model.contains("deepseek") {
                    body["extra_body"] = serde_json::json!({
                        "thinking": { "type": "enabled" }
                    });
                }
            }
        } else {
            body["temperature"] = serde_json::json!(0.7);
        }

        // 🔍 后端日志：请求概览
        println!(
            "🤖 [LLM] → {} | model={} | msgs={} | tools={} | thinking={:?}",
            url, self.model, messages.len(), tools.len(), current_effort
        );

        let mut attempts = 0;
        let resp = loop {
            attempts += 1;
            let request_fut = self.client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.api_key.expose_secret()))
                .header("Content-Type", "application/json")
                .json(&body)
                .send();
                
            match request_fut.await {
                Ok(r) => {
                    let status = r.status();
                    if (status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error()) && attempts < 3 {
                        let delay = std::time::Duration::from_secs(1 << attempts);
                        println!("⚠️  [LLM] 收到状态码 {}，将在 {} 秒后重试 (第 {} 次)...", status, delay.as_secs(), attempts);
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                    break Ok(r);
                }
                Err(e) => {
                    if attempts < 3 {
                        let delay = std::time::Duration::from_secs(1 << attempts);
                        println!("⚠️  [LLM] 请求连接失败 {}，将在 {} 秒后重试 (第 {} 次)...", e, delay.as_secs(), attempts);
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                    break Err(e);
                }
            }
        }.map_err(|e| format!("LLM 请求失败: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            eprintln!("❌ [LLM] 错误 {status}: {text}");
            return Err(format!("LLM 返回错误 {status}: {text}"));
        }

        let json: serde_json::Value = resp.json().await
            .map_err(|e| format!("解析 LLM 响应失败: {e}"))?;

        let choice = json["choices"].get(0)
            .ok_or("LLM 返回无 choices")?;
        let message = &choice["message"];
        let content = message["content"].as_str().unwrap_or("").to_string();
        let finish_reason = choice["finish_reason"].as_str().unwrap_or("stop").to_string();

        // DS thinking mode: 提取 reasoning_content
        let thought = message["reasoning_content"].as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let tool_calls: Vec<ToolCallRecord> = if let Some(calls) = message["tool_calls"].as_array() {
            calls.iter().filter_map(|c| {
                let id = c["id"].as_str().unwrap_or("").to_string();
                let function = &c["function"];
                let name = function["name"].as_str().unwrap_or("").to_string();
                let args_str = function["arguments"].as_str().unwrap_or("{}");
                let args: serde_json::Value = serde_json::from_str(args_str).unwrap_or(serde_json::json!({}));
                Some(ToolCallRecord {
                    id,
                    tool_name: name,
                    arguments: args,
                    result: None,
                    status: ToolCallStatus::Pending,
                    timestamp: chrono::Utc::now(),
                })
            }).collect()
        } else {
            vec![]
        };

        let input_tokens = json["usage"]["prompt_tokens"].as_u64().unwrap_or(0);
        let output_tokens = json["usage"]["completion_tokens"].as_u64().unwrap_or(0);

        // 🔍 后端日志：响应摘要
        println!(
            "✅ [LLM] ← finish={} | content_len={} | thought_len={} | tool_calls={} | tokens in={} out={}",
            finish_reason,
            content.len(),
            thought.as_ref().map_or(0, |t| t.len()),
            tool_calls.len(),
            input_tokens,
            output_tokens,
        );

        fn sanitize_sensitive_content(text: &str) -> String {
            let re_key = regex::Regex::new(r#"(?i)(key|password|secret|token|pass|auth|credential|private_key|api_key)\s*[:=]\s*['"a-zA-Z0-9_\-\.\+:]{8,}"#).unwrap();
            re_key.replace_all(text, "$1=******").to_string()
        }

        if let Some(ref t) = thought {
            let safe_len = t.char_indices().nth(200).map(|(i, _)| i).unwrap_or(t.len());
            let redact_thought = sanitize_sensitive_content(&t[..safe_len]);
            println!("💭 [LLM] reasoning: {}", redact_thought);
        }
        if !content.is_empty() {
            let safe_len = content.char_indices().nth(300).map(|(i, _)| i).unwrap_or(content.len());
            let redact_content = sanitize_sensitive_content(&content[..safe_len]);
            println!("📝 [LLM] content: {}", redact_content);
        }
        for tc in &tool_calls {
            println!("🔧 [LLM] tool_call: {} ({})", tc.tool_name, tc.id);
        }

        Ok(LlmResponse {
            content,
            thought,
            tool_calls,
            input_tokens,
            output_tokens,
            finish_reason,
        })
    }

    fn name(&self) -> &str { "openai" }

    fn estimate_cost(&self, input: u64, output: u64) -> f64 {
        // DS v4 定价: ¥2/1M input, ¥4/1M output（在 agent_manager CostConfig 中配置为元）
        (input as f64 * 2.0 / 1_000_000.0) + (output as f64 * 4.0 / 1_000_000.0)
    }
}