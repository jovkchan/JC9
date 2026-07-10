use rusqlite::params;
use std::sync::{Arc, Mutex};
use super::mcp_server::McpServerConfig;

/// 从数据库加载 MCP Server 配置
pub fn load_mcp_config(conn: &Arc<Mutex<rusqlite::Connection>>) -> Result<Option<McpServerConfig>, String> {
    let conn = conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = 'mcp_server_config'")
        .map_err(|e| e.to_string())?;

    let result: Option<String> = stmt.query_row([], |row| row.get(0)).ok();

    match result {
        Some(json_str) => {
            let config: McpServerConfig = serde_json::from_str(&json_str)
                .map_err(|e| format!("解析 MCP 配置失败: {}", e))?;
            println!("📂 加载 MCP 配置: group_ids={:?}", config.group_ids);
            Ok(Some(config))
        }
        None => Ok(None),
    }
}

/// 保存 MCP Server 配置到数据库
pub fn save_mcp_config(conn: &Arc<Mutex<rusqlite::Connection>>, config: &McpServerConfig) -> Result<(), String> {
    let json_str = serde_json::to_string(config)
        .map_err(|e| format!("序列化 MCP 配置失败: {}", e))?;
    println!("💾 保存 MCP 配置: group_ids={:?}", config.group_ids);

    let conn = conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES ('mcp_server_config', ?1)",
        params![json_str],
    ).map_err(|e| e.to_string())?;

    Ok(())
}
