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
        if need_init { db.migrate_from_json()?; }
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
        let tags_json = serde_json::to_string(&note.tags).unwrap_or_else(|_| "[]".into());
        conn.execute("INSERT OR REPLACE INTO notes (id, user_id, group_id, title, content, format, is_pinned, tags, visibility, sort_order, version, is_deleted, is_archived, created_at, updated_at) VALUES (?1, 'local', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![note.id, note.group_id, note.title, note.content, note.format, note.is_pinned as i32, tags_json, note.visibility, note.sort_order, note.version, note.is_deleted as i32, note.is_archived as i32, note.created_at, note.updated_at],
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

fn builtin_shortcuts() -> Vec<Shortcut> {
    fn s(id: &str, name: &str, command: &str, category: &str, description: &str) -> Shortcut {
        Shortcut { id: id.into(), name: name.into(), command: command.into(), category: category.into(), description: description.into(), favorite: false, use_count: 0, is_builtin: true }
    }
    vec![
        s("go-bug","go bug","go bug","Go","start go bug report"),
        s("go-build","go build","go build -o bin/app.exe .","Go","compile packages"),
        s("go-clean","go clean","go clean","Go","remove object files"),
        s("go-doc","go doc","go doc","Go","show documentation"),
        s("go-fix","go fix","go fix ./...","Go","update to new APIs"),
        s("go-fmt","go fmt","go fmt ./...","Go","gofmt"),
        s("go-generate","go generate","go generate ./...","Go","generate Go files"),
        s("go-get","go get","go get","Go","add dependencies"),
        s("go-install","go install","go install","Go","compile and install"),
        s("go-list","go list","go list ./...","Go","list packages"),
        s("go-mod-tidy","go mod tidy","go mod tidy","Go","tidy modules"),
        s("go-mod-verify","go mod verify","go mod verify","Go","verify modules"),
        s("go-mod-vendor","go mod vendor","go mod vendor","Go","vendor modules"),
        s("go-work","go work","go work","Go","workspace maintenance"),
        s("go-run","go run","go run .","Go","compile and run"),
        s("go-telemetry","go telemetry","go telemetry","Go","manage telemetry"),
        s("go-test","go test","go test ./...","Go","test packages"),
        s("go-test-cover","go test -cover","go test -cover ./...","Go","test with coverage"),
        s("go-tool","go tool","go tool","Go","run go tool"),
        s("go-version","go version","go version","Go","print go version"),
        s("go-vet","go vet","go vet ./...","Go","report likely mistakes"),
        s("npm-install","npm install","npm install","Node","install dependencies"),
        s("npm-run-dev","npm run dev","npm run dev","Node","start dev server"),
        s("npm-run-build","npm run build","npm run build","Node","production build"),
        s("npm-audit","npm audit fix","npm audit fix","Node","audit and fix"),
        s("npm-init","npm init -y","npm init -y","Node","init package.json"),
        s("yarn-install","yarn install","yarn install","Node","yarn install"),
        s("yarn-dev","yarn dev","yarn dev","Node","yarn dev server"),
        s("npx-tsc","npx tsc --noEmit","npx tsc --noEmit","Node","TypeScript check"),
        s("git-status","git status","git status","Git","show working tree status"),
        s("git-pull","git pull","git pull","Git","fetch and merge"),
        s("git-push","git push","git push","Git","push commits"),
        s("git-commit","git commit -m \"\"","git commit -m \"\"","Git","commit changes"),
        s("git-add-all","git add","git add .","Git","stage all changes"),
        s("git-log","git log --oneline -10","git log --oneline -10","Git","last 10 commits"),
        s("git-branch","git branch -a","git branch -a","Git","list branches"),
        s("git-checkout","git checkout","git checkout ","Git","switch branch"),
        s("git-stash","git stash","git stash","Git","stash changes"),
        s("git-diff","git diff","git diff","Git","show unstaged changes"),
        s("git-clone","git clone","git clone ","Git","clone repo"),
        s("git-merge","git merge","git merge ","Git","merge branch"),
        s("git-rebase","git rebase","git rebase ","Git","rebase"),
    ]
}

pub fn get_db_path() -> Result<PathBuf, String> {
    Ok(dirs_data().join("jc9.db"))
}

fn dirs_data() -> PathBuf {
    #[cfg(target_os = "windows")]
    { std::env::var("APPDATA").ok().map(PathBuf::from).map(|p| p.join("jc9")).unwrap_or_else(|| PathBuf::from(".")) }
    #[cfg(not(target_os = "windows"))]
    { std::env::var("HOME").ok().map(PathBuf::from).map(|p| p.join(".local").join("share").join("jc9")).unwrap_or_else(|| PathBuf::from(".")) }
}
