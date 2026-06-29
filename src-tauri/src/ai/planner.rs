use std::sync::Arc;
use super::llm::{LlmProvider, LlmMessage};
use super::types::{TaskNode, TaskStatus};
use super::blackboard::SharedBlackboard;
use chrono::Utc;

/// 任务拆解与 Planner 代理 (大模型驱动与优雅降级实现)
pub struct Planner;

impl Planner {
    /// 规划任务，如果大模型支持则异步分析拆解，否则自动回退至预设快照
    pub async fn plan(
        provider: Arc<dyn LlmProvider>,
        blackboard: Arc<SharedBlackboard>,
        session_id: String,
        request: String,
    ) -> Vec<TaskNode> {
        // 如果是 mock，不调用大模型直接返回预设
        if provider.name() == "mock" {
            return Self::generate_mock_plan(session_id, request);
        }

        // 从黑板中拉取所有历史 Worker 生成的经验沉淀 (Takeaways)，以实现反思与任务树自适应调整
        let mut takeaways_context = String::new();
        let blackboard_entries = blackboard.get_all().await;
        if !blackboard_entries.is_empty() {
            takeaways_context.push_str("\n\n【已发生的历史任务运行经验 (Takeaways，用于自适应修正后面的规划任务)】:\n");
            for entry in blackboard_entries {
                takeaways_context.push_str(&format!(
                    "- 来源 Worker {}: {}\n",
                    entry.source_worker, entry.value
                ));
            }
        }

        let system_prompt = r#"你是一个专业的任务分解与规划 Agent。
你的任务是将用户提供的开发需求拆解为结构化的任务树（最多包含一个根任务和若干子任务节点）。

你必须直接返回符合以下 JSON 结构的数组，不要包含任何 markdown 标记、普通解释文本或 ```json 包裹：
[
  {
    "id": "随机唯一的uuid字符串",
    "parentId": null, // 根任务写 null，子任务写父任务的 id
    "sessionId": "会话 id，由用户提供",
    "title": "任务标题（如 '编写阶乘核心逻辑'）",
    "description": "任务的具体描述及执行步骤",
    "status": "pending", // 可选: pending, inprogress, blocked, completed, failed
    "priority": 1, // 优先级：数字越小越优先，从 1 开始
    "assignedWorker": null,
    "subTasks": [], // 子任务的 id 数组，非子任务填空数组
    "createdAt": "当前时间，由用户提供",
    "updatedAt": "当前时间，由用户提供",
    "result": null
  }
]
确保整个返回数据符合标准 JSON 格式，且字段名称采用 camelCase。"#.to_string();

        let user_message = format!(
            "会话ID: {}\n当前时间: {}\n开发需求: {}\n{}请进行结构化任务拆解并直接返回 JSON 数组。",
            session_id, Utc::now().to_rfc3339(), request, takeaways_context
        );

        let messages = vec![
            LlmMessage::system(system_prompt),
            LlmMessage::user(user_message),
        ];

        match provider.chat(&messages, &[]).await {
            Ok(response) => {
                let cleaned = response.content
                    .trim()
                    .trim_start_matches("```json")
                    .trim_end_matches("```")
                    .trim()
                    .to_string();

                match serde_json::from_str::<Vec<TaskNode>>(&cleaned) {
                    Ok(mut nodes) => {
                        for node in &mut nodes {
                            node.session_id = session_id.clone();
                        }
                        nodes
                    }
                    Err(e) => {
                        println!("解析大模型 Planner 响应失败: {e}。内容为: {}。降级为预设规划树。", response.content);
                        Self::generate_mock_plan(session_id, request)
                    }
                }
            }
            Err(e) => {
                println!("大模型规划调用失败: {e}。降级为预设规划树。");
                Self::generate_mock_plan(session_id, request)
            }
        }
    }

    fn generate_mock_plan(session_id: String, request: String) -> Vec<TaskNode> {
        let root_id = uuid::Uuid::new_v4().to_string();
        let sub1_id = uuid::Uuid::new_v4().to_string();
        let sub2_id = uuid::Uuid::new_v4().to_string();

        let root_node = TaskNode {
            id: root_id.clone(),
            parent_id: None,
            session_id: session_id.clone(),
            title: format!("规划主任务: {}", request),
            description: format!("处理用户的原始需求并规划执行方案: {}", request),
            status: TaskStatus::InProgress,
            priority: 1,
            assigned_worker: None,
            sub_tasks: vec![sub1_id.clone(), sub2_id.clone()],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            result: None,
        };

        let sub1 = TaskNode {
            id: sub1_id,
            parent_id: Some(root_id.clone()),
            session_id: session_id.clone(),
            title: "① 扫描与感知宿主环境".into(),
            description: "获取系统环境变量（已安全脱敏）及常用 CLI 工具的版本信息".into(),
            status: TaskStatus::Pending,
            priority: 2,
            assigned_worker: None,
            sub_tasks: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            result: None,
        };

        let sub2 = TaskNode {
            id: sub2_id,
            parent_id: Some(root_id),
            session_id,
            title: "② 分析和编辑核心代码".into(),
            description: "读取目标文件并进行修改与安全沙箱执行".into(),
            status: TaskStatus::Pending,
            priority: 2,
            assigned_worker: None,
            sub_tasks: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            result: None,
        };

        vec![root_node, sub1, sub2]
    }
}
