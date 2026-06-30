# JC9 后端规范

## 技术选型

| 项目 | 规范 |
|------|------|
| 语言 | Rust (edition 2021) |
| 构建 | Cargo |
| 异步 | Tokio (full features) |
| 数据库 | rusqlite (bundled + load_extension) |
| Web 框架 | Tauri 2 |
| HTTP | reqwest |
| AST | Tree-sitter |

## 代码风格

- 遵循 `rustfmt` 默认格式
- 遵循 clippy 建议（`cargo clippy`）
- 所有公有函数添加文档注释 `///`
- 错误处理使用 `Result<T, String>`（返回给 Tauri 前端）

## 文件组织

```
src-tauri/src/
  main.rs           ← 应用入口，仅启动
  lib.rs            ← Tauri commands + AppState + run()
  database.rs       ← SQLite 数据库（表管理 + CRUD）
  process.rs        ← 终端进程管理
  ai/               ← AI Agent 子系统
    mod.rs          ← 模块导出
    types.rs        ← 核心类型
    agent_manager.rs← 聚合入口
    ...             ← 各模块
```

## Tauri Command 规范

### 添加新 command 步骤

```rust
// 1. 在 lib.rs 中定义函数
#[tauri::command]
fn your_command(state: State<'_, Mutex<AppState>>, param1: String) -> Result<String, String> {
    let app_state = state.lock().map_err(|e| e.to_string())?;
    // 访问 db
    app_state.db.some_method(&param1)
    // 或访问 ai_manager
    // let ai = app_state.ai_manager.clone();
    // drop(app_state); // 提前释放锁
    // ai.some_method().await;
}

// 2. 在 invoke_handler! 中注册
.invoke_handler(tauri::generate_handler![
    your_command,  // ← 必须加在这里
])
```

### 同步 vs 异步

- 简单数据库读取：同步 `fn`（因为 `Mutex::lock` 是同步的）
- 涉及 AI Agent 或网络请求：`async fn`
- 异步命令中要先释放 `AppState` 锁再 await，避免死锁：

```rust
#[tauri::command]
async fn async_command(state: State<'_, Mutex<AppState>>) -> Result<(), String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.ai_manager.clone() // 只克隆 Arc，不持有锁
    };
    // 此时 AppState 锁已释放
    ai_manager.some_async_method().await;
    Ok(())
}
```

## 数据库操作规范

```rust
// 1. 获取连接
let conn = self.conn.lock().map_err(|e| e.to_string())?;

// 2. 参数化查询（禁止拼接 SQL）
conn.execute(
    "INSERT INTO table (id, name) VALUES (?1, ?2)",
    params![id, name],
).map_err(|e| e.to_string())?;

// 3. 查询结果映射
let mut stmt = conn.prepare("SELECT id, name FROM table").map_err(|e| e.to_string())?;
let results = stmt.query_map([], |row| {
    Ok(SomeStruct {
        id: row.get(0)?,
        name: row.get(1)?,
    })
}).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();
```

## AI Agent 模块规范

### 添加新工具步骤

1. 在 `tools.rs` 中实现 `Tool` trait
2. 定义 `ToolDefinition`（名称、描述、参数 JSON Schema）
3. 在 `AgentManager::new()` 中注册到 `ToolRegistry`
4. 在 `approval.rs` 中标注安全风险等级
5. 运行 `cargo check` 验证

### 添加新 MCP 服务器

在 `mcp_client.rs` 中注册新服务器：
```rust
// SSE 传输
mcp_client.connect("server-name", url).await

// stdio 传输
mcp_client.connect_stdio("server-name", "npx", &["@modelcontextprotocol/server-filesystem", "."]).await
```

## 编译检查清单

每次修改后依次检查：

```bash
# 1. Rust 编译
cd src-tauri && cargo check

# 2. Clippy（可选但建议）
cd src-tauri && cargo clippy

# 3. 前端类型检查
npx vue-tsc --noEmit

# 4. 完整 Tauri 构建（仅发布前）
cd src-tauri && cargo build
```
