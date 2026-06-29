use std::sync::Mutex;
use std::path::PathBuf;
use rusqlite::{params, Connection};
use chrono::{DateTime, Utc};
use super::types::*;

/// 知识库 - SQLite 持久化与纯 Rust 相似度（TF-IDF）混合检索实现
pub struct KnowledgeBase {
    conn: Mutex<Connection>,
    auto_promote_threshold: f64,
}

impl KnowledgeBase {
    pub fn new(db_path: PathBuf) -> Self {
        let conn = Connection::open(db_path).expect("无法打开知识库本地 SQLite 数据库");
        conn.execute(
            r#"CREATE TABLE IF NOT EXISTS knowledge (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                tags TEXT NOT NULL,
                entry_type TEXT NOT NULL,
                confidence REAL NOT NULL,
                is_draft INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )"#,
            [],
        ).expect("无法初始化知识库本地表结构");

        Self {
            conn: Mutex::new(conn),
            auto_promote_threshold: 0.7,
        }
    }

    /// 向持久化库插入或覆写条目
    pub async fn add_entry(&self, mut entry: KbEntry) -> String {
        if entry.id.is_empty() {
            entry.id = uuid::Uuid::new_v4().to_string();
        }
        if entry.confidence >= self.auto_promote_threshold {
            entry.is_draft = false;
        }

        let tags_str = entry.tags.join(",");
        let is_draft_int = if entry.is_draft { 1 } else { 0 };
        let entry_type_str = match entry.entry_type {
            KbEntryType::ConfigNote => "config_note",
            KbEntryType::Solution => "solution",
            KbEntryType::PitfallNote => "pitfall_note",
            KbEntryType::Pattern => "pattern",
            KbEntryType::ApiReference => "api_reference",
            KbEntryType::Takeaway => "takeaway",
        };

        let conn = self.conn.lock().unwrap();
        let _ = conn.execute(
            r#"INSERT OR REPLACE INTO knowledge 
            (id, title, content, tags, entry_type, confidence, is_draft, created_at, updated_at) 
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#,
            params![
                entry.id,
                entry.title,
                entry.content,
                tags_str,
                entry_type_str,
                entry.confidence,
                is_draft_int,
                entry.created_at.to_rfc3339(),
                entry.updated_at.to_rfc3339()
            ],
        );
        entry.id
    }

    /// 纯 Rust TF-IDF 相关字频匹配检索算法
    pub async fn search(&self, query: &str, limit: usize) -> Vec<KbEntry> {
        let all_entries = self.get_entries_internal(false).await;
        
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        let mut scored: Vec<(f64, KbEntry)> = all_entries.into_iter()
            .map(|e| {
                let title_lower = e.title.to_lowercase();
                let content_lower = e.content.to_lowercase();
                let tags_lower: Vec<String> = e.tags.iter().map(|t| t.to_lowercase()).collect();
                
                let mut score = 0.0;
                for term in &query_terms {
                    if title_lower.contains(term) {
                        score += 3.0;
                    }
                    if content_lower.contains(term) {
                        score += 1.0;
                    }
                    for tag in &tags_lower {
                        if tag.contains(term) {
                            score += 2.0;
                        }
                    }
                }
                score *= e.confidence;
                (score, e)
            })
            .filter(|(s, _)| *s > 0.0)
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().take(limit).map(|(_, e)| e).collect()
    }

    pub async fn search_by_type(&self, entry_type: KbEntryType, limit: usize) -> Vec<KbEntry> {
        let all_entries = self.get_entries_internal(false).await;
        all_entries.into_iter()
            .filter(|e| e.entry_type == entry_type)
            .take(limit)
            .collect()
    }

    pub async fn promote(&self, entry_id: &str) -> bool {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute(
            "UPDATE knowledge SET is_draft = 0, updated_at = ?2 WHERE id = ?1",
            params![entry_id, Utc::now().to_rfc3339()],
        );
        rows.is_ok() && rows.unwrap() > 0
    }

    pub async fn update_confidence(&self, entry_id: &str, confidence: f64) -> bool {
        let is_draft = if confidence >= self.auto_promote_threshold { 0 } else { 1 };
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute(
            "UPDATE knowledge SET confidence = ?2, is_draft = ?3, updated_at = ?4 WHERE id = ?1",
            params![entry_id, confidence, is_draft, Utc::now().to_rfc3339()],
        );
        rows.is_ok() && rows.unwrap() > 0
    }

    pub async fn list_all(&self) -> Vec<KbEntry> {
        self.get_entries_internal(false).await
    }

    pub async fn list_drafts(&self) -> Vec<KbEntry> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, content, tags, entry_type, confidence, is_draft, created_at, updated_at FROM knowledge WHERE is_draft = 1"
        ).unwrap();
        self.query_to_vector(&mut stmt)
    }

    async fn get_entries_internal(&self, include_drafts: bool) -> Vec<KbEntry> {
        let conn = self.conn.lock().unwrap();
        let sql = if include_drafts {
            "SELECT id, title, content, tags, entry_type, confidence, is_draft, created_at, updated_at FROM knowledge"
        } else {
            "SELECT id, title, content, tags, entry_type, confidence, is_draft, created_at, updated_at FROM knowledge WHERE is_draft = 0"
        };
        let mut stmt = conn.prepare(sql).unwrap();
        self.query_to_vector(&mut stmt)
    }

    fn query_to_vector(&self, stmt: &mut rusqlite::Statement) -> Vec<KbEntry> {
        let entries_iter = stmt.query_map([], |row| {
            let tags_str: String = row.get(3)?;
            let tags = if tags_str.is_empty() { vec![] } else { tags_str.split(',').map(|s| s.to_string()).collect() };
            
            let entry_type_str: String = row.get(4)?;
            let entry_type = match entry_type_str.as_str() {
                "config_note" => KbEntryType::ConfigNote,
                "solution" => KbEntryType::Solution,
                "pitfall_note" => KbEntryType::PitfallNote,
                "pattern" => KbEntryType::Pattern,
                "api_reference" => KbEntryType::ApiReference,
                _ => KbEntryType::Takeaway,
            };

            let created_at_str: String = row.get(7)?;
            let updated_at_str: String = row.get(8)?;

            Ok(KbEntry {
                id: row.get(0)?,
                title: row.get(1)?,
                content: row.get(2)?,
                entry_type,
                tags,
                source_session: None,
                confidence: row.get(5)?,
                is_draft: row.get::<_, i32>(6)? != 0,
                created_at: DateTime::parse_from_rfc3339(&created_at_str).map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
                updated_at: DateTime::parse_from_rfc3339(&updated_at_str).map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
                embedding: None,
            })
        }).unwrap();

        let mut all_entries = Vec::new();
        for entry in entries_iter {
            if let Ok(e) = entry {
                all_entries.push(e);
            }
        }
        all_entries
    }
}