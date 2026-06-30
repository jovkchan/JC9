use std::sync::{Arc, Mutex};
use chrono::Utc;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 追踪事件类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TraceEventType {
    SessionCreated,
    SessionCompleted,
    TaskPlanned,
    TaskCompleted,
    WorkerSpawned,
    WorkerKilled,
    ToolInvocation,
    ToolResult,
    CheckpointSaved,
    CheckpointRestored,
    CostUpdated,
    CostExceeded,
    Thought,
    Observation,
    McpServerConnected,
    McpServerDisconnected,
}

impl TraceEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SessionCreated => "session_created",
            Self::SessionCompleted => "session_completed",
            Self::TaskPlanned => "task_planned",
            Self::TaskCompleted => "task_completed",
            Self::WorkerSpawned => "worker_spawned",
            Self::WorkerKilled => "worker_killed",
            Self::ToolInvocation => "tool_invocation",
            Self::ToolResult => "tool_result",
            Self::CheckpointSaved => "checkpoint_saved",
            Self::CheckpointRestored => "checkpoint_restored",
            Self::CostUpdated => "cost_updated",
            Self::CostExceeded => "cost_exceeded",
            Self::Thought => "thought",
            Self::Observation => "observation",
            Self::McpServerConnected => "mcp_server_connected",
            Self::McpServerDisconnected => "mcp_server_disconnected",
        }
    }
}

/// 追踪事件记录
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceEvent {
    pub id: String,
    pub session_id: String,
    pub worker_id: Option<String>,
    pub event_type: String,
    pub event_data: Value,
    pub created_at: String,
}

/// 事件追踪器 — 将 Agent 执行链持久化到 SQLite
///
/// 追踪所有关键事件（会话、任务、Worker、工具调用、checkpoint、成本），
/// 支持前端 TraceViewer 回溯完整执行过程。
pub struct Tracer {
    db_conn: Option<Arc<Mutex<Connection>>>,
}

impl Tracer {
    pub fn new(db_conn: Option<Arc<Mutex<Connection>>>) -> Self {
        Self { db_conn }
    }

    /// 记录追踪事件
    pub async fn record(&self, session_id: &str, worker_id: Option<&str>, event_type: TraceEventType, event_data: Value) {
        let id = uuid::Uuid::new_v4().to_string();
        let ev = TraceEvent {
            id: id.clone(),
            session_id: session_id.to_string(),
            worker_id: worker_id.map(|s| s.to_string()),
            event_type: event_type.as_str().to_string(),
            event_data,
            created_at: Utc::now().to_rfc3339(),
        };

        if let Some(ref conn_arc) = self.db_conn {
            if let Ok(conn) = conn_arc.lock() {
                let _ = conn.execute(
                    "INSERT INTO tracing_events (id, session_id, worker_id, event_type, event_data, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![
                        ev.id,
                        ev.session_id,
                        ev.worker_id,
                        ev.event_type,
                        serde_json::to_string(&ev.event_data).unwrap_or_default(),
                        ev.created_at,
                    ],
                );
            }
        }
    }

    /// 按 session_id 查询事件链
    pub async fn get_session_events(&self, session_id: &str) -> Vec<TraceEvent> {
        let mut events = Vec::new();
        if let Some(ref conn_arc) = self.db_conn {
            if let Ok(conn) = conn_arc.lock() {
                let mut stmt = match conn.prepare(
                    "SELECT id, session_id, worker_id, event_type, event_data, created_at FROM tracing_events WHERE session_id = ?1 ORDER BY created_at ASC"
                ) {
                    Ok(s) => s,
                    Err(_) => return events,
                };
                let result = stmt.query_map(rusqlite::params![session_id], |row| {
                    let data_str: String = row.get(4)?;
                    Ok(TraceEvent {
                        id: row.get(0)?,
                        session_id: row.get(1)?,
                        worker_id: row.get(2)?,
                        event_type: row.get(3)?,
                        event_data: serde_json::from_str(&data_str).unwrap_or_default(),
                        created_at: row.get(5)?,
                    })
                });
                if let Ok(rows) = result {
                    for row in rows {
                        if let Ok(ev) = row { events.push(ev); }
                    }
                }
            }
        }
        events
    }

    /// 按 event_type 过滤查询
    pub async fn get_events_by_type(&self, session_id: &str, event_type: &str) -> Vec<TraceEvent> {
        let mut events = Vec::new();
        if let Some(ref conn_arc) = self.db_conn {
            if let Ok(conn) = conn_arc.lock() {
                let mut stmt = match conn.prepare(
                    "SELECT id, session_id, worker_id, event_type, event_data, created_at FROM tracing_events WHERE session_id = ?1 AND event_type = ?2 ORDER BY created_at ASC"
                ) {
                    Ok(s) => s,
                    Err(_) => return events,
                };
                let result = stmt.query_map(rusqlite::params![session_id, event_type], |row| {
                    let data_str: String = row.get(4)?;
                    Ok(TraceEvent {
                        id: row.get(0)?,
                        session_id: row.get(1)?,
                        worker_id: row.get(2)?,
                        event_type: row.get(3)?,
                        event_data: serde_json::from_str(&data_str).unwrap_or_default(),
                        created_at: row.get(5)?,
                    })
                });
                if let Ok(rows) = result {
                    for row in rows {
                        if let Ok(ev) = row { events.push(ev); }
                    }
                }
            }
        }
        events
    }

    /// 获取最近的 N 条事件（全局）
    pub async fn get_recent_events(&self, limit: usize) -> Vec<TraceEvent> {
        let mut events = Vec::new();
        if let Some(ref conn_arc) = self.db_conn {
            if let Ok(conn) = conn_arc.lock() {
                let mut stmt = match conn.prepare(
                    "SELECT id, session_id, worker_id, event_type, event_data, created_at FROM tracing_events ORDER BY created_at DESC LIMIT ?1"
                ) {
                    Ok(s) => s,
                    Err(_) => return events,
                };
                let result = stmt.query_map(rusqlite::params![limit as i64], |row| {
                    let data_str: String = row.get(4)?;
                    Ok(TraceEvent {
                        id: row.get(0)?,
                        session_id: row.get(1)?,
                        worker_id: row.get(2)?,
                        event_type: row.get(3)?,
                        event_data: serde_json::from_str(&data_str).unwrap_or_default(),
                        created_at: row.get(5)?,
                    })
                });
                if let Ok(rows) = result {
                    for row in rows {
                        if let Ok(ev) = row { events.push(ev); }
                    }
                }
            }
        }
        events
    }
}
