# JC9 AI Agent 架构

## 子系统一览

JC9 的 AI Agent 由 20 个 Rust 模块组成：

| # | 模块 | 文件 | 职责 |
|---|------|------|------|
| 1 | 类型系统 | `types.rs` | 会话、消息、任务、Worker、黑板、审批等核心类型 |
| 2 | LLM 提供者 | `llm.rs` | OpenAI 兼容 API / Mock LLM 双模式 |
| 3 | 工具注册表 | `tools.rs` | 工具注册、调用、生命周期管理 |
| 4 | ReAct 循环 | `react_loop.rs` | Thought → Action → Observation 推理闭环 |
| 5 | 知识库 | `knowledge_base.rs` | 知识条目 CRUD + TF-IDF 搜索 + 置信度 |
| 6 | 向量存储 | `vector_store.rs` | sqlite-vec / 余弦相似度 语义搜索 |
| 7 | Planner | `planner.rs` | 任务分解（Mock 降级 / LLM 拆解） |
| 8 | Worker 管理 | `worker_manager.rs` | 并发调度、限流、生命周期 |
| 9 | 审批队列 | `approval.rs` | 高危操作聚合审批 + 超时 |
| 10 | 黑板 | `blackboard.rs` | Worker 间结构化数据共享 |
| 11 | 安全沙箱 | `security.rs` | 路径校验、命令黑白名单、越界拦截 |
| 12 | 熔断器 | `loop_breaker.rs` | 死循环检测 + 强制中断 |
| 13 | 宿主检测 | `host_detector.rs` | 环境采集 + 敏感变量脱敏 |
| 14 | 摘要器 | `summarizer.rs` | 运行轨迹分析 + Takeaways 沉淀 |
| 15 | COW 工作区 | `workspace.rs` | 隔离沙箱 + 三向合并 |
| 16 | 三向合并 | `diff_merge.rs` | 行级 + 结构性冲突合并 |
| 17 | MCP 客户端 | `mcp_client.rs` | MCP 协议通信 |
| 18 | AST 解析 | `ast_parser.rs` | Tree-sitter 符号提取 |
| 19 | Agent 管理器 | `agent_manager.rs` | 聚合入口，所有子系统的持有者 |
| 20 | 技能加载器 | `skill_loader.rs` | 将 `.jc9/` 技能文件同步到知识库 |

## ReAct 循环工作流程

```
用户输入
    │
    ▼
┌─────────────────────────────────────────────┐
│ 1. System Prompt 组装                        │
│    ├─ 身份定义                              │
│    ├─ 工具列表（从 ToolRegistry 自动生成）   │
│    ├─ 宿主环境（从 HostDetector）           │
│    ├─ Repo Map（项目结构感知）              │
│    ├─ 知识库上下文（从 KnowledgeBase）      │
│    ├─ 相关技能（从 knowledge 表中按需搜索）  │
│    └─ 安全约束 + 成本提醒                   │
└─────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────┐
│ 2. ReAct 循环                               │
│    ├─ Thought: 分析当前状态，决定下一步     │
│    ├─ Action: 调用工具（经 ToolRegistry）   │
│    │   ├─ 低风险 → 直接执行                │
│    │   ├─ 高风险 → 审批队列 → 用户确认     │
│    │   └─ 超限 → 成本熔断                  │
│    ├─ Observation: 获取工具返回结果         │
│    ├─ LoopBreaker: 检查是否陷入死循环       │
│    └─ 重复直到任务完成或达到上限           │
└─────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────┐
│ 3. 总结沉淀                                 │
│    ├─ Summarizer 分析运行轨迹               │
│    └─ Takeaways 写入知识库（草稿箱）        │
└─────────────────────────────────────────────┘
```

## ToolRegistry 工作方式

```rust
// 工具定义结构
pub struct ToolDefinition {
    pub name: String,          // 工具名称
    pub description: String,   // 工具描述
    pub parameters: JsonSchema,// 参数 JSON Schema
    pub risk_level: RiskLevel, // Low | Medium | High | Critical
}

// 风险等级规则
// Low:    read_file, list_dir, glob_search          → 直接放行
// Medium: grep_search, find_references              → 日志记录
// High:   write_file, apply_diff, run_command       → 审批队列
// Critical: terminal_input, delete_file             → 审批 + 确认
```

## 添加新工具完整流程

```rust
// 1. tools.rs 中实现 Tool trait
struct MyTool;
#[async_trait]
impl Tool for MyTool {
    fn name(&self) -> &str { "my_tool" }
    fn description(&self) -> &str { "工具描述" }
    fn parameters(&self) -> serde_json::Value { /* JSON Schema */ }
    async fn execute(&self, args: serde_json::Value) -> Result<String, String> {
        // 工具逻辑
    }
}

// 2. AgentManager::new() 中注册
let tool_registry = Arc::new(ToolRegistry::new());
tool_registry.register("my_tool", MyTool, RiskLevel::Low);

// 3. tools.rs 的 get_tool_definitions() 中返回定义
definitions.push(ToolDefinition {
    name: "my_tool".into(),
    description: "工具描述".into(),
    parameters: /* JSON Schema */,
    risk_level: RiskLevel::Low,
});
```

## 知识库同步机制

```
笔记保存 → save_note() → db.save_note() + kb.add_entry(note_to_kb_entry())

技能文件 → skill_loader.rs → 解析 YAML → kb.add_entry(skill_entry())

Agent 沉淀 → Summarizer → kb.add_entry(takeaway_entry())

删除笔记 → permanently_delete_note() → kb.remove_entry("note_{id}")
```

## 技能加载机制（运行期 Agent 如何用技能）

```
应用启动
  │
  ▼
skill_loader::scan_skills_dir(".jc9/")
  │
  ▼
解析每个 SKILL.md 的 YAML frontmatter
  │
  ▼
过滤 scope: runtime 或 scope: both
  │
  ▼
写入 knowledge 表，生成向量嵌入
  │
  ▼
用户提问时 Agent 对问题做语义搜索
  │
  ▼
命中技能 → 技能内容注入 System Prompt
  │
  ▼
Agent 按技能步骤执行操作
```

## MCP 集成

MCP 工具通过 `mcp_{server_name}_{tool_name}` 格式注册到 ToolRegistry，Agent 可以像调用本地工具一样调用 MCP 远程工具。

```rust
// 连接 MCP 服务器
mcp_client.connect("my-server", "https://example.com/mcp").await

// 工具自动注册
// → ToolRegistry 中出现 "mcp_my_server_list_files"
// → Agent 可调用
```
