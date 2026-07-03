use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    pub id: String,
    pub name: String,
    pub command: String,
    pub working_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub commands: Vec<Command>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Shortcut {
    pub id: String,
    pub name: String,
    pub command: String,
    pub category: String,
    pub description: String,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub use_count: i32,
    #[serde(default)]
    pub is_builtin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteGroup {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub group_id: Option<String>,
    pub title: String,
    pub content: String,
    pub format: String,
    #[serde(default)]
    pub is_pinned: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "default_visibility")]
    pub visibility: String,
    #[serde(default)]
    pub sort_order: i32,
    #[serde(default)]
    pub version: i32,
    #[serde(default)]
    pub is_deleted: bool,
    #[serde(default)]
    pub is_archived: bool,
    pub created_at: String,
    pub updated_at: String,
}

fn default_visibility() -> String { "PRIVATE".into() }

#[derive(Clone)]
pub struct Database {
    pub conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new() -> Result<Self, String> {
        let db_path = get_db_path()?;

        let need_init = !db_path.exists();
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let conn = Connection::open(&db_path).map_err(|e| format!("cannot open db: {e}"))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;")
            .map_err(|e| format!("pragma error: {e}"))?;
        let db = Database { conn: Arc::new(Mutex::new(conn)) };
        db.create_tables()?;
        db.ensure_user()?;
        // 确保固定分组"未分组"存在
        db.ensure_uncategorized_group()?;
        // 仅在首次创建数据库时执行完整迁移（包括删除 JSON 源文件）
        if need_init { db.migrate_from_json()?; }
        // 每次启动都检查 JSON 文件，将遗漏的项目合并到 SQLite
        db.sync_json_projects()?;
        // 确保 default-shortcuts.json 已写入 runtime 目录
        let quick_dir = dirs_data().join("quick");
        let shortcuts_json = quick_dir.join("default-shortcuts.json");
        if !shortcuts_json.exists() {
            if let Some(parent) = shortcuts_json.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(&shortcuts_json, include_str!("default-shortcuts.json"));
        }
        Ok(db)
    }

    fn create_tables(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS users (id TEXT PRIMARY KEY, username TEXT NOT NULL UNIQUE, nickname TEXT NOT NULL DEFAULT '', avatar_url TEXT NOT NULL DEFAULT '', created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS projects (id TEXT PRIMARY KEY, user_id TEXT NOT NULL, name TEXT NOT NULL, created_at TEXT NOT NULL, FOREIGN KEY (user_id) REFERENCES users(id));
            CREATE TABLE IF NOT EXISTS project_commands (id TEXT PRIMARY KEY, project_id TEXT NOT NULL, name TEXT NOT NULL, command TEXT NOT NULL, working_dir TEXT NOT NULL DEFAULT '', FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE);
            CREATE TABLE IF NOT EXISTS shortcuts (id TEXT PRIMARY KEY, user_id TEXT NOT NULL, name TEXT NOT NULL, command TEXT NOT NULL, category TEXT NOT NULL DEFAULT '', description TEXT NOT NULL DEFAULT '', is_favorite INTEGER NOT NULL DEFAULT 0, use_count INTEGER NOT NULL DEFAULT 0, is_builtin INTEGER NOT NULL DEFAULT 0, FOREIGN KEY (user_id) REFERENCES users(id));
            CREATE TABLE IF NOT EXISTS note_groups (id TEXT PRIMARY KEY, user_id TEXT NOT NULL, name TEXT NOT NULL, parent_id TEXT, sort_order INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, FOREIGN KEY (user_id) REFERENCES users(id));
            CREATE TABLE IF NOT EXISTS notes (id TEXT PRIMARY KEY, user_id TEXT NOT NULL, group_id TEXT, title TEXT NOT NULL DEFAULT '', content TEXT NOT NULL DEFAULT '', format TEXT NOT NULL DEFAULT 'plain', is_pinned INTEGER NOT NULL DEFAULT 0, tags TEXT NOT NULL DEFAULT '[]', visibility TEXT NOT NULL DEFAULT 'PRIVATE', sort_order INTEGER NOT NULL DEFAULT 0, version INTEGER NOT NULL DEFAULT 1, is_deleted INTEGER NOT NULL DEFAULT 0, is_archived INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, FOREIGN KEY (user_id) REFERENCES users(id), FOREIGN KEY (group_id) REFERENCES note_groups(id) ON DELETE SET NULL);
            CREATE TABLE IF NOT EXISTS tags (id TEXT PRIMARY KEY, user_id TEXT NOT NULL, name TEXT NOT NULL, color TEXT NOT NULL DEFAULT '', FOREIGN KEY (user_id) REFERENCES users(id));
            CREATE TABLE IF NOT EXISTS resources (id TEXT PRIMARY KEY, user_id TEXT NOT NULL, note_id TEXT, filename TEXT NOT NULL, file_path TEXT NOT NULL, size INTEGER NOT NULL DEFAULT 0, mime_type TEXT NOT NULL DEFAULT '', created_at TEXT NOT NULL, FOREIGN KEY (user_id) REFERENCES users(id), FOREIGN KEY (note_id) REFERENCES notes(id) ON DELETE SET NULL);
            CREATE TABLE IF NOT EXISTS sync_log (id TEXT PRIMARY KEY, table_name TEXT NOT NULL, record_id TEXT NOT NULL, action TEXT NOT NULL, version INTEGER NOT NULL, synced INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS knowledge (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                tags TEXT NOT NULL,
                entry_type TEXT NOT NULL,
                confidence REAL NOT NULL,
                is_draft INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS ai_sessions (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'active',
                project_id TEXT,
                task_description TEXT NOT NULL DEFAULT '',
                token_count INTEGER NOT NULL DEFAULT 0,
                cost_usd REAL NOT NULL DEFAULT 0.0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS embeddings (
                id TEXT PRIMARY KEY,
                source_id TEXT NOT NULL,
                content TEXT NOT NULL,
                embedding BLOB NOT NULL
            );
            CREATE TABLE IF NOT EXISTS react_checkpoints (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                worker_id TEXT NOT NULL,
                iteration INTEGER NOT NULL,
                thought TEXT NOT NULL DEFAULT '',
                action TEXT NOT NULL DEFAULT '{}',
                observation TEXT NOT NULL DEFAULT '',
                timestamp TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES ai_sessions(id)
            );
            CREATE TABLE IF NOT EXISTS mcp_servers (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                transport TEXT NOT NULL,
                url TEXT,
                command TEXT,
                args TEXT,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS tracing_events (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                worker_id TEXT,
                event_type TEXT NOT NULL,
                event_data TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_tracing_events_session ON tracing_events(session_id, created_at);
            CREATE INDEX IF NOT EXISTS idx_tracing_events_type ON tracing_events(event_type, created_at);
        ").map_err(|e| format!("create tables: {e}"))?;
        let _ = conn.execute("ALTER TABLE notes ADD COLUMN is_archived INTEGER NOT NULL DEFAULT 0", []);
        Ok(())
    }

    fn ensure_user(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("INSERT OR IGNORE INTO users (id, username, nickname, avatar_url, created_at, updated_at) VALUES ('local', 'local', 'local', '', datetime('now'), datetime('now'))", [])
            .map_err(|e| format!("ensure user: {e}"))?;
        Ok(())
    }

    fn migrate_from_json(&self) -> Result<(), String> {
        let dir = dirs_data();
        let pp = dir.join("jc9-projects.json");
        if pp.exists() {
            if let Ok(content) = fs::read_to_string(&pp) {
                if let Ok(projects) = serde_json::from_str::<Vec<Project>>(&content) {
                    let conn = self.conn.lock().map_err(|e| e.to_string())?;
                    for p in &projects {
                        conn.execute("INSERT OR IGNORE INTO projects (id, user_id, name, created_at) VALUES (?1, 'local', ?2, ?3)", params![p.id, p.name, p.created_at]).ok();
                        for c in &p.commands {
                            conn.execute("INSERT OR IGNORE INTO project_commands (id, project_id, name, command, working_dir) VALUES (?1, ?2, ?3, ?4, ?5)", params![c.id, p.id, c.name, c.command, c.working_dir]).ok();
                        }
                    }
                    let _ = fs::remove_file(&pp);
                }
            }
        }
        let sp = dir.join("jc9-shortcuts.json");
        if sp.exists() {
            if let Ok(content) = fs::read_to_string(&sp) {
                if let Ok(shortcuts) = serde_json::from_str::<Vec<Shortcut>>(&content) {
                    let conn = self.conn.lock().map_err(|e| e.to_string())?;
                    for s in &shortcuts {
                        conn.execute("INSERT OR IGNORE INTO shortcuts (id, user_id, name, command, category, description, is_builtin) VALUES (?1, 'local', ?2, ?3, ?4, ?5, 0)", params![s.id, s.name, s.command, s.category, s.description]).ok();
                    }
                    let _ = fs::remove_file(&sp);
                }
            }
        }
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("INSERT OR REPLACE INTO settings (key, value) VALUES ('json_migrated', 'true')", []).ok();
        Ok(())
    }

    /// 每次启动调用：检查 jc9-projects.json 中是否有 SQLite 里不存在的项目，合并进去
    fn sync_json_projects(&self) -> Result<(), String> {
        let dir = dirs_data();
        let pp = dir.join("jc9-projects.json");
        if !pp.exists() { return Ok(()); }
        let content = match fs::read_to_string(&pp) {
            Ok(c) => c,
            Err(_) => return Ok(()),
        };
        let json_projects: Vec<Project> = match serde_json::from_str(&content) {
            Ok(p) => p,
            Err(_) => return Ok(()),
        };
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut count = 0usize;
        for p in &json_projects {
            let exists: bool = conn.query_row(
                "SELECT COUNT(*) FROM projects WHERE id = ?1",
                params![p.id],
                |row| row.get::<_,i32>(0),
            ).map(|n| n > 0).unwrap_or(false);
            if !exists {
                conn.execute("INSERT INTO projects (id, user_id, name, created_at) VALUES (?1, 'local', ?2, ?3)",
                    params![p.id, p.name, p.created_at]).ok();
                for c in &p.commands {
                    conn.execute("INSERT INTO project_commands (id, project_id, name, command, working_dir) VALUES (?1, ?2, ?3, ?4, ?5)",
                        params![c.id, p.id, c.name, c.command, c.working_dir]).ok();
                }
                count += 1;
            }
        }
        if count > 0 {
            println!("✅ 从 JSON 合并了 {} 个项目到 SQLite", count);
        }
        Ok(())
    }

    pub fn get_projects(&self) -> Result<Vec<Project>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT id, name, created_at FROM projects WHERE user_id = 'local' ORDER BY created_at DESC").map_err(|e| e.to_string())?;
        let projects: Vec<Project> = stmt.query_map([], |row| Ok((row.get::<_,String>(0)?, row.get::<_,String>(1)?, row.get::<_,String>(2)?)))
            .map_err(|e| e.to_string())?.filter_map(|r| r.ok())
            .map(|(id, name, created_at)| {
                let cmds = Self::load_commands_inner(&*conn, &id);
                Project { id, name, commands: cmds, created_at }
            }).collect();
        Ok(projects)
    }

    fn load_commands_inner(conn: &Connection, project_id: &str) -> Vec<Command> {
        let mut stmt = match conn.prepare("SELECT id, name, command, working_dir FROM project_commands WHERE project_id = ?1") { Ok(s) => s, Err(_) => return Vec::new() };
        if let Ok(rows) = stmt.query_map(params![project_id], |row| Ok(Command { id: row.get(0)?, name: row.get(1)?, command: row.get(2)?, working_dir: row.get::<_,String>(3)?.to_string() })) {
            return rows.filter_map(|r| r.ok()).collect();
        }
        Vec::new()
    }

    pub fn save_projects(&self, projects: &[Project]) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM projects WHERE user_id = 'local'", []).map_err(|e| e.to_string())?;
        for p in projects {
            conn.execute("INSERT INTO projects (id, user_id, name, created_at) VALUES (?1, 'local', ?2, ?3)", params![p.id, p.name, p.created_at]).map_err(|e| e.to_string())?;
            for c in &p.commands {
                conn.execute("INSERT INTO project_commands (id, project_id, name, command, working_dir) VALUES (?1, ?2, ?3, ?4, ?5)", params![c.id, p.id, c.name, c.command, c.working_dir]).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    pub fn load_shortcuts(&self) -> Vec<Shortcut> {
        let conn = match self.conn.lock() { Ok(c) => c, Err(_) => return Vec::new() };
        let mut builtin = builtin_shortcuts();
        if let Ok(mut stmt) = conn.prepare("SELECT id, name, command, category, description, is_favorite, use_count, is_builtin FROM shortcuts WHERE user_id = 'local' ORDER BY category, name") {
            if let Ok(rows) = stmt.query_map([], |row| Ok(Shortcut { id: row.get(0)?, name: row.get(1)?, command: row.get(2)?, category: row.get(3)?, description: row.get(4)?, favorite: row.get::<_,i32>(5)? != 0, use_count: row.get(6)?, is_builtin: row.get::<_,i32>(7)? != 0 })) {
                for r in rows.flatten() { builtin.push(r); }
            }
        }
        builtin
    }

    pub fn save_shortcuts(&self, shortcuts: &[Shortcut]) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM shortcuts WHERE user_id = 'local' AND is_builtin = 0", []).map_err(|e| e.to_string())?;
        for s in shortcuts {
            conn.execute("INSERT OR REPLACE INTO shortcuts (id, user_id, name, command, category, description, is_favorite, use_count, is_builtin) VALUES (?1, 'local', ?2, ?3, ?4, ?5, ?6, ?7, 0)", params![s.id, s.name, s.command, s.category, s.description, s.favorite as i32, s.use_count]).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn get_note_groups(&self) -> Result<Vec<NoteGroup>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT id, name, parent_id, sort_order, created_at, updated_at FROM note_groups WHERE user_id = 'local' ORDER BY sort_order, name").map_err(|e| e.to_string())?;
        let groups = stmt.query_map([], |row| Ok(NoteGroup { id: row.get(0)?, name: row.get(1)?, parent_id: row.get(2)?, sort_order: row.get(3)?, created_at: row.get(4)?, updated_at: row.get(5)? })).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();
        Ok(groups)
    }

    pub fn save_note_group(&self, group: &NoteGroup) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("INSERT OR REPLACE INTO note_groups (id, user_id, name, parent_id, sort_order, created_at, updated_at) VALUES (?1, 'local', ?2, ?3, ?4, ?5, ?6)", params![group.id, group.name, group.parent_id, group.sort_order, group.created_at, group.updated_at]).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_note_group(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let count: i32 = conn.query_row("SELECT COUNT(*) FROM notes WHERE group_id = ?1 AND is_deleted = 0", params![id], |row| row.get(0)).map_err(|e| e.to_string())?;
        if count > 0 { return Err(format!("group has {} notes", count)); }
        conn.execute("UPDATE notes SET group_id = NULL WHERE group_id = ?1", params![id]).map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM note_groups WHERE id = ?1", params![id]).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_notes(&self, group_id: Option<&str>, include_deleted: bool) -> Result<Vec<Note>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let sql = match (group_id, include_deleted) {
            (Some(_), true) => "SELECT id, group_id, title, content, format, is_pinned, tags, visibility, sort_order, version, is_deleted, is_archived, created_at, updated_at FROM notes WHERE user_id = 'local' AND group_id = ?1 ORDER BY is_pinned DESC, updated_at DESC",
            (Some(_), false) => "SELECT id, group_id, title, content, format, is_pinned, tags, visibility, sort_order, version, is_deleted, is_archived, created_at, updated_at FROM notes WHERE user_id = 'local' AND group_id = ?1 AND is_deleted = 0 ORDER BY is_pinned DESC, updated_at DESC",
            (None, true) => "SELECT id, group_id, title, content, format, is_pinned, tags, visibility, sort_order, version, is_deleted, is_archived, created_at, updated_at FROM notes WHERE user_id = 'local' ORDER BY is_pinned DESC, updated_at DESC",
            (None, false) => "SELECT id, group_id, title, content, format, is_pinned, tags, visibility, sort_order, version, is_deleted, is_archived, created_at, updated_at FROM notes WHERE user_id = 'local' AND is_deleted = 0 ORDER BY is_pinned DESC, updated_at DESC",
        };
        let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
        let notes = if let Some(gid) = group_id {
            stmt.query_map(params![gid], map_note).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect()
        } else {
            stmt.query_map([], map_note).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect()
        };
        Ok(notes)
    }

    pub fn get_note_by_id(&self, id: &str) -> Result<Option<Note>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT id, group_id, title, content, format, is_pinned, tags, visibility, sort_order, version, is_deleted, is_archived, created_at, updated_at FROM notes WHERE id = ?1 AND user_id = 'local'").map_err(|e| e.to_string())?;
        let mut rows = stmt.query_map(params![id], map_note).map_err(|e| e.to_string())?;
        Ok(rows.next().and_then(|r| r.ok()))
    }

    pub fn save_note(&self, note: &Note) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        // 没指定分组 → 自动归入"未分组"
        let group_id = note.group_id.clone().or_else(|| {
            conn.query_row(
                "SELECT id FROM note_groups WHERE user_id = 'local' AND name = '未分组' LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            ).ok()
        });

        let tags_json = serde_json::to_string(&note.tags).unwrap_or_else(|_| "[]".into());
        conn.execute("INSERT OR REPLACE INTO notes (id, user_id, group_id, title, content, format, is_pinned, tags, visibility, sort_order, version, is_deleted, is_archived, created_at, updated_at) VALUES (?1, 'local', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![note.id, group_id, note.title, note.content, note.format, note.is_pinned as i32, tags_json, note.visibility, note.sort_order, note.version, note.is_deleted as i32, note.is_archived as i32, note.created_at, note.updated_at],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 确保"未分组"固定分组存在（启动时调用）
    fn ensure_uncategorized_group(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let exists: bool = conn.query_row(
            "SELECT COUNT(*) FROM note_groups WHERE user_id = 'local' AND name = '未分组'",
            [],
            |row| row.get::<_, i32>(0),
        ).map(|n| n > 0).unwrap_or(false);
        if !exists {
            let now = chrono::Utc::now().to_rfc3339();
            let id = "fixed_uncategorized";
            conn.execute(
                "INSERT INTO note_groups (id, user_id, name, parent_id, sort_order, created_at, updated_at) VALUES (?1, 'local', '未分组', NULL, 0, ?2, ?2)",
                params![id, now],
            ).map_err(|e| e.to_string())?;
            println!("✅ 已创建固定分组: 未分组");
        }

        // 将历史遗留的 group_id=NULL 的笔记归入"未分组"（首次运行一次性修复）
        let orphan_count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM notes WHERE user_id = 'local' AND group_id IS NULL AND is_deleted = 0",
            [],
            |row| row.get(0),
        ).unwrap_or(0);
        if orphan_count > 0 {
            let id: String = conn.query_row(
                "SELECT id FROM note_groups WHERE user_id = 'local' AND name = '未分组' LIMIT 1",
                [],
                |row| row.get(0),
            ).map_err(|e| e.to_string())?;
            conn.execute(
                "UPDATE notes SET group_id = ?1 WHERE user_id = 'local' AND group_id IS NULL",
                params![id],
            ).map_err(|e| e.to_string())?;
            println!("✅ 已将 {} 条历史笔记归入「未分组」", orphan_count);
        }
        Ok(())
    }

    /// 将笔记移动到指定分组
    pub fn move_note_to_group(&self, note_id: &str, group_id: Option<&str>) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE notes SET group_id = ?1, updated_at = ?2 WHERE id = ?3 AND user_id = 'local'",
            params![group_id, chrono::Utc::now().to_rfc3339(), note_id],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_note(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("UPDATE notes SET is_deleted = 1, updated_at = datetime('now') WHERE id = ?1", params![id]).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn permanently_delete_note(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM notes WHERE id = ?1", params![id]).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn search_notes(&self, query: &str) -> Result<Vec<Note>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let pattern = format!("%{}%", query.replace('%', "\\%").replace('_', "\\_"));
        let mut stmt = conn.prepare("SELECT id, group_id, title, content, format, is_pinned, tags, visibility, sort_order, version, is_deleted, is_archived, created_at, updated_at FROM notes WHERE user_id = 'local' AND is_deleted = 0 AND (title LIKE ?1 ESCAPE '\\' OR content LIKE ?1 ESCAPE '\\') ORDER BY updated_at DESC LIMIT 50").map_err(|e| e.to_string())?;
        let notes = stmt.query_map(params![pattern], map_note).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();
        Ok(notes)
    }

    pub fn get_note_count(&self) -> Result<i32, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let count: i32 = conn.query_row("SELECT COUNT(*) FROM notes WHERE user_id = 'local' AND is_deleted = 0", [], |row| row.get(0)).map_err(|e| e.to_string())?;
        Ok(count)
    }

    // ── Checkpoint 持久化 ──

    #[allow(dead_code)]
    pub fn save_checkpoint(&self, session_id: &str, worker_id: &str, iteration: u32, thought: &str, action: &str, observation: &str, timestamp: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let id = format!("cp_{}_{}", worker_id, iteration);
        conn.execute(
            "INSERT OR REPLACE INTO react_checkpoints (id, session_id, worker_id, iteration, thought, action, observation, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![id, session_id, worker_id, iteration, thought, action, observation, timestamp],
        ).map_err(|e| format!("保存 checkpoint 失败: {}", e))?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn load_checkpoints(&self, session_id: &str) -> Result<Vec<(u32, String, String, String, String)>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(
            "SELECT iteration, thought, action, observation, timestamp FROM react_checkpoints WHERE session_id = ?1 ORDER BY iteration ASC"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(rusqlite::params![session_id], |row| {
            Ok((row.get::<_,u32>(0)?, row.get::<_,String>(1)?, row.get::<_,String>(2)?, row.get::<_,String>(3)?, row.get::<_,String>(4)?))
        }).map_err(|e| e.to_string())?;
        let mut results = Vec::new();
        for row in rows.flatten() {
            results.push(row);
        }
        Ok(results)
    }

    #[allow(dead_code)]
    pub fn clear_session_checkpoints(&self, session_id: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM react_checkpoints WHERE session_id = ?1", rusqlite::params![session_id])
            .map_err(|e| format!("清理 checkpoint 失败: {}", e))?;
        Ok(())
    }

    // ── MCP 服务器配置持久化 ──

    pub fn save_mcp_server(&self, id: &str, name: &str, transport: &str, url: Option<&str>, command: Option<&str>, args: Option<&str>) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR REPLACE INTO mcp_servers (id, name, transport, url, command, args, enabled, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, COALESCE((SELECT created_at FROM mcp_servers WHERE id=?1), ?7), ?7)",
            rusqlite::params![id, name, transport, url, command, args, now],
        ).map_err(|e| format!("保存 MCP 服务器失败: {}", e))?;
        Ok(())
    }

    pub fn delete_mcp_server(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM mcp_servers WHERE id = ?1", rusqlite::params![id])
            .map_err(|e| format!("删除 MCP 服务器失败: {}", e))?;
        Ok(())
    }

    pub fn list_mcp_servers(&self) -> Result<Vec<(String, String, String, Option<String>, Option<String>, Option<String>, bool)>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(
            "SELECT id, name, transport, url, command, args, enabled FROM mcp_servers ORDER BY name"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_,String>(0)?,
                row.get::<_,String>(1)?,
                row.get::<_,String>(2)?,
                row.get::<_,Option<String>>(3)?,
                row.get::<_,Option<String>>(4)?,
                row.get::<_,Option<String>>(5)?,
                row.get::<_,i32>(6)? != 0,
            ))
        }).map_err(|e| e.to_string())?;
        let mut results = Vec::new();
        for row in rows.flatten() { results.push(row); }
        Ok(results)
    }

    pub fn get_enabled_mcp_servers(&self) -> Result<Vec<(String, String, String, Option<String>, Option<String>, Option<String>)>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(
            "SELECT id, name, transport, url, command, args FROM mcp_servers WHERE enabled = 1 ORDER BY name"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_,String>(0)?,
                row.get::<_,String>(1)?,
                row.get::<_,String>(2)?,
                row.get::<_,Option<String>>(3)?,
                row.get::<_,Option<String>>(4)?,
                row.get::<_,Option<String>>(5)?,
            ))
        }).map_err(|e| e.to_string())?;
        let mut results = Vec::new();
        for row in rows.flatten() { results.push(row); }
        Ok(results)
    }
}

fn map_note(row: &rusqlite::Row) -> rusqlite::Result<Note> {
    let tags_str: String = row.get(6)?;
    let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
    Ok(Note {
        id: row.get(0)?, group_id: row.get(1)?, title: row.get(2)?, content: row.get(3)?,
        format: row.get(4)?, is_pinned: row.get::<_,i32>(5)? != 0, tags,
        visibility: row.get(7)?, sort_order: row.get(8)?, version: row.get(9)?,
        is_deleted: row.get::<_,i32>(10)? != 0, is_archived: row.get::<_,i32>(11)? != 0,
        created_at: row.get(12)?, updated_at: row.get(13)?,
    })
}

/// 从运行时 JSON 文件 (~/.jc9/data/quick/default-shortcuts.json) 加载内置快捷命令。
/// 如果文件不存在，则从编译时嵌入的 JSON 自动创建。
fn builtin_shortcuts() -> Vec<Shortcut> {
    let quick_dir = dirs_data().join("quick");
    let json_path = quick_dir.join("default-shortcuts.json");

    // 首次运行时，从嵌入的 JSON 写入 runtime 路径
    if !json_path.exists() {
        if let Some(parent) = json_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let embedded = include_str!("default-shortcuts.json");
        let _ = fs::write(&json_path, embedded);
        println!("✅ 已创建默认快捷命令文件: {:?}", json_path);
    }

    // 从 runtime JSON 读取
    if let Ok(content) = fs::read_to_string(&json_path) {
        if let Ok(list) = serde_json::from_str::<Vec<ShortcutValue>>(&content) {
            return list.into_iter().map(|s| Shortcut {
                id: s.id, name: s.name, command: s.command, category: s.category,
                description: s.description, favorite: false, use_count: 0, is_builtin: true,
            }).collect();
        }
        println!("⚠️ 解析快捷命令 JSON 失败，使用嵌入的默认值");
    }

    // fallback: 直接解析嵌入的 JSON
    let embedded: &str = include_str!("default-shortcuts.json");
    serde_json::from_str::<Vec<ShortcutValue>>(embedded).unwrap_or_default()
        .into_iter().map(|s| Shortcut {
            id: s.id, name: s.name, command: s.command, category: s.category,
            description: s.description, favorite: false, use_count: 0, is_builtin: true,
        }).collect()
}

#[derive(Deserialize)]
struct ShortcutValue {
    id: String, name: String, command: String, category: String, description: String,
}

pub fn get_db_path() -> Result<PathBuf, String> {
    Ok(dirs_data().join("jc9.db"))
}

fn dirs_data() -> PathBuf {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| {
            // fallback：用 exe 所在目录
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .to_string_lossy().to_string()
        });
    PathBuf::from(home).join(".jc9").join("data")
}
