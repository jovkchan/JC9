use std::path::PathBuf;
use std::sync::Mutex;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

/// 向量存储 - 集成 sqlite-vec 扩展实现语义检索
/// 如果无法加载 sqlite-vec 扩展，回退到纯 Rust 余弦相似度

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorEntry {
    pub id: String,
    pub source_id: String,   // 关联的知识条目 ID
    pub content: String,      // 原始文本内容
    pub embedding: Vec<f32>,  // 向量 (维度根据模型而定)
}

pub struct VectorStore {
    conn: std::sync::Arc<Mutex<Connection>>,
    has_sqlite_vec: bool,
    embedding_dim: usize,
}

impl VectorStore {
    /// 创建向量存储，尝试加载 sqlite-vec 扩展
    pub fn new(conn: std::sync::Arc<Mutex<Connection>>) -> Self {
        let mut has_sqlite_vec = false;
        let conn_guard = conn.lock().unwrap();

        // 尝试加载 sqlite-vec 扩展
        let vec_dll_path = Self::find_vec_extension();
        if let Some(dll_path) = &vec_dll_path {
            // 在 Windows 上，sqlite-vec 扩展名为 vec0.dll
            unsafe {
                let result = conn_guard.load_extension_enable();
                if result.is_ok() {
                    let load_result = conn_guard.load_extension(dll_path, None);
                    if load_result.is_ok() {
                        has_sqlite_vec = true;
                        println!("✅ sqlite-vec 扩展加载成功");
                    } else {
                        println!("⚠️  sqlite-vec 加载失败: {:?}，回退到纯 Rust 余弦相似度", load_result.err());
                        let _ = conn_guard.load_extension_disable();
                    }
                }
            }
        } else {
            println!("⚠️  未找到 sqlite-vec DLL，回退到纯 Rust 余弦相似度");
        }

        // 如果 sqlite-vec 加载成功，创建虚拟表
        if has_sqlite_vec {
            let _ = conn_guard.execute(
                "CREATE VIRTUAL TABLE IF NOT EXISTS vec_embeddings USING vec0(embedding float[1536])",
                [],
            );
        }

        drop(conn_guard);

        Self {
            conn,
            has_sqlite_vec,
            embedding_dim: 1536, // OpenAI text-embedding-3-small 默认维度
        }
    }

    /// 克隆连接引用（用于异步任务中独立操作）
    pub fn clone_conn(&self) -> Self {
        Self {
            conn: self.conn.clone(),
            has_sqlite_vec: self.has_sqlite_vec,
            embedding_dim: self.embedding_dim,
        }
    }

    /// 查找 sqlite-vec 扩展 DLL
    fn find_vec_extension() -> Option<PathBuf> {
        // 搜索 vec0.dll 的候选路径
        let cwd = std::env::current_dir().ok();
        let exe_dir = std::env::current_exe().ok()
            .and_then(|p| p.parent().map(|pp| pp.to_path_buf()));

        let mut candidates: Vec<PathBuf> = Vec::new();

        // 可执行文件同目录
        if let Some(ref dir) = exe_dir {
            candidates.push(dir.join("vec0.dll"));
        }
        // 当前工作目录
        if let Some(ref dir) = cwd {
            candidates.push(dir.join("vec0.dll"));
        }
        // src-tauri 子目录（从 cwd 或 exe_dir 向上找）
        for base in [&cwd, &exe_dir].iter().filter_map(|p| p.as_ref()) {
            candidates.push(base.join("src-tauri").join("vec0.dll"));
            // 尝试父级的 src-tauri 目录
            if let Some(parent) = base.parent() {
                candidates.push(parent.join("src-tauri").join("vec0.dll"));
            }
        }

        for candidate in &candidates {
            if candidate.exists() {
                return Some(candidate.clone());
            }
        }
        None
    }

    /// 生成文本的嵌入向量 (调用 OpenAI Embeddings API)
    pub async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, String> {
        // 尝试使用 OPENAI_API_KEY
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            let base_url = std::env::var("OPENAI_BASE_URL")
                .unwrap_or_else(|_| "https://api.openai.com/v1".into());

            let client = reqwest::Client::new();
            let body = serde_json::json!({
                "model": "text-embedding-3-small",
                "input": text,
            });

            let resp = client
                .post(format!("{}/embeddings", base_url))
                .header("Authorization", format!("Bearer {}", key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("Embedding API 请求失败: {}", e))?;

            let json: serde_json::Value = resp.json().await
                .map_err(|e| format!("解析 embedding 响应失败: {}", e))?;

            if let Some(data) = json["data"].as_array() {
                if let Some(first) = data.first() {
                    if let Some(embedding) = first["embedding"].as_array() {
                        return Ok(embedding.iter()
                            .filter_map(|v| v.as_f64().map(|f| f as f32))
                            .collect());
                    }
                }
            }
            Err("Embedding API 返回格式异常".into())
        } else {
            // 无 API Key 时，生成简单的哈希向量作为降级方案
            Ok(Self::hash_embedding(text))
        }
    }

    /// 将文本哈希为模拟向量（7维，用于无 API 时的降级）
    fn hash_embedding(text: &str) -> Vec<f32> {
        use sha2::{Sha256, Digest};
        let hash = Sha256::digest(text.as_bytes());
        // 将 32 字节哈希扩展为 1536 维向量
        let mut vec = Vec::with_capacity(1536);
        for i in 0..1536 {
            let byte_val = hash[i % 32] as f32 / 255.0;
            vec.push(byte_val);
        }
        vec
    }

    /// 存储向量条目
    pub async fn upsert(&self, entry: &VectorEntry) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let blob: Vec<u8> = entry.embedding.iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();

        let _ = conn.execute(
            "INSERT OR REPLACE INTO embeddings (id, source_id, content, embedding) VALUES (?1, ?2, ?3, ?4)",
            params![entry.id, entry.source_id, entry.content, blob],
        );

        // 如果 sqlite-vec 可用，同时写入虚拟表
        if self.has_sqlite_vec {
            let vec_blob: Vec<u8> = entry.embedding.iter()
                .flat_map(|f| f.to_le_bytes())
                .collect();
            let _ = conn.execute(
                "INSERT OR REPLACE INTO vec_embeddings (id, embedding) VALUES (?1, ?2)",
                params![entry.id, vec_blob],
            );
        }

        Ok(())
    }

    /// 向量相似度搜索 (KNN)
    pub async fn search(
        &self,
        query_embedding: &[f32],
        limit: usize,
        threshold: f32,
    ) -> Vec<(String, f32, String)> {
        // 使用 sqlite-vec 虚拟表 (如果可用)
        if self.has_sqlite_vec && query_embedding.len() == self.embedding_dim {
            if let Ok(results) = self.search_with_vec_ext(query_embedding, limit) {
                return results;
            }
        }

        // 回退：纯 Rust 余弦相似度
        self.search_cosine(query_embedding, limit, threshold)
    }

    /// 使用 sqlite-vec 虚拟表进行 KNN 搜索
    fn search_with_vec_ext(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(String, f32, String)>, String> {
        let conn = self.conn.lock().unwrap();
        let blob: Vec<u8> = query_embedding.iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();

        let sql = format!(
            "SELECT v.id, v.distance, e.content FROM vec_embeddings v 
             JOIN embeddings e ON v.id = e.id 
             WHERE v.embedding MATCH ?1 AND k = ?2 
             ORDER BY v.distance"
        );

        let mut stmt = conn.prepare(&sql)
            .map_err(|e| format!("vec_embeddings 查询失败: {}", e))?;

        let rows = stmt.query_map(params![blob, limit as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, f64>(1)? as f32,
                row.get::<_, String>(2)?,
            ))
        }).map_err(|e| e.to_string())?;

        let mut results = Vec::new();
        for row in rows {
            if let Ok(r) = row {
                results.push(r);
            }
        }
        Ok(results)
    }

    /// 纯 Rust 余弦相似度搜索
    fn search_cosine(
        &self,
        query_embedding: &[f32],
        limit: usize,
        threshold: f32,
    ) -> Vec<(String, f32, String)> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, source_id, content, embedding FROM embeddings"
        ).unwrap();

        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Vec<u8>>(3)?,
            ))
        }).unwrap();

        let mut scored: Vec<(String, f32, String)> = Vec::new();
        for row in rows {
            if let Ok((id, _source_id, content, blob)) = row {
                let stored_embedding: Vec<f32> = blob
                    .chunks_exact(4)
                    .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    .collect();

                if stored_embedding.len() == query_embedding.len() {
                    let similarity = cosine_similarity(query_embedding, &stored_embedding);
                    if similarity >= threshold {
                        scored.push((id, similarity, content));
                    }
                }
            }
        }

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        scored
    }

    /// 文本语义搜索（自动生成 embedding 并查询）
    pub async fn semantic_search(
        &self,
        query: &str,
        limit: usize,
    ) -> Vec<(String, f32, String)> {
        match self.generate_embedding(query).await {
            Ok(embedding) => self.search(&embedding, limit, 0.3).await,
            Err(e) => {
                println!("Embedding 生成失败: {}，回退到关键词搜索", e);
                vec![]
            }
        }
    }

    /// 检查是否使用 sqlite-vec
    pub fn using_sqlite_vec(&self) -> bool {
        self.has_sqlite_vec
    }
}

/// 计算两个向量的余弦相似度
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let mut dot = 0.0_f32;
    let mut norm_a = 0.0_f32;
    let mut norm_b = 0.0_f32;

    for i in 0..a.len() {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a.sqrt() * norm_b.sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &c) - 0.0).abs() < 0.001);

        let d = vec![1.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &d);
        assert!(sim > 0.7 && sim < 0.71);
    }
}
