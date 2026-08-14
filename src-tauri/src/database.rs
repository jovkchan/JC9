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
pub struct Memory {
    pub id: String,
    pub scope: String,
    pub topic_key: String,
    pub title: String,
    pub content: String,
    pub memory_type: String,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryPage {
    pub items: Vec<Memory>,
    pub total: i64,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteVersion {
    pub id: String,
    pub note_id: String,
    pub title: String,
    pub content: String,
    pub format: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub version: i32,
    pub created_at: String,
}

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
            CREATE TABLE IF NOT EXISTS memories (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL DEFAULT 'local',
                scope TEXT NOT NULL DEFAULT '',
                topic_key TEXT NOT NULL DEFAULT '',
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                memory_type TEXT NOT NULL DEFAULT 'discovery',
                tags TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS mcp_api_keys (
                id TEXT PRIMARY KEY,
                key TEXT NOT NULL UNIQUE,
                label TEXT NOT NULL DEFAULT '',
                scope TEXT NOT NULL DEFAULT '',
                group_ids TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS tags (id TEXT PRIMARY KEY, user_id TEXT NOT NULL, name TEXT NOT NULL, color TEXT NOT NULL DEFAULT '', FOREIGN KEY (user_id) REFERENCES users(id));
            CREATE TABLE IF NOT EXISTS resources (id TEXT PRIMARY KEY, user_id TEXT NOT NULL, note_id TEXT, filename TEXT NOT NULL, file_path TEXT NOT NULL, size INTEGER NOT NULL DEFAULT 0, mime_type TEXT NOT NULL DEFAULT '', created_at TEXT NOT NULL, FOREIGN KEY (user_id) REFERENCES users(id), FOREIGN KEY (note_id) REFERENCES notes(id) ON DELETE SET NULL);
            CREATE TABLE IF NOT EXISTS sync_log (id TEXT PRIMARY KEY, table_name TEXT NOT NULL, record_id TEXT NOT NULL, action TEXT NOT NULL, version INTEGER NOT NULL, synced INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS note_versions (
                id TEXT PRIMARY KEY,
                note_id TEXT NOT NULL,
                title TEXT NOT NULL DEFAULT '',
                content TEXT NOT NULL DEFAULT '',
                format TEXT NOT NULL DEFAULT 'markdown',
                tags TEXT NOT NULL DEFAULT '[]',
                version INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (note_id) REFERENCES notes(id)
            );
            CREATE INDEX IF NOT EXISTS idx_note_versions_note_id ON note_versions(note_id);
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
                embedding BLOB NOT NULL,
                vec_rowid INTEGER
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
            -- FTS5 全文索引：内容同步表，自动跟随 knowledge 表更新
            CREATE VIRTUAL TABLE IF NOT EXISTS knowledge_fts USING fts5(title, content, tags, content=knowledge, content_rowid=rowid);
            -- FTS5 触发器：保持 knowledge 和 knowledge_fts 同步
            CREATE TRIGGER IF NOT EXISTS knowledge_ai AFTER INSERT ON knowledge BEGIN
                INSERT INTO knowledge_fts(rowid, title, content, tags) VALUES (new.rowid, new.title, new.content, new.tags);
            END;
            CREATE TRIGGER IF NOT EXISTS knowledge_ad AFTER DELETE ON knowledge BEGIN
                INSERT INTO knowledge_fts(knowledge_fts, rowid, title, content, tags) VALUES('delete', old.rowid, old.title, old.content, old.tags);
            END;
            CREATE TRIGGER IF NOT EXISTS knowledge_au AFTER UPDATE ON knowledge BEGIN
                INSERT INTO knowledge_fts(knowledge_fts, rowid, title, content, tags) VALUES('delete', old.rowid, old.title, old.content, old.tags);
                INSERT INTO knowledge_fts(rowid, title, content, tags) VALUES (new.rowid, new.title, new.content, new.tags);
            END;
        ").map_err(|e| format!("create tables: {e}"))?;
        let _ = conn.execute("ALTER TABLE notes ADD COLUMN is_archived INTEGER NOT NULL DEFAULT 0", []);
        let _ = conn.execute("ALTER TABLE embeddings ADD COLUMN vec_rowid INTEGER", []);
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

    /// 读取设置项
    #[allow(dead_code)]
    pub fn get_setting(&self, key: &str) -> Result<Option<String>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let result = conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        );
        match result {
            Ok(val) => Ok(Some(val)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("get_setting: {e}")),
        }
    }

    /// 写入设置项
    #[allow(dead_code)]
    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        ).map_err(|e| format!("set_setting: {e}"))?;
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

        // 同步写入 tags 表（去重：同名标签只保留一条）
        for tag in &note.tags {
            let tag_id = format!("tag_{}", tag.to_lowercase());
            let _ = conn.execute(
                "INSERT OR IGNORE INTO tags (id, user_id, name, color) VALUES (?1, 'local', ?2, '')",
                params![tag_id, tag],
            );
        }
        Ok(())
    }

    // ══════════════════════════════════════════════════════════════
    // 版本历史管理
    // ══════════════════════════════════════════════════════════════

    /// 保存笔记版本快照
    pub fn save_note_version(&self, note: &Note) -> Result<Option<String>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        // 计算下一个版本号
        let next_version: i32 = conn.query_row(
            "SELECT COALESCE(MAX(version), 0) + 1 FROM note_versions WHERE note_id = ?1",
            params![note.id],
            |row| row.get(0),
        ).unwrap_or(1);

        let version_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let tags_json = serde_json::to_string(&note.tags).unwrap_or_else(|_| "[]".into());

        conn.execute(
            "INSERT INTO note_versions (id, note_id, title, content, format, tags, version, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![version_id, note.id, note.title, note.content, note.format, tags_json, next_version, now],
        ).map_err(|e| format!("保存版本快照失败: {}", e))?;

        // 裁剪超出上限（默认 50 条）的旧版本
        const MAX_VERSIONS: i32 = 50;
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM note_versions WHERE note_id = ?1",
            params![note.id],
            |row| row.get(0),
        ).unwrap_or(0);
        if count > MAX_VERSIONS {
            conn.execute(
                "DELETE FROM note_versions WHERE note_id = ?1 AND id NOT IN (SELECT id FROM note_versions WHERE note_id = ?1 ORDER BY version DESC LIMIT ?2)",
                params![note.id, MAX_VERSIONS],
            ).ok();
        }

        Ok(Some(version_id))
    }

    /// 获取笔记的所有版本（按版本号倒序）
    pub fn get_note_versions(&self, note_id: &str, limit: usize) -> Result<Vec<NoteVersion>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(
            "SELECT id, note_id, title, content, format, tags, version, created_at FROM note_versions WHERE note_id = ?1 ORDER BY version DESC LIMIT ?2"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![note_id, limit as i64], |row| {
            let tags_str: String = row.get(5)?;
            let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
            Ok(NoteVersion {
                id: row.get(0)?,
                note_id: row.get(1)?,
                title: row.get(2)?,
                content: row.get(3)?,
                format: row.get(4)?,
                tags,
                version: row.get(6)?,
                created_at: row.get(7)?,
            })
        }).map_err(|e| e.to_string())?;
        let versions: Vec<NoteVersion> = rows.filter_map(|r| r.ok()).collect();
        Ok(versions)
    }

    /// 获取单个版本详情
    pub fn get_note_version_by_id(&self, version_id: &str) -> Result<Option<NoteVersion>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(
            "SELECT id, note_id, title, content, format, tags, version, created_at FROM note_versions WHERE id = ?1"
        ).map_err(|e| e.to_string())?;
        let mut rows = stmt.query_map(params![version_id], |row| {
            let tags_str: String = row.get(5)?;
            let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
            Ok(NoteVersion {
                id: row.get(0)?,
                note_id: row.get(1)?,
                title: row.get(2)?,
                content: row.get(3)?,
                format: row.get(4)?,
                tags,
                version: row.get(6)?,
                created_at: row.get(7)?,
            })
        }).map_err(|e| e.to_string())?;
        Ok(rows.next().and_then(|r| r.ok()))
    }

    /// 恢复笔记到指定版本：将版本快照内容写回 notes 表
    pub fn restore_note_version(&self, note_id: &str, version_id: &str) -> Result<Note, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        // 查找版本快照
        let version = conn.query_row(
            "SELECT id, note_id, title, content, format, tags, version, created_at FROM note_versions WHERE id = ?1 AND note_id = ?2",
            params![version_id, note_id],
            |row| {
                let tags_str: String = row.get(5)?;
                let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
                Ok(NoteVersion {
                    id: row.get(0)?,
                    note_id: row.get(1)?,
                    title: row.get(2)?,
                    content: row.get(3)?,
                    format: row.get(4)?,
                    tags,
                    version: row.get(6)?,
                    created_at: row.get(7)?,
                })
            }
        ).map_err(|_| format!("版本不存在: {}", version_id))?;

        // 恢复前，为当前内容创建一个自动保存版本（保险）
        let current: Option<Note> = conn.query_row(
            "SELECT id, group_id, title, content, format, is_pinned, tags, visibility, sort_order, version, is_deleted, is_archived, created_at, updated_at FROM notes WHERE id = ?1 AND user_id = 'local'",
            params![note_id],
            map_note,
        ).ok();
        if let Some(ref cur) = current {
            let tags_json = serde_json::to_string(&cur.tags).unwrap_or_else(|_| "[]".into());
            let next_ver: i32 = conn.query_row(
                "SELECT COALESCE(MAX(version), 0) + 1 FROM note_versions WHERE note_id = ?1",
                params![note_id],
                |row| row.get(0),
            ).unwrap_or(1);
            let _ = conn.execute(
                "INSERT INTO note_versions (id, note_id, title, content, format, tags, version, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![uuid::Uuid::new_v4().to_string(), note_id, cur.title, cur.content, cur.format, tags_json, next_ver, chrono::Utc::now().to_rfc3339()],
            );
        }

        // 用版本快照内容覆盖笔记
        let now = chrono::Utc::now().to_rfc3339();
        let tags_json = serde_json::to_string(&version.tags).unwrap_or_else(|_| "[]".into());
        conn.execute(
            "UPDATE notes SET title=?1, content=?2, format=?3, tags=?4, updated_at=?5 WHERE id=?6 AND user_id='local'",
            params![version.title, version.content, version.format, tags_json, now, note_id],
        ).map_err(|e| format!("恢复版本失败: {}", e))?;

        // 读取恢复后的笔记返回
        let restored = conn.query_row(
            "SELECT id, group_id, title, content, format, is_pinned, tags, visibility, sort_order, version, is_deleted, is_archived, created_at, updated_at FROM notes WHERE id = ?1 AND user_id = 'local'",
            params![note_id],
            map_note,
        ).map_err(|e| format!("恢复后读取失败: {}", e))?;

        Ok(restored)
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

    // ── Memories ──

    pub fn add_memory(&self, memory: &Memory) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let tags_json = serde_json::to_string(&memory.tags).unwrap_or_else(|_| "[]".into());
        conn.execute(
            "INSERT OR REPLACE INTO memories (id, user_id, scope, topic_key, title, content, memory_type, tags, created_at, updated_at) VALUES (?1, 'local', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![memory.id, memory.scope, memory.topic_key, memory.title, memory.content, memory.memory_type, tags_json, memory.created_at, memory.updated_at],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn update_memory(&self, id: &str, title: Option<&str>, content: Option<&str>, memory_type: Option<&str>, topic_key: Option<&str>) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let now = chrono::Utc::now().to_rfc3339();
        if let Some(t) = title { conn.execute("UPDATE memories SET title=?1, updated_at=?2 WHERE id=?3", params![t, now, id]).map_err(|e| e.to_string())?; }
        if let Some(c) = content { conn.execute("UPDATE memories SET content=?1, updated_at=?2 WHERE id=?3", params![c, now, id]).map_err(|e| e.to_string())?; }
        if let Some(mt) = memory_type { conn.execute("UPDATE memories SET memory_type=?1, updated_at=?2 WHERE id=?3", params![mt, now, id]).map_err(|e| e.to_string())?; }
        if let Some(tk) = topic_key { conn.execute("UPDATE memories SET topic_key=?1, updated_at=?2 WHERE id=?3", params![tk, now, id]).map_err(|e| e.to_string())?; }
        Ok(())
    }

    pub fn delete_memory(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM memories WHERE id=?1", params![id]).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_memories(&self, search: &str, page: i64, page_size: i64, scope_filter: &str) -> Result<MemoryPage, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let search_pattern = format!("%{}%", search);
        let has_scope = !scope_filter.is_empty();

        let total: i64 = match (search.is_empty(), has_scope) {
            (true, false) => conn.query_row("SELECT COUNT(*) FROM memories WHERE user_id='local'", [], |r| r.get(0)).map_err(|e| e.to_string())?,
            (true, true) => conn.query_row("SELECT COUNT(*) FROM memories WHERE user_id='local' AND scope=?1", params![scope_filter], |r| r.get(0)).map_err(|e| e.to_string())?,
            (false, false) => conn.query_row("SELECT COUNT(*) FROM memories WHERE user_id='local' AND (title LIKE ?1 OR content LIKE ?1)", params![search_pattern], |r| r.get(0)).map_err(|e| e.to_string())?,
            (false, true) => conn.query_row("SELECT COUNT(*) FROM memories WHERE user_id='local' AND scope=?1 AND (title LIKE ?2 OR content LIKE ?2)", params![scope_filter, search_pattern], |r| r.get(0)).map_err(|e| e.to_string())?,
        };

        let offset = (page - 1).max(0) * page_size;

        let items: Vec<Memory> = if search.is_empty() && !has_scope {
            let mut stmt = conn.prepare(
                "SELECT id, scope, topic_key, title, content, memory_type, tags, created_at, updated_at FROM memories WHERE user_id='local' ORDER BY updated_at DESC LIMIT ?1 OFFSET ?2"
            ).map_err(|e| e.to_string())?;
            let result: Vec<Memory> = stmt.query_map(params![page_size, offset], Self::map_memory_row)
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();
            result
        } else if !search.is_empty() && !has_scope {
            let mut stmt = conn.prepare(
                "SELECT id, scope, topic_key, title, content, memory_type, tags, created_at, updated_at FROM memories WHERE user_id='local' AND (title LIKE ?1 OR content LIKE ?1) ORDER BY updated_at DESC LIMIT ?2 OFFSET ?3"
            ).map_err(|e| e.to_string())?;
            let result: Vec<Memory> = stmt.query_map(params![search_pattern, page_size, offset], Self::map_memory_row)
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();
            result
        } else if search.is_empty() && has_scope {
            let mut stmt = conn.prepare(
                "SELECT id, scope, topic_key, title, content, memory_type, tags, created_at, updated_at FROM memories WHERE user_id='local' AND scope=?1 ORDER BY updated_at DESC LIMIT ?2 OFFSET ?3"
            ).map_err(|e| e.to_string())?;
            let result: Vec<Memory> = stmt.query_map(params![scope_filter, page_size, offset], Self::map_memory_row)
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();
            result
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, scope, topic_key, title, content, memory_type, tags, created_at, updated_at FROM memories WHERE user_id='local' AND scope=?1 AND (title LIKE ?2 OR content LIKE ?2) ORDER BY updated_at DESC LIMIT ?3 OFFSET ?4"
            ).map_err(|e| e.to_string())?;
            let result: Vec<Memory> = stmt.query_map(params![scope_filter, search_pattern, page_size, offset], Self::map_memory_row)
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();
            result
        };

        Ok(MemoryPage { items, total })
    }

    fn map_memory_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Memory> {
        Ok(Memory {
            id: row.get(0)?, scope: row.get(1)?, topic_key: row.get(2)?, title: row.get(3)?, content: row.get(4)?,
            memory_type: row.get(5)?,
            tags: serde_json::from_str(&row.get::<_,String>(6).unwrap_or_default()).unwrap_or_default(),
            created_at: row.get(7)?, updated_at: row.get(8)?,
        })
    }

    #[allow(dead_code)]
    pub fn get_memory_by_topic(&self, topic_key: &str) -> Result<Option<Memory>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT id, topic_key, title, content, memory_type, tags, created_at, updated_at FROM memories WHERE user_id='local' AND topic_key=?1 LIMIT 1").map_err(|e| e.to_string())?;
        let mut rows = stmt.query_map(params![topic_key], |row| {
            let tags_str: String = row.get::<_,String>(5).unwrap_or_default();
            Ok(Memory {
                id: row.get(0)?, scope: String::new(), topic_key: row.get(1)?, title: row.get(2)?, content: row.get(3)?,
                memory_type: row.get(4)?, tags: serde_json::from_str(&tags_str).unwrap_or_default(),
                created_at: row.get(6)?, updated_at: row.get(7)?,
            })
        }).map_err(|e| e.to_string())?;
        Ok(rows.next().and_then(|r| r.ok()))
    }

    pub fn compress_memories(&self, ids: &[String], new_id: &str) -> Result<String, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let placeholders: Vec<String> = ids.iter().enumerate().map(|(i, _)| format!("?{}", i+1)).collect();
        let sql = format!("SELECT title, content FROM memories WHERE id IN ({}) ORDER BY updated_at DESC", placeholders.join(","));
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let params: Vec<&dyn rusqlite::types::ToSql> = ids.iter().map(|id| id as &dyn rusqlite::types::ToSql).collect();
        let rows = stmt.query_map(params.as_slice(), |row| {
            Ok((row.get::<_,String>(0)?, row.get::<_,String>(1)?))
        }).map_err(|e| e.to_string())?;
        let mut parts = vec![];
        for r in rows { if let Ok((t, c)) = r { parts.push(format!("- **{}**: {}", t, c.chars().take(200).collect::<String>())); } }
        let compressed = format!("# 记忆压缩\n\n> {} 条记忆合并\n\n{}\n\n---\n压缩时间: {}", ids.len(), parts.join("\n"), chrono::Utc::now().to_rfc3339());
        // 删除原记忆
        for id in ids { conn.execute("DELETE FROM memories WHERE id=?1", params![id]).map_err(|e| e.to_string())?; }
        // 插入压缩结果
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute("INSERT INTO memories (id, user_id, topic_key, title, content, memory_type, tags, created_at, updated_at) VALUES (?1,'local','compressed','记忆压缩',?2,'summary','[\"compressed\"]',?3,?3)", params![new_id, compressed, now]).map_err(|e| e.to_string())?;
        Ok(new_id.to_string())
    }

    pub fn get_note_count(&self) -> Result<i32, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let count: i32 = conn.query_row("SELECT COUNT(*) FROM notes WHERE user_id = 'local' AND is_deleted = 0", [], |row| row.get(0)).map_err(|e| e.to_string())?;
        Ok(count)
    }

    /// 数据库诊断统计（knowledge / embeddings / vec_embeddings / fts 行数）
    #[allow(dead_code)]
    pub fn get_database_stats(&self) -> Result<serde_json::Value, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let count_table = |table: &str| -> i64 {
            conn.query_row(&format!("SELECT COUNT(*) FROM {}", table), [], |r| r.get(0)).unwrap_or(-1)
        };
        let note_count: i64 = conn.query_row("SELECT COUNT(*) FROM notes WHERE is_deleted=0", [], |r| r.get(0)).unwrap_or(0);
        Ok(serde_json::json!({
            "knowledge": count_table("knowledge"),
            "embeddings": count_table("embeddings"),
            "vec_embeddings": count_table("vec_embeddings"),
            "knowledge_fts": count_table("knowledge_fts"),
            "notes": note_count
        }))
    }

    /// 重建 FTS5 索引（knowledge_fts 内容同步表的初始化/修复）
    #[allow(dead_code)]
    pub fn rebuild_knowledge_fts(&self) -> Result<usize, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let affected: usize = conn.execute(
            "INSERT INTO knowledge_fts(knowledge_fts) VALUES('rebuild')", []
        ).map_err(|e| format!("FTS5 rebuild failed: {}", e))?;
        Ok(affected)
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

/// 读取 AI 配置（模型、端点等），存为 JSON 文件
pub fn get_ai_config() -> Result<String, String> {
    let path = dirs_data().join("ai-config.json");
    if path.exists() {
        fs::read_to_string(&path).map_err(|e| format!("读取 ai-config 失败: {e}"))
    } else {
        Ok("{}".to_string())
    }
}

/// 保存 AI 配置到 JSON 文件
pub fn save_ai_config(config: &str) -> Result<(), String> {
    let path = dirs_data().join("ai-config.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, config).map_err(|e| format!("保存 ai-config 失败: {e}"))
}

/// 读取动画效果配置（流光/光晕），存为 JSON 文件
pub fn get_effect_config() -> Result<String, String> {
    let path = dirs_data().join("effect-config.json");
    if path.exists() {
        fs::read_to_string(&path).map_err(|e| format!("读取 effect-config 失败: {e}"))
    } else {
        Ok("{}".to_string())
    }
}

/// 保存动画效果配置到 JSON 文件
pub fn save_effect_config(config: &str) -> Result<(), String> {
    let path = dirs_data().join("effect-config.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, config).map_err(|e| format!("保存 effect-config 失败: {e}"))
}

// ── 自动化（积木编辑器，F1b）──

/// 读取自动化列表（~/.jc9/data/automations.json）
pub fn get_automations() -> Result<String, String> {
    let path = dirs_data().join("automations.json");
    if path.exists() {
        fs::read_to_string(&path).map_err(|e| format!("读取 automations 失败: {e}"))
    } else {
        let default = "[]";
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&path, default);
        Ok(default.to_string())
    }
}

/// 保存自动化列表（整表 JSON）
pub fn save_automations_json(automations_json: &str) -> Result<(), String> {
    let path = dirs_data().join("automations.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, automations_json).map_err(|e| format!("保存 automations 失败: {e}"))
}

/// 删除单个自动化（读 → 按 id 过滤 → 写回）
pub fn delete_automation(id: &str) -> Result<(), String> {
    let path = dirs_data().join("automations.json");
    let content = if path.exists() {
        fs::read_to_string(&path).unwrap_or_else(|_| "[]".to_string())
    } else {
        "[]".to_string()
    };
    let arr: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("解析 automations 失败: {e}"))?;
    let filtered: Vec<serde_json::Value> = arr
        .as_array()
        .map(|xs| {
            xs.iter()
                .filter(|v| v.get("id").and_then(|i| i.as_str()) != Some(id))
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    let out = serde_json::to_string(&filtered).map_err(|e| e.to_string())?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, out).map_err(|e| format!("删除 automations 失败: {e}"))
}

// ── 凭据（登录，F1b 前端壳明文 JSON；F3 字段级 AES-GCM 加密）──

/// 读取凭据列表（~/.jc9/data/credentials.json，仅掩码）
pub fn get_credentials() -> Result<String, String> {
    let path = dirs_data().join("credentials.json");
    if path.exists() {
        fs::read_to_string(&path).map_err(|e| format!("读取 credentials 失败: {e}"))
    } else {
        let default = "[]";
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&path, default);
        Ok(default.to_string())
    }
}

/// 保存凭据列表（整表 JSON）
pub fn save_credentials_json(credentials_json: &str) -> Result<(), String> {
    let path = dirs_data().join("credentials.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, credentials_json).map_err(|e| format!("保存 credentials 失败: {e}"))
}

/// 按 id 新增/更新一条凭据（敏感字段 AES-GCM 加密后落盘；F3）
pub fn upsert_credential(credential_json: &str) -> Result<(), String> {
    let key = crate::credential_crypto::load_or_create_key()?;
    let mut record: serde_json::Value = serde_json::from_str(credential_json)
        .map_err(|e| format!("解析凭据失败: {e}"))?;
    // 敏感字段加密（password/token/kubeconfig），明文不落盘
    if let Some(fields) = record.get_mut("fields").and_then(|f| f.as_object_mut()) {
        for k in crate::credential_crypto::sensitive_fields() {
            if let Some(v) = fields.get(*k) {
                if let Some(s) = v.as_str() {
                    if !s.is_empty() && !crate::credential_crypto::is_encrypted(s) {
                        let enc = crate::credential_crypto::encrypt_field(&key, s)?;
                        fields.insert(k.to_string(), serde_json::Value::String(enc));
                    }
                }
            }
        }
    }
    let path = dirs_data().join("credentials.json");
    let content = if path.exists() {
        fs::read_to_string(&path).unwrap_or_else(|_| "[]".to_string())
    } else {
        "[]".to_string()
    };
    let mut arr: Vec<serde_json::Value> =
        serde_json::from_str(&content).map_err(|e| format!("解析 credentials 失败: {e}"))?;
    let id = record.get("id").and_then(|i| i.as_str()).unwrap_or("");
    arr.retain(|c| c.get("id").and_then(|i| i.as_str()) != Some(id));
    arr.push(record);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let out = serde_json::to_string(&arr).map_err(|e| e.to_string())?;
    fs::write(&path, out).map_err(|e| format!("保存 credentials 失败: {e}"))
}

/// 删除单个凭据（读 → 按 id 过滤 → 写回）
pub fn delete_credential(id: &str) -> Result<(), String> {
    let path = dirs_data().join("credentials.json");
    let content = if path.exists() {
        fs::read_to_string(&path).unwrap_or_else(|_| "[]".to_string())
    } else {
        "[]".to_string()
    };
    let arr: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("解析 credentials 失败: {e}"))?;
    let filtered: Vec<serde_json::Value> = arr
        .as_array()
        .map(|xs| {
            xs.iter()
                .filter(|v| v.get("id").and_then(|i| i.as_str()) != Some(id))
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    let out = serde_json::to_string(&filtered).map_err(|e| e.to_string())?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, out).map_err(|e| format!("删除 credentials 失败: {e}"))
}

