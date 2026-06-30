use std::sync::{Arc, Mutex};
use rusqlite::{params, Connection};
use chrono::{DateTime, Utc};
use super::types::*;
use super::vector_store::VectorStore;

/// 知识库 - SQLite 持久化 + TF-IDF + 向量语义检索 (sqlite-vec)
pub struct KnowledgeBase {
    conn: Arc<Mutex<Connection>>,
    vector_store: VectorStore,
    auto_promote_threshold: f64,
}

impl KnowledgeBase {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        let vector_store = VectorStore::new(conn.clone());

        Self {
            conn,
            vector_store,
            auto_promote_threshold: 0.7,
        }
    }

    /// 向持久化库插入或覆写条目（含可信度自动计分）
    pub async fn add_entry(&self, mut entry: KbEntry) -> String {
        if entry.id.is_empty() {
            entry.id = uuid::Uuid::new_v4().to_string();
        }

        // 自动计算初始置信度
        if entry.confidence == 0.0 || (entry.is_draft && entry.confidence < 0.5) {
            entry.confidence = self.compute_initial_confidence(&entry);
        }

        // 搜索相似条目并互相增强置信度
        let similar = self.find_similar_entries(&entry.content, 3).await;
        if !similar.is_empty() {
            let boost = 0.05 * similar.len() as f64;
            entry.confidence = (entry.confidence + boost).min(1.0);
            // 被多次确认的相似条目也加分
            for sim in &similar {
                let new_conf = (sim.confidence + 0.03).min(1.0);
                let _ = self.update_confidence(&sim.id, new_conf).await;
            }
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

        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return entry.id,
        };
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
        drop(conn);

        // 异步生成并存储向量嵌入（不阻塞主流程）
        let entry_id = entry.id.clone();
        let entry_content = entry.content.clone();
        let vector_store = self.vector_store.clone_conn();
        tokio::spawn(async move {
            match vector_store.generate_embedding(&entry_content).await {
                Ok(embedding) => {
                    // 使用 source_id 作为 embedding 的主键，确保 UPSERT 正确替换而非追加
                    let vec_entry = super::vector_store::VectorEntry {
                        id: format!("emb_{}", entry_id),
                        source_id: entry_id,
                        content: entry_content,
                        embedding,
                    };
                    if let Err(e) = vector_store.upsert(&vec_entry).await {
                        println!("⚠️  向量存储失败: {}", e);
                    }
                }
                Err(e) => println!("⚠️  向量生成失败: {}", e),
            }
        });

        entry.id
    }

    /// 根据条目类型和来源计算初始置信度
    fn compute_initial_confidence(&self, entry: &KbEntry) -> f64 {
        let base = match entry.entry_type {
            KbEntryType::Takeaway => {
                if entry.tags.iter().any(|t| t == "auto-generated") {
                    0.45
                } else {
                    0.55
                }
            }
            KbEntryType::PitfallNote => 0.5,
            KbEntryType::Pattern => 0.55,
            KbEntryType::Solution => 0.5,
            KbEntryType::ApiReference => 0.6,
            KbEntryType::ConfigNote => 0.65,
        };

        if entry.title.len() > 10 && entry.title.len() < 120 {
            base + 0.05
        } else {
            base
        }
    }

    /// 搜索内容相似的知识条目
    async fn find_similar_entries(&self, content: &str, limit: usize) -> Vec<KbEntry> {
        let all = self.get_entries_internal(true).await;
        let query_lower = content.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace()
            .filter(|w| w.len() > 2)
            .collect();

        let mut scored: Vec<(f64, KbEntry)> = all.into_iter()
            .map(|e| {
                let content_lower = e.content.to_lowercase();
                let mut score = 0.0;
                for term in &query_terms {
                    if content_lower.contains(term) {
                        score += 1.0;
                    }
                }
                let norm_score = if query_terms.is_empty() { 0.0 } else { score / query_terms.len() as f64 };
                (norm_score, e)
            })
            .filter(|(s, _)| *s > 0.3)
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().take(limit).map(|(_, e)| e).collect()
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

    /// 向量语义搜索（sqlite-vec 或纯 Rust 余弦相似度）
    pub async fn semantic_search(&self, query: &str, limit: usize) -> Vec<(String, f32, String)> {
        self.vector_store.semantic_search(query, limit).await
    }

    /// 检查是否使用 sqlite-vec 加速
    pub fn using_sqlite_vec(&self) -> bool {
        self.vector_store.using_sqlite_vec()
    }

    pub async fn search_by_type(&self, entry_type: KbEntryType, limit: usize) -> Vec<KbEntry> {
        let all_entries = self.get_entries_internal(false).await;
        all_entries.into_iter()
            .filter(|e| e.entry_type == entry_type)
            .take(limit)
            .collect()
    }

    pub async fn promote(&self, entry_id: &str) -> bool {
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return false,
        };
        let rows = conn.execute(
            "UPDATE knowledge SET is_draft = 0, updated_at = ?2 WHERE id = ?1",
            params![entry_id, Utc::now().to_rfc3339()],
        );
        rows.is_ok() && rows.unwrap() > 0
    }

    /// 删除知识条目及其向量嵌入（用于笔记永久删除时同步清理）
    pub async fn remove_entry(&self, entry_id: &str) -> bool {
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return false,
        };
        let _ = conn.execute("DELETE FROM embeddings WHERE source_id = ?1", params![entry_id]);
        let _ = conn.execute("DELETE FROM vec_embeddings WHERE id IN (SELECT id FROM embeddings WHERE source_id = ?1)", params![entry_id]);
        conn.execute("DELETE FROM knowledge WHERE id = ?1", params![entry_id]).is_ok()
    }

    pub async fn update_confidence(&self, entry_id: &str, confidence: f64) -> bool {
        let is_draft = if confidence >= self.auto_promote_threshold { 0 } else { 1 };
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return false,
        };
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
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return vec![],
        };
        let mut stmt = match conn.prepare(
            "SELECT id, title, content, tags, entry_type, confidence, is_draft, created_at, updated_at FROM knowledge WHERE is_draft = 1"
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        self.query_to_vector(&mut stmt).unwrap_or_default()
    }

    async fn get_entries_internal(&self, include_drafts: bool) -> Vec<KbEntry> {
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return vec![],
        };
        let sql = if include_drafts {
            "SELECT id, title, content, tags, entry_type, confidence, is_draft, created_at, updated_at FROM knowledge"
        } else {
            "SELECT id, title, content, tags, entry_type, confidence, is_draft, created_at, updated_at FROM knowledge WHERE is_draft = 0"
        };
        let mut stmt = match conn.prepare(sql) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        self.query_to_vector(&mut stmt).unwrap_or_default()
    }

    // ── Session 会话持久化 ──

    /// 保存/更新一个 AI 会话
    pub async fn save_session(&self, session: &AiSession) -> bool {
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return false,
        };
        let status_str = match session.status {
            SessionStatus::Active => "active",
            SessionStatus::Paused => "paused",
            SessionStatus::Completed => "completed",
            SessionStatus::Failed => "failed",
            SessionStatus::Cancelled => "cancelled",
        };
        let result = conn.execute(
            r#"INSERT OR REPLACE INTO ai_sessions 
            (id, title, status, project_id, task_description, token_count, cost_usd, created_at, updated_at) 
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#,
            params![
                session.id,
                session.title,
                status_str,
                session.project_id,
                session.task_description,
                session.token_count as i64,
                session.cost_usd,
                session.created_at.to_rfc3339(),
                session.updated_at.to_rfc3339(),
            ],
        );
        result.is_ok()
    }

    /// 加载所有 AI 会话
    pub async fn load_sessions(&self) -> Vec<AiSession> {
        self.load_sessions_blocking()
    }

    /// 同步版本：加载所有 AI 会话（内部使用，无需 async 运行时）
    pub fn load_sessions_blocking(&self) -> Vec<AiSession> {
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return vec![],
        };
        let mut stmt = match conn.prepare(
            "SELECT id, title, status, project_id, task_description, token_count, cost_usd, created_at, updated_at FROM ai_sessions ORDER BY updated_at DESC"
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        let rows = stmt.query_map([], |row| {
            let status_str: String = row.get(2)?;
            let status = match status_str.as_str() {
                "active" => SessionStatus::Active,
                "paused" => SessionStatus::Paused,
                "completed" => SessionStatus::Completed,
                "failed" => SessionStatus::Failed,
                _ => SessionStatus::Cancelled,
            };
            let created_at_str: String = row.get(7)?;
            let updated_at_str: String = row.get(8)?;
            Ok(AiSession {
                id: row.get(0)?,
                title: row.get(1)?,
                status,
                project_id: row.get(3)?,
                task_description: row.get(4)?,
                token_count: row.get::<_, i64>(5)? as u64,
                cost_usd: row.get(6)?,
                created_at: DateTime::parse_from_rfc3339(&created_at_str).map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
                updated_at: DateTime::parse_from_rfc3339(&updated_at_str).map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
            })
        });

        match rows {
            Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
            Err(_) => vec![],
        }
    }

    /// 删除一个会话
    pub async fn delete_session(&self, session_id: &str) -> bool {
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return false,
        };
        conn.execute("DELETE FROM ai_sessions WHERE id = ?1", params![session_id])
            .is_ok()
    }

    fn query_to_vector(&self, stmt: &mut rusqlite::Statement) -> Result<Vec<KbEntry>, String> {
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
        }).map_err(|e| e.to_string())?;

        let mut all_entries = Vec::new();
        for entry in entries_iter {
            if let Ok(e) = entry {
                all_entries.push(e);
            }
        }
        Ok(all_entries)
    }
}