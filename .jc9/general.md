# JC9 核心规则

## 项目简介

JC9 是一个本地项目管理与终端工具，基于 Tauri 2 构建，前端 Vue 3 + TypeScript，后端 Rust。内置 AI Agent 子系统（ReAct 循环 + 多 Worker 并发 + 语义知识库）。

## 技术栈

| 层 | 技术 | 关键依赖 |
|----|------|---------|
| 桌面壳 | Tauri 2 | `tauri-plugin-shell`, `tauri-plugin-dialog` |
| 前端 | Vue 3 + TypeScript + Pinia | Composition API, `<script setup>` |
| 样式 | SCSS | `variables.scss`, `mixins.scss` |
| 后端 | Rust | `rusqlite` (bundled), `tokio`, `reqwest` |
| 数据库 | SQLite (WAL) | `jc9.db` → `%APPDATA%/jc9/jc9.db` |
| 向量搜索 | sqlite-vec / 余弦相似度 | `vec0.dll` → `src-tauri/vec0.dll` |
| AI 推理 | OpenAI 兼容 API | 支持 DeepSeek / 阿里云 / Anthropic |
| AST 解析 | Tree-sitter | TypeScript, Rust, Vue SFC |

## 硬性规则（必须遵守）

### R1：修改 Rust 后端后必须 `cargo check`

```bash
cd src-tauri && cargo check
```

不要只编译单个文件，`cargo check` 会检查所有依赖和类型。

### R2：修改 Vue 组件后必须 `vue-tsc --noEmit`

```bash
npx vue-tsc --noEmit
```

确保 TypeScript 类型完全正确。

### R3：添加 SQLite 表时必须同时更新 `create_tables()`

`database.rs` 中的 `create_tables()` 方法集中管理所有表结构。在这之外任何地方单独执行 `CREATE TABLE` 都是禁止的。

**例外**：`vec_embeddings` 虚拟表依赖 sqlite-vec 扩展是否加载成功，因此在 `vector_store.rs` 中动态创建。

### R4：所有 Tauri command 必须注册到 `invoke_handler!`

在 `lib.rs` 的 `invoke_handler!` 宏中注册，否则前端无法调用。遗漏注册是最高频的错误之一。

```rust
.invoke_handler(tauri::generate_handler![
    your_new_command,  // ← 别忘了加这行
    ...
])
```

### R5：所有 AI Agent 工具必须注册到 `ToolRegistry`

在 `tools.rs` 中实现 `Tool` trait，然后在 `AgentManager::new()` 中注册。否则 Agent 无法使用该工具。

### R6：数据库连接必须通过 `Arc<Mutex<Connection>>` 共享

`Database` 和 `KnowledgeBase` 使用同一个 `Arc<Mutex<Connection>>`，不允许各自打开独立连接。

## 已知的坑

### 坑 1：`tokio::spawn` 在 Tauri setup 中不可用

```rust
// ❌ 错误：在 Tauri setup 闭包中直接调用
tokio::spawn(async { ... });

// ✅ 正确：使用 Tauri 提供的异步运行时
tauri::async_runtime::spawn(async { ... });
```

### 坑 2：笔记同步知识库使用 `note_{id}` 前缀

```rust
// 笔记 ID → 知识条目 ID 的映射规则
KbEntry.id = format!("note_{}", note.id);
// 保证唯一且可追溯
```

### 坑 3：永久删除笔记时要同步清理知识库

软删除（`is_deleted = 1`）不动知识库（可恢复），永久删除必须调用 `KnowledgeBase::remove_entry()`。

### 坑 4：sqlite-vec 可能未加载

如果 `vec0.dll` 不存在或加载失败，向量搜索会自动降级为纯 Rust 余弦相似度。这不是错误，但性能较慢。

### 坑 5：添加新 enum 值时同步更新所有 match 分支

如在 `KbEntryType`、`SessionStatus` 等 enum 中加新值，必须检查所有 `match` 或 `if let` 分支是否已覆盖。

## 搜索排除规则

以下目录包含构建产物或生成代码，搜索时应排除：

| 目录 | 原因 |
|------|------|
| `target/` | Rust 编译输出 |
| `src-tauri/target/` | Rust 编译输出 |
| `node_modules/` | JS 依赖 |
| `dist/` | 构建产物 |
| `src-tauri/gen/` | Tauri 生成代码 |
| `src-tauri/icons/` | 图标资源 |

搜索文件时指定路径：`search_files(path="src", pattern="xxx", file_pattern="*.ts")`。
