---
name: add-new-tool
description: 给 AI Agent 添加一个新工具的完整流程
type: workflow
scope: dev
version: 1
---

# 添加新工具工作流

## 步骤

### Step 1: 在 `tools.rs` 中定义 Tool

```rust
use async_trait::async_trait;
use serde_json::Value;
use super::tools::{Tool, ToolDefinition, RiskLevel};

pub struct MyNewTool;

#[async_trait]
impl Tool for MyNewTool {
    fn name(&self) -> &str {
        "my_new_tool"
    }

    fn description(&self) -> &str {
        "工具功能描述"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "param1": {
                    "type": "string",
                    "description": "参数说明"
                }
            },
            "required": ["param1"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String, String> {
        let param1 = args["param1"].as_str().ok_or("缺少 param1")?;
        // 工具逻辑
        Ok("执行结果".into())
    }
}
```

### Step 2: 在 AgentManager 中注册

在 `agent_manager.rs` 的 `new()` 方法中添加：

```rust
tool_registry.register(
    "my_new_tool",
    Arc::new(MyNewTool),
    RiskLevel::Low,  // Low | Medium | High | Critical
);
```

### Step 3: 标注风险等级

| 等级 | 适用工具 | 行为 |
|------|---------|------|
| Low | 只读操作（read, list, search） | 直接放行 |
| Medium | 分析操作（grep, find_refs） | 日志记录 |
| High | 写操作（write, edit, run） | 审批队列 |
| Critical | 危险操作（delete, terminal_input） | 审批 + 确认 |

### Step 4: 更新工具定义列表

在 `tools.rs` 的 `get_tool_definitions()` 中添加：

```rust
definitions.push(ToolDefinition {
    name: "my_new_tool".into(),
    description: "工具功能描述".into(),
    parameters: serde_json::json!({/* JSON Schema */}),
    risk_level: RiskLevel::Low,
});
```

### Step 5: 验证

```bash
cd src-tauri && cargo check
```
