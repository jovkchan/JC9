---
name: database-migration
description: SQLite 数据库迁移的完整流程
type: workflow
scope: dev
version: 1
---

# 数据库迁移工作流

## 步骤

### Step 1: 在 `create_tables()` 中添加新表

```rust
// database.rs
conn.execute_batch("
    CREATE TABLE IF NOT EXISTS new_table (
        id TEXT PRIMARY KEY,
        field1 TEXT NOT NULL,
        field2 INTEGER NOT NULL DEFAULT 0,
        created_at TEXT NOT NULL
    );
").map_err(|e| format!("{e}"))?;
```

### Step 2: 给已有表加新列

```rust
let _ = conn.execute(
    "ALTER TABLE existing_table ADD COLUMN new_field TEXT NOT NULL DEFAULT ''",
    [],
);
```

`let _ =` 忽略错误是因为列可能已存在。

### Step 3: 定义 Rust struct

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewStruct {
    pub id: String,
    pub field1: String,
    pub field2: i32,
    pub created_at: String,
}
```

### Step 4: 添加 CRUD 方法

```rust
impl Database {
    pub fn get_all(&self) -> Result<Vec<NewStruct>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT id, field1, field2, created_at FROM new_table")
            .map_err(|e| e.to_string())?;
        let items = stmt.query_map([], |row| {
            Ok(NewStruct {
                id: row.get(0)?,
                field1: row.get(1)?,
                field2: row.get(2)?,
                created_at: row.get(3)?,
            })
        }).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();
        Ok(items)
    }
}
```

### Step 5: 添加 Tauri command（如需前端调用）

见 `workflows/add-new-command.md`

### Step 6: 验证

```bash
cd src-tauri && cargo check
```
