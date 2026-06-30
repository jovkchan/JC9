---
name: sqlite-optimization
description: SQLite 性能优化指南，包括 WAL 模式、索引、连接管理
type: skill
scope: both
trigger: 需要进行数据库优化或排查性能问题时
version: 1
---

# SQLite 优化指南

## WAL 模式

JC9 默认启用 WAL（Write-Ahead Logging）模式：

```sql
PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;
PRAGMA busy_timeout=5000;
```

WAL 模式允许并发读取，写入不阻塞读取。

## 连接管理

使用 `Arc<Mutex<Connection>>` 共享连接：

```rust
// Database 和 KnowledgeBase 共享同一个连接
let conn = Arc::new(Mutex::new(Connection::open(path)));
let db = Database { conn: conn.clone() };
let kb = KnowledgeBase::new(conn);
```

注意：`Mutex` 保护的是单个连接，同一时间只有一个线程能操作数据库。

## 常见优化

### 批量插入

```rust
// 慢：逐条插入
for item in items {
    conn.execute("INSERT ...", params![item])?;
}

// 快：事务批量
let tx = conn.transaction()?;
for item in items {
    tx.execute("INSERT ...", params![item])?;
}
tx.commit()?;
```

### 查询优化

- 使用 `LIMIT` 限制返回行数
- 避免 `SELECT *`，只选需要的列
- 合理使用索引（但不要过度索引）

## sqlite-vec 扩展

- `vec0.dll` 位于 `src-tauri/vec0.dll`
- 如果加载失败，自动降级为纯 Rust 余弦相似度
- 降级后语义搜索功能不变，但性能降低 10-50 倍
- 检查状态：`ai_vec_status` command

## 常见问题

### 数据库锁死

如果出现 `database is locked` 错误：
1. 确认所有数据库操作后立即释放连接锁
2. 确认没有嵌套锁定（先锁 AppState 再锁 conn 等）
3. `busy_timeout=5000` 会自动重试等待