use std::sync::Arc;
use super::llm::{LlmProvider, LlmMessage};
use super::types::ReActStep;

/// 总结代理 (Summarizer) - 负责提炼 Worker 运行轨迹并生成 Takeaways 经验沉淀
pub struct Summarizer;

impl Summarizer {
    /// 分析 ReAct 轨迹，提取核心更改和修复逻辑
    pub async fn summarize_run(
        provider: Arc<dyn LlmProvider>,
        history: &[ReActStep],
    ) -> Result<String, String> {
        if provider.name() == "mock" {
            return Ok("【Mock 总结】: 顺利完成了代码扫描与修复，未发现遗留 Bug。".to_string());
        }

        if history.is_empty() {
            return Ok("未执行任何有效步骤。".to_string());
        }

        let mut history_str = String::new();
        for step in history {
            history_str.push_str(&format!(
                "--- 迭代第 {} 轮 ---\n【Thought】: {}\n",
                step.iteration, step.thought
            ));
            if let Some(ref action) = step.action {
                history_str.push_str(&format!(
                    "【Action】: 运行工具 '{}', 参数: {}\n",
                    action.tool_name, action.arguments
                ));
            }
            if let Some(ref obs) = step.observation {
                // 做适量截断防止 Prompt 过长
                let truncated_obs = if obs.len() > 1000 {
                    format!("{}... (内容过长已截断)", &obs[..1000])
                } else {
                    obs.clone()
                };
                history_str.push_str(&format!("【Observation】: {}\n", truncated_obs));
            }
        }

        let system_prompt = r#"你是一个资深的代码审查与总结 Agent。
请阅读下面的 Worker 迭代轨迹（Thought-Action-Observation 记录），精炼地提取出此次修改中已解决的问题、涉及的文件位置及任何需要共享给其他并发代理的避坑常识 (Takeaways)。

你的输出必须直接是简炼的结论条目列表，如：
- 修复了 src/main.rs 第 20 行因未进行空校验引起的 NullPointerException 风险。
- 外部依赖包 rocket 已通过 cargo build 成功下载编译。

绝对不要包含任何前言、总结说明、解释及 markdown 标记包裹。"#.to_string();

        let user_message = format!(
            "这是该 Worker 运行历史记录：\n{}\n请输出经验沉淀：",
            history_str
        );

        let messages = vec![
            LlmMessage::system(system_prompt),
            LlmMessage::user(user_message),
        ];

        match provider.chat(&messages, &[]).await {
            Ok(res) => Ok(res.content.trim().to_string()),
            Err(e) => Err(e.to_string()),
        }
    }
}
