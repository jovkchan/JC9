use async_trait::async_trait;
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
    pub tool_calls: Vec<ToolCallRecord>,
}

impl LlmMessage {
    pub fn system(content: String) -> Self {
        Self { role: MessageRole::System, content, tool_calls: vec![] }
    }
    pub fn user(content: String) -> Self {
        Self { role: MessageRole::User, content, tool_calls: vec![] }
    }
    pub fn assistant(content: String) -> Self {
        Self { role: MessageRole::Assistant, content, tool_calls: vec![] }
    }
    pub fn tool(content: String) -> Self {
        Self { role: MessageRole::Tool, content, tool_calls: vec![] }
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

/// OpenAI 兼容 Provider
pub struct OpenAiProvider {
    api_key: String,
    base_url: String,
    model: String,
    client: reqwest::Client,
}

impl OpenAiProvider {
    pub fn new(api_key: String, base_url: Option<String>, model: Option<String>) -> Self {
        Self {
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.openai.com/v1".into()),
            model: model.unwrap_or_else(|| "gpt-4o".into()),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn chat(&self, messages: &[LlmMessage], tools: &[ToolDefinition]) -> Result<LlmResponse, String> {
        let url = format!("{}/chat/completions", self.base_url);

        let messages_json: Vec<serde_json::Value> = messages.iter().map(|m| {
            serde_json::json!({
                "role": match m.role {
                    MessageRole::System => "system",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::Tool => "tool",
                },
                "content": m.content,
            })
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

        let body = serde_json::json!({
            "model": self.model,
            "messages": messages_json,
            "tools": tools_json,
            "tool_choice": "auto",
        });

        let resp = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("LLM 请求失败: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("LLM 返回错误 {status}: {text}"));
        }

        let json: serde_json::Value = resp.json().await
            .map_err(|e| format!("解析 LLM 响应失败: {e}"))?;

        let choice = json["choices"].get(0)
            .ok_or("LLM 返回无 choices")?;
        let message = &choice["message"];
        let content = message["content"].as_str().unwrap_or("").to_string();
        let finish_reason = choice["finish_reason"].as_str().unwrap_or("stop").to_string();

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

        Ok(LlmResponse {
            content,
            thought: None,
            tool_calls,
            input_tokens,
            output_tokens,
            finish_reason,
        })
    }

    fn name(&self) -> &str { "openai" }

    fn estimate_cost(&self, input: u64, output: u64) -> f64 {
        // GPT-4o 定价: $2.50/1M input, $10.00/1M output
        (input as f64 * 2.5 / 1_000_000.0) + (output as f64 * 10.0 / 1_000_000.0)
    }
}