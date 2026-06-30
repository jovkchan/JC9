use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::fs;
use rusqlite::{Connection, params};
use chrono::Utc;

/// 技能范围
#[derive(Debug, Clone, PartialEq)]
pub enum SkillScope {
    Dev,
    Runtime,
    Both,
}

impl SkillScope {
    fn from_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "runtime" => SkillScope::Runtime,
            "both" => SkillScope::Both,
            _ => SkillScope::Dev,
        }
    }

    /// 是否需要同步到运行期知识库
    fn should_sync_to_runtime(&self) -> bool {
        matches!(self, SkillScope::Runtime | SkillScope::Both)
    }
}

/// 解析后的技能文档
#[derive(Debug, Clone)]
pub struct SkillDoc {
    pub id: String,
    pub name: String,
    pub description: String,
    pub scope: SkillScope,
    pub r#type: String,
    pub content: String,
    pub file_path: PathBuf,
}

/// 技能加载器 - 将 .jc9/ 下的技能文件同步到知识库
pub struct SkillLoader {
    skills_dir: PathBuf,
    conn: Arc<Mutex<Connection>>,
}

impl SkillLoader {
    pub fn new(skills_dir: PathBuf, conn: Arc<Mutex<Connection>>) -> Self {
        Self { skills_dir, conn }
    }

    /// 扫描并同步所有技能文件到知识库
    pub async fn sync_all(&self) -> usize {
        let skills = self.scan_all();
        if skills.is_empty() {
            println!("📋 未发现技能文件");
            return 0;
        }

        let mut count = 0usize;
        for skill in &skills {
            if !skill.scope.should_sync_to_runtime() {
                continue;
            }
            if self.upsert_skill(skill) {
                count += 1;
            }
        }
        if count > 0 {
            println!("✅ 已同步 {} 个技能到知识库", count);
        }
        count
    }

    /// 扫描 .jc9/ 下所有技能文件
    pub fn scan_all(&self) -> Vec<SkillDoc> {
        let mut skills = Vec::new();

        // 扫描 skills/ 目录
        let skills_path = self.skills_dir.join("skills");
        self.scan_skills_recursive(&skills_path, &mut skills);

        // 扫描 workflows/ 目录（作为 skill 类型）
        let workflows_path = self.skills_dir.join("workflows");
        self.scan_workflows(&workflows_path, &mut skills);

        skills
    }

    /// 递归扫描 skills/ 下的 SKILL.md
    fn scan_skills_recursive(&self, dir: &Path, skills: &mut Vec<SkillDoc>) {
        if !dir.exists() || !dir.is_dir() {
            return;
        }
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // 递归子目录
                    self.scan_skills_recursive(&path, skills);
                } else if path.file_name().map_or(false, |n| n == "SKILL.md") {
                    if let Some(skill) = Self::parse_skill_file(&path) {
                        skills.push(skill);
                    }
                }
            }
        }
    }

    /// 扫描 workflows/ 下的 .md 文件
    fn scan_workflows(&self, dir: &Path, skills: &mut Vec<SkillDoc>) {
        if !dir.exists() || !dir.is_dir() {
            return;
        }
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "md") {
                    if let Some(skill) = Self::parse_skill_file(&path) {
                        skills.push(skill);
                    }
                }
            }
        }
    }

    /// 解析单个技能文件的 YAML frontmatter + Markdown
    fn parse_skill_file(path: &Path) -> Option<SkillDoc> {
        let content = fs::read_to_string(path).ok()?;

        // 解析 YAML frontmatter (--- 包围的头部)
        let content_trimmed = content.trim_start();
        if !content_trimmed.starts_with("---") {
            // 无 frontmatter，使用文件名作为标识
            let name = path.file_stem()?.to_string_lossy().to_string();
            return Some(SkillDoc {
                id: format!("skill_{}", name.replace('-', "_")),
                name: name.clone(),
                description: String::new(),
                scope: SkillScope::Both,
                r#type: "skill".into(),
                content: content.clone(),
                file_path: path.to_path_buf(),
            });
        }

        // 去掉开头的 ---
        let after_first = &content_trimmed[3..];

        // 找结尾的 ---
        let end_pos = after_first.find("\n---")?;
        let frontmatter_str = &after_first[..end_pos];
        let body = after_first[end_pos + 4..].trim().to_string();

        // 解析 frontmatter 字段
        let mut name = String::new();
        let mut description = String::new();
        let mut scope_str = String::new();
        let mut r#type = String::new();

        for line in frontmatter_str.lines() {
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim().trim_matches('"');
                match key {
                    "name" => name = value.to_string(),
                    "description" => description = value.to_string(),
                    "scope" => scope_str = value.to_string(),
                    "type" => r#type = value.to_string(),
                    _ => {}
                }
            }
        }

        let skill_name = if name.is_empty() {
            path.file_stem()?.to_string_lossy().to_string()
        } else {
            name
        };

        Some(SkillDoc {
            id: format!("skill_{}", skill_name.replace('-', "_")),
            name: skill_name,
            description,
            scope: SkillScope::from_str(&scope_str),
            r#type: if r#type.is_empty() { "skill".into() } else { r#type },
            content: body,
            file_path: path.to_path_buf(),
        })
    }

    /// 将技能写入 knowledge 表（UPSERT）
    fn upsert_skill(&self, skill: &SkillDoc) -> bool {
        let now = Utc::now().to_rfc3339();
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return false,
        };

        // 检查是否已存在且内容未变
        let existing: Option<String> = conn
            .query_row(
                "SELECT content FROM knowledge WHERE id = ?1",
                params![skill.id],
                |row| row.get(0),
            )
            .ok();

        if let Some(ref existing_content) = existing {
            if existing_content == &skill.content {
                // 内容未变，跳过
                return true;
            }
        }

        let tags = format!("skill,{}", match skill.scope {
            SkillScope::Dev => "dev",
            SkillScope::Runtime => "runtime",
            SkillScope::Both => "both",
        });

        let result = conn.execute(
            r#"INSERT OR REPLACE INTO knowledge
            (id, title, content, tags, entry_type, confidence, is_draft, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, 'config_note', 1.0, 0, ?5, ?6)"#,
            params![
                skill.id,
                skill.name,
                skill.content,
                tags,
                now,
                now,
            ],
        );

        result.is_ok()
    }
}
