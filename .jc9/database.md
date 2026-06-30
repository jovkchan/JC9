# JC9 数据库规范

## 基本信息

- **数据库文件**：`%APPDATA%/jc9/jc9.db`（Windows）
- **引擎**：SQLite 3，WAL 模式
- **扩展**：sqlite-vec (`vec0.dll`)，用于向量搜索
- **管理类**：`src-tauri/src/database.rs`（主库）、`src-tauri/src/ai/knowledge_base.rs`（知识库）
- **连接共享**：`Arc<Mutex<Connection>>`，`Database` 和 `KnowledgeBase` 共享同一连接

## 所有表清单

| 表名 | 用途 | 管理类 | 重要字段 |
|------|------|--------|---------|
| `settings` | 键值对设置 | Database | `key`, `value` |
| `users` | 用户 | Database | `id`, `username` |
| `projects` | 项目 | Database | `id`, `name`, `created_at` |
| `project_commands` | 项目命令 | Database | `project_id`, `command`, `working_dir` |
| `shortcuts` | 快捷键 | Database | `name`, `command`, `category`, `is_favorite` |
| `note_groups` | 笔记分组 | Database | `name`, `parent_id`, `sort_order` |
| `notes` | 笔记 | Database | `title`, `content`, `tags`, `is_deleted`, `is_archived` |
| `tags` | 标签 | Database | `name`, `color` |
| `resources` | 资源文件 | Database | `note_id`, `filename`, `file_path` |
| `sync_log` | 同步日志 | Database | `table_name`, `action`, `version` |
| `knowledge` | AI 知识条目 | KnowledgeBase | `title`, `content`, `entry_type`, `confidence`, `is_draft` |
| `ai_sessions` | AI 会话 | KnowledgeBase | `title`, `status`, `token_count`, `cost_usd` |
| `embeddings` | 向量嵌入 | KnowledgeBase | `source_id`, `content`, `embedding` (BLOB) |
| `vec_embeddings` | sqlite-vec 虚拟表 | VectorStore | `embedding float[1536]` |

## 关键规范

### 时间格式
所有时间字段使用 **RFC3339 格式字符串**（如 `2026-06-30T10:15:30+08:00`）。

```rust
// 写入
Utc::now().to_rfc3339()
// 读取
DateTime::parse_from_rfc3339(&s).map(|d| d.with_timezone(&Utc))
```

### 软删除
`notes` 表使用 `is_deleted INTEGER` 字段做软删除：
- `is_deleted = 0`：正常
- `is_deleted = 1`：已删除
- 查询时默认 `WHERE is_deleted = 0`
- 永久删除才执行 `DELETE FROM`

### 向量嵌入
`embeddings.embedding` 字段存储为 BLOB：
```rust
// 序列化：f32 数组 → 小端字节数组
let blob: Vec<u8> = embedding.iter()
    .flat_map(|f| f.to_le_bytes())
    .collect();

// 反序列化：小端字节数组 → f32 数组
let stored: Vec<f32> = blob.chunks_exact(4)
    .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
    .collect();
```

### 笔记→知识库同步
笔记自动同步到知识库使用固定 ID 前缀：
```rust
// database.rs save_note() 中自动执行
let kb_id = format!("note_{}", note.id);
// 保证 ID 不冲突且可追溯
```

### 技能→知识库同步
`.jc9/` 下的技能文件同步到知识库使用固定 ID 前缀：
```rust
let skill_id = format!("skill_{}", skill_name);
```

## 添加新表完整步骤

```rust
// 1. database.rs 的 create_tables() 中添加
conn.execute_batch("
    CREATE TABLE IF NOT EXISTS your_table (
        id TEXT PRIMARY KEY,
        ...
    );
").map_err(|e| format!("{e}"))?;

// 2. 如果有新字段要加到已有表
let _ = conn.execute("ALTER TABLE notes ADD COLUMN new_field TEXT NOT NULL DEFAULT ''", []);

// 3. 定义对应的 Rust struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YourStruct {
    pub id: String,
    // ...
}

// 4. 在 Database impl 中添加 CRUD 方法
impl Database {
    pub fn your_method(&self) -> Result<Vec<YourStruct>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        // ...
    }
}

// 5. 在 lib.rs 中添加 Tauri command（如果需要前端调用）
#[tauri::command]
fn your_command(state: State<'_, Mutex<AppState>>, ...) -> Result<..., String> {
    state.lock().map_err(|e| e.to_string())?.db.your_method(...)
}

// 6. 在 invoke_handler! 中注册
.invoke_handler(tauri::generate_handler![
    your_command,  // ← 加在这里
])
```
