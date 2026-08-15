use std::path::PathBuf;
use std::sync::Mutex;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

/// 当前平台的 sqlite-vec 可加载扩展：返回 (扩展文件名, 编译期嵌入字节)。
/// 跨平台支持：Windows → vec0.dll；Linux → vec0-*.so；macOS → vec0-*.dylib。
/// 嵌入 exe 避免分发时缺少扩展文件；运行时找不到现成文件会自动释放到 exe 同级目录。
fn vec_ext() -> (&'static str, &'static [u8]) {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        ("vec0.dll", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/vec0.dll")))
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        ("vec0-linux-x86_64.so", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/vec0-linux-x86_64.so")))
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        ("vec0-linux-aarch64.so", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/vec0-linux-aarch64.so")))
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        ("vec0-macos-aarch64.dylib", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/vec0-macos-aarch64.dylib")))
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        ("vec0-macos-x86_64.dylib", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/vec0-macos-x86_64.dylib")))
    }
    // 未覆盖的平台（如 Android / iOS / 其他 arch）：编译期直接报错，提示补充扩展文件
    #[cfg(not(any(
        all(target_os = "windows", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
    )))]
    {
        compile_error!("不支持的平台：请在 src-tauri 放置对应 sqlite-vec 扩展并在此配置");
    }
}

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

        // 尝试加载 sqlite-vec 扩展（Windows: vec0.dll / Linux: vec0.so / macOS: vec0.dylib）
        let vec_ext_path = Self::find_vec_extension();
        if let Some(ext_path) = &vec_ext_path {
            unsafe {
                let result = conn_guard.load_extension_enable();
                if result.is_ok() {
                    let load_result = conn_guard.load_extension(ext_path, None);
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
            println!("⚠️  未找到 sqlite-vec 扩展，回退到纯 Rust 余弦相似度");
        }

        // 如果 sqlite-vec 加载成功，创建虚拟表
        if has_sqlite_vec {
            match conn_guard.execute(
                "CREATE VIRTUAL TABLE IF NOT EXISTS vec_embeddings USING vec0(embedding float[1536])",
                [],
            ) {
                Ok(_) => println!("   vec_embeddings 虚拟表已就绪"),
                Err(e) => println!("❌ vec_embeddings 虚拟表创建失败: {}", e),
            }
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

    /// 启动诊断：打印各表行数 + sqlite-vec 状态
    pub fn log_diagnostics(&self) {
        let conn = self.conn.lock().unwrap();
        let count = |t: &str| -> i64 {
            conn.query_row(&format!("SELECT COUNT(*) FROM {}", t), [], |r| r.get(0)).unwrap_or(-1)
        };
        println!("🧠 VectorStore 诊断 ─────────────────────");
        println!("   sqlite-vec 已加载: {}", self.has_sqlite_vec);
        println!("   embedding 维度:    {}", self.embedding_dim);
        println!("   knowledge 条目:    {}", count("knowledge"));
        println!("   embeddings 向量:   {}", count("embeddings"));
        println!("   vec_embeddings:    {}", count("vec_embeddings"));
        println!("   knowledge_fts:     {}", count("knowledge_fts"));
        println!("   ────────────────────────────────────────");
    }

    /// 重建全部知识条目的向量：读取 knowledge 表，逐条生成并 upsert
    pub async fn reindex_all(&self) -> Result<usize, String> {
        let ids: Vec<(String, String)> = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare("SELECT id, content FROM knowledge").map_err(|e| e.to_string())?;
            let rows: Vec<(String, String)> = stmt.query_map([], |row| Ok((row.get::<_,String>(0)?, row.get::<_,String>(1)?)))
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();
            rows
        };

        let total = ids.len();
        println!("🔄 开始重建 {} 条向量...", total);
        let mut count = 0usize;
        for (id, content) in &ids {
            match self.generate_embedding(content).await {
                Ok(embedding) => {
                    let entry = VectorEntry {
                        id: format!("emb_{}", id),
                        source_id: id.clone(),
                        content: content.clone(),
                        embedding,
                    };
                    if let Err(e) = self.upsert(&entry).await {
                        println!("   ❌ [{}/{}] upsert 失败: {}", count, total, e);
                    } else {
                        count += 1;
                    }
                }
                Err(e) => println!("   ❌ [{}/{}] 向量生成失败: {}", count, total, e),
            }
        }
        println!("✅ 重建完成: {}/{} 条已索引", count, total);
        Ok(count)
    }

    /// 查找 sqlite-vec 扩展文件（找不到则从嵌入的字节自动释放到 exe 同级目录）
    fn find_vec_extension() -> Option<PathBuf> {
        let (file_name, bytes) = vec_ext();
        let cwd = std::env::current_dir().ok();
        let exe_dir = std::env::current_exe().ok()
            .and_then(|p| p.parent().map(|pp| pp.to_path_buf()));

        let mut candidates: Vec<PathBuf> = Vec::new();

        // 可执行文件同目录
        if let Some(ref dir) = exe_dir {
            candidates.push(dir.join(file_name));
        }
        // 当前工作目录
        if let Some(ref dir) = cwd {
            candidates.push(dir.join(file_name));
        }
        // src-tauri 子目录（从 cwd 或 exe_dir 向上找）
        for base in [&cwd, &exe_dir].iter().filter_map(|p| p.as_ref()) {
            candidates.push(base.join("src-tauri").join(file_name));
            // 尝试父级的 src-tauri 目录
            if let Some(parent) = base.parent() {
                candidates.push(parent.join("src-tauri").join(file_name));
            }
        }

        // 先检查是否有现成的
        for candidate in &candidates {
            if candidate.exists() {
                return Some(candidate.clone());
            }
        }

        // 所有候选都不存在 → 从嵌入的字节释放到 exe 同级目录
        if let Some(ref dir) = exe_dir {
            let target = dir.join(file_name);
            // 写入之前确保父目录存在
            if let Some(parent) = target.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            match std::fs::write(&target, bytes) {
                Ok(()) => {
                    println!("📦 已自动释放 {} 到 {}", file_name, target.display());
                    return Some(target);
                }
                Err(e) => {
                    println!("⚠️  自动释放 {} 失败: {e}", file_name);
                }
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
            // 无 API Key 时：字符 3-gram 词袋向量（语义区分度远高于 SHA256 哈希）
            Ok(Self::ngram_embedding(text, self.embedding_dim))
        }
    }

    /// 将文本哈希为模拟向量（7维，用于无 API 时的降级）
    #[allow(dead_code)]
    fn hash_embedding(text: &str) -> Vec<f32> {
        use sha2::{Sha256, Digest};
        let hash = Sha256::digest(text.as_bytes());
        let mut vec = Vec::with_capacity(1536);
        for idx in 0..1536 {
            let byte_val = hash[idx % 32] as f32 / 255.0;
            vec.push(byte_val);
        }
        vec
    }

    /// 字符 n-gram 词袋向量 — 比 hash_embedding 语义区分度高得多
    /// 提取文本中所有 3-gram，哈希到 1536 维桶，加权后归一化
    fn ngram_embedding(text: &str, dim: usize) -> Vec<f32> {
        let mut vec = vec![0.0f32; dim];
        let lower = text.to_lowercase();
        let chars: Vec<char> = lower.chars().collect();
        if chars.is_empty() { return vec; }

        let mut total = 0u32;
        let hash_ngram = |s: &[char]| -> usize {
            let h = s.iter().fold(0u64, |acc, &c| acc.wrapping_mul(31).wrapping_add(c as u64));
            ((h.wrapping_mul(2654435761)) as usize) % dim
        };

        // 2-gram
        for w in chars.windows(2) {
            vec[hash_ngram(w)] += 1.0;
            total += 1;
        }
        // 3-gram (higher weight)
        for w in chars.windows(3) {
            vec[hash_ngram(w)] += 1.5;
            total += 1;
        }

        if total == 0 { return vec; }

        // BM25-style TF saturation
        let avg_tf = (total as f32 / dim as f32).max(0.001);
        let k1 = 1.2f32;
        for val in vec.iter_mut() {
            if *val > 0.0 {
                let tf = *val;
                *val = (tf * (k1 + 1.0)) / (tf + k1 * (1.0 - 0.75 + 0.75 * avg_tf));
            }
        }

        Self::normalize(&vec)
    }

    fn normalize(vec: &[f32]) -> Vec<f32> {
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm < 1e-9 { return vec.to_vec(); }
        vec.iter().map(|x| x / norm).collect()
    }

    /// 存储向量条目
    pub async fn upsert(&self, entry: &VectorEntry) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let blob: Vec<u8> = entry.embedding.iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();

        // 计算确定性整数 rowid（id → i64 hash），用于桥接 embeddings 和 vec_embeddings
        let rowid = Self::id_to_rowid(&entry.id);

        let _ = conn.execute(
            "INSERT OR REPLACE INTO embeddings (id, source_id, content, embedding, vec_rowid) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![entry.id, entry.source_id, entry.content, blob, rowid],
        );

        // 如果 sqlite-vec 可用，同时写入虚拟表（vec0 只有 rowid + embedding 两列）
        if self.has_sqlite_vec {
            let vec_blob: Vec<u8> = entry.embedding.iter()
                .flat_map(|f| f.to_le_bytes())
                .collect();
            if let Err(e) = conn.execute(
                "INSERT OR REPLACE INTO vec_embeddings(rowid, embedding) VALUES (?1, ?2)",
                params![rowid, vec_blob],
            ) {
                println!("❌ vec_embeddings 写入失败 (rowid={}): {}", rowid, e);
            }
        }

        Ok(())
    }

    /// 文本 ID → 确定性整数 rowid（乘法哈希）
    fn id_to_rowid(id: &str) -> i64 {
        let mut h: u64 = 5381;
        for b in id.bytes() {
            h = h.wrapping_mul(33).wrapping_add(b as u64);
        }
        (h & 0x7FFF_FFFF_FFFF_FFFF) as i64 // 保证非负
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

        // vec0 虚拟表只有 rowid(INTEGER) + embedding(FLOAT[])，通过 vec_rowid 桥接到 embeddings 表
        let sql = format!(
            "SELECT e.source_id, v.distance, e.content FROM vec_embeddings v 
             JOIN embeddings e ON v.rowid = e.vec_rowid 
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

    for idx in 0..a.len() {
        dot += a[idx] * b[idx];
        norm_a += a[idx] * a[idx];
        norm_b += b[idx] * b[idx];
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
