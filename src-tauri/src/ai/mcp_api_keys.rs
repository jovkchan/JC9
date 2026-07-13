use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyRecord {
    pub id: String,
    pub key: String,
    pub label: String,
    pub scope: String,
    pub group_ids: Vec<String>,
    pub created_at: String,
}

/// 创建 mcp_api_keys 表（在 database.rs 初始化阶段调用）
pub fn create_table(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS mcp_api_keys (
            id TEXT PRIMARY KEY,
            key TEXT NOT NULL UNIQUE,
            label TEXT NOT NULL DEFAULT '',
            scope TEXT NOT NULL DEFAULT '',
            group_ids TEXT NOT NULL DEFAULT '[]',
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    ).map_err(|e| format!("创建 mcp_api_keys 表失败: {}", e))?;
    Ok(())
}

/// 列出所有 API Keys
pub fn list_keys(conn: &Arc<Mutex<Connection>>) -> Result<Vec<ApiKeyRecord>, String> {
    let conn = conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, key, label, scope, group_ids, created_at FROM mcp_api_keys ORDER BY created_at"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        let gids: String = row.get(4)?;
        Ok(ApiKeyRecord {
            id: row.get(0)?,
            key: row.get(1)?,
            label: row.get(2)?,
            scope: row.get(3)?,
            group_ids: serde_json::from_str(&gids).unwrap_or_default(),
            created_at: row.get(5)?,
        })
    }).map_err(|e| e.to_string())?;
    let mut keys = Vec::new();
    for r in rows { keys.push(r.map_err(|e| e.to_string())?); }
    Ok(keys)
}

/// 添加 API Key
pub fn add_key(conn: &Arc<Mutex<Connection>>, key: &str, label: &str, scope: &str, group_ids: &[String]) -> Result<ApiKeyRecord, String> {
    let conn = conn.lock().map_err(|e| e.to_string())?;
    let id = uuid::Uuid::new_v4().to_string();
    let gids_json = serde_json::to_string(group_ids).unwrap_or_default();
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO mcp_api_keys (id, key, label, scope, group_ids, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, key, label, scope, gids_json, now],
    ).map_err(|e| format!("添加 Key 失败: {}", e))?;
    Ok(ApiKeyRecord { id, key: key.to_string(), label: label.to_string(), scope: scope.to_string(), group_ids: group_ids.to_vec(), created_at: now })
}

/// 更新 API Key
pub fn update_key(conn: &Arc<Mutex<Connection>>, id: &str, label: &str, scope: &str, group_ids: &[String]) -> Result<(), String> {
    let conn = conn.lock().map_err(|e| e.to_string())?;
    let gids_json = serde_json::to_string(group_ids).unwrap_or_default();
    conn.execute(
        "UPDATE mcp_api_keys SET label=?1, scope=?2, group_ids=?3 WHERE id=?4",
        params![label, scope, gids_json, id],
    ).map_err(|e| format!("更新 Key 失败: {}", e))?;
    Ok(())
}

/// 删除 API Key
pub fn delete_key(conn: &Arc<Mutex<Connection>>, id: &str) -> Result<(), String> {
    let conn = conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM mcp_api_keys WHERE id=?1", params![id])
        .map_err(|e| format!("删除 Key 失败: {}", e))?;
    Ok(())
}

/// 获取单个 Key（用于启动时验证）
pub fn get_key_by_value(conn: &Arc<Mutex<Connection>>, key_val: &str) -> Result<Option<ApiKeyRecord>, String> {
    let conn = conn.lock().map_err(|e| e.to_string())?;
    let result = conn.query_row(
        "SELECT id, key, label, scope, group_ids, created_at FROM mcp_api_keys WHERE key=?1",
        params![key_val],
        |row| {
            let gids: String = row.get(4)?;
            Ok(ApiKeyRecord {
                id: row.get(0)?,
                key: row.get(1)?,
                label: row.get(2)?,
                scope: row.get(3)?,
                group_ids: serde_json::from_str(&gids).unwrap_or_default(),
                created_at: row.get(5)?,
            })
        },
    );
    match result {
        Ok(r) => Ok(Some(r)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}
