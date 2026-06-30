use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::ai::ast_parser::{AstParser, SymbolInfo};

/// Repo Map — 项目结构感知
///
/// 生成项目的结构摘要，包括目录树、关键文件、符号索引和依赖摘要，
/// 注入到 Agent System Prompt 使 Agent 对项目有全局认知。
pub struct RepoMap {
    workspace_root: Arc<RwLock<PathBuf>>,
    /// 需要排除的目录/文件模式
    exclude_patterns: HashSet<String>,
}

impl RepoMap {
    pub fn new(workspace_root: Arc<RwLock<PathBuf>>) -> Self {
        let mut exclude = HashSet::new();
        exclude.insert("node_modules".into());
        exclude.insert("target".into());
        exclude.insert(".git".into());
        exclude.insert("dist".into());
        exclude.insert(".next".into());
        exclude.insert("build".into());
        exclude.insert("__pycache__".into());
        exclude.insert(".jc9".into());
        exclude.insert("gen".into());
        exclude.insert("icons".into());
        exclude.insert("debug".into());
        Self {
            workspace_root,
            exclude_patterns: exclude,
        }
    }

    /// 生成完整的 Repo Map Markdown
    pub async fn generate(&self) -> String {
        let root = self.workspace_root.read().await.clone();
        let mut parts = Vec::new();

        // 1. 目录树
        parts.push("## 📁 项目结构\n".to_string());
        let tree = self.build_tree(&root, 0, 3);
        parts.push(format!("```\n{}\n```\n", tree));

        // 2. 关键文件
        parts.push("## 📄 关键文件\n".to_string());
        let key_files = self.find_key_files(&root);
        if key_files.is_empty() {
            parts.push("（无）\n".to_string());
        } else {
            for (path, desc) in &key_files {
                parts.push(format!("- `{}` — {}\n", path, desc));
            }
        }
        parts.push("\n".to_string());

        // 3. 符号索引
        parts.push("## 🔧 公开符号索引\n".to_string());
        let symbols = self.collect_symbols(&root).await;
        if symbols.is_empty() {
            parts.push("（无）\n".to_string());
        } else {
            // 按文件分组
            let mut by_file: Vec<(String, Vec<&SymbolInfo>)> = Vec::new();
            let mut file_order: Vec<String> = Vec::new();
            for sym in &symbols {
                let file_key = sym.parent.clone().unwrap_or_else(|| "unknown".to_string());
                if let Some(pos) = by_file.iter().position(|(f, _)| *f == file_key) {
                    by_file[pos].1.push(sym);
                } else {
                    file_order.push(file_key.clone());
                    by_file.push((file_key, vec![sym]));
                }
            }
            for file_key in &file_order {
                if let Some((_, syms)) = by_file.iter().find(|(f, _)| f == file_key) {
                    // 只显示最多 15 个符号/文件
                    parts.push(format!("  `{}`:\n", file_key));
                    for sym in syms.iter().take(15) {
                        let vis = sym.visibility.as_deref().unwrap_or("");
                        let kind = format!("{:?}", sym.kind).to_lowercase();
                        parts.push(format!("    {}{} {} (L{})\n", vis, kind, sym.name, sym.line));
                    }
                    if syms.len() > 15 {
                        parts.push(format!("    ... 还有 {} 个\n", syms.len() - 15));
                    }
                }
            }
        }
        parts.push("\n".to_string());

        // 4. 依赖摘要
        parts.push("## 📦 依赖摘要\n".to_string());
        let deps = self.collect_dependencies(&root);
        if deps.is_empty() {
            parts.push("（无法检测）\n".to_string());
        } else {
            for dep in &deps {
                parts.push(format!("- {}\n", dep));
            }
        }

        parts.concat()
    }

    /// 构建目录树文本（最多 depth 层）
    fn build_tree(&self, dir: &Path, current_depth: usize, max_depth: usize) -> String {
        if current_depth > max_depth {
            return String::new();
        }
        let mut result = String::new();
        if current_depth == 0 {
            if let Some(name) = dir.file_name() {
                result.push_str(&format!("{}/\n", name.to_string_lossy()));
            } else {
                result.push_str(&format!("{}/\n", dir.display()));
            }
        }

        if let Ok(entries) = std::fs::read_dir(dir) {
            let mut items: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    !self.exclude_patterns.contains(&name) && !name.starts_with('.')
                })
                .collect();
            items.sort_by_key(|e| {
                let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                (!is_dir, e.file_name())
            });

            for entry in items {
                let name = entry.file_name().to_string_lossy().to_string();
                let sub_indent = "  ".repeat(current_depth + 1);
                if let Ok(ft) = entry.file_type() {
                    if ft.is_dir() {
                        result.push_str(&format!("{}{}/\n", sub_indent, name));
                        if current_depth + 1 < max_depth {
                            result.push_str(&self.build_tree(&entry.path(), current_depth + 1, max_depth));
                        }
                    } else {
                        result.push_str(&format!("{}{}\n", sub_indent, name));
                    }
                }
            }
        }
        result
    }

    /// 查找关键配置文件
    fn find_key_files(&self, root: &Path) -> Vec<(String, String)> {
        let mut files = Vec::new();
        let candidates = [
            ("Cargo.toml", "Rust 项目配置与依赖"),
            ("package.json", "Node.js 项目配置与依赖"),
            ("tsconfig.json", "TypeScript 编译配置"),
            ("vite.config.ts", "Vite 构建配置（若有）"),
            ("tauri.conf.json", "Tauri 桌面应用配置"),
            ("vue.config.js", "Vue 构建配置（若有）"),
            (".env", "环境变量配置（若有）"),
            (".env.example", "环境变量示例"),
            ("Dockerfile", "容器化配置（若有）"),
            ("docker-compose.yml", "容器编排配置（若有）"),
            ("Makefile", "构建任务定义（若有）"),
            ("README.md", "项目说明文档"),
        ];
        for (name, desc) in &candidates {
            let path = root.join(name);
            if path.exists() {
                files.push((name.to_string(), desc.to_string()));
            }
        }
        files
    }

    /// 收集项目中的公开符号（利用 Tree-sitter）
    async fn collect_symbols(&self, root: &Path) -> Vec<SymbolInfo> {
        let mut symbols = Vec::new();
        let mut parser = AstParser::new();

        // 递归遍历源码目录
        let src_dirs = ["src", "src-tauri/src", "lib", "app"];
        for dir_name in &src_dirs {
            let dir = root.join(dir_name);
            if !dir.exists() { continue; }
            Self::walk_and_parse(&dir, &mut parser, &mut symbols, &self.exclude_patterns);
        }

        // 去重（同名+同文件视为重复）
        symbols.sort_by(|a, b| (a.name.clone(), a.line).cmp(&(b.name.clone(), b.line)));
        symbols.dedup_by(|a, b| a.name == b.name && a.parent == b.parent);

        symbols
    }

    /// 递归遍历目录并解析文件
    fn walk_and_parse(dir: &Path, parser: &mut AstParser, symbols: &mut Vec<SymbolInfo>, exclude: &HashSet<String>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            let mut items: Vec<_> = entries.filter_map(|e| e.ok()).collect();
            items.sort_by_key(|e| e.file_name());

            for entry in items {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') || exclude.contains(&name) { continue; }

                let path = entry.path();
                if let Ok(ft) = entry.file_type() {
                    if ft.is_dir() {
                        Self::walk_and_parse(&path, parser, symbols, exclude);
                    } else if ft.is_file() {
                        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                        if matches!(ext, "ts" | "tsx" | "js" | "jsx" | "vue" | "rs") {
                            if let Ok(file_syms) = parser.parse_file(&path) {
                                let rel_path = path.to_string_lossy().to_string();
                                for mut sym in file_syms {
                                    sym.parent = Some(rel_path.clone());
                                    symbols.push(sym);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// 收集依赖摘要
    fn collect_dependencies(&self, root: &Path) -> Vec<String> {
        let mut deps = Vec::new();

        // Cargo.toml
        let cargo_path = root.join("Cargo.toml");
        if cargo_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&cargo_path) {
                let mut in_deps = false;
                let mut count = 0;
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("[dependencies]") {
                        in_deps = true;
                        continue;
                    }
                    if in_deps {
                        if trimmed.starts_with('[') { break; }
                        if !trimmed.is_empty() && !trimmed.starts_with('#') {
                            count += 1;
                        }
                    }
                }
                deps.push(format!("Rust (Cargo.toml): {} 个依赖", count));
            }
        }

        // package.json
        let pkg_path = root.join("package.json");
        if pkg_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&pkg_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    let dep_count = json.get("dependencies")
                        .and_then(|v| v.as_object()).map(|o| o.len()).unwrap_or(0);
                    let dev_count = json.get("devDependencies")
                        .and_then(|v| v.as_object()).map(|o| o.len()).unwrap_or(0);
                    deps.push(format!("Node.js (package.json): {} 生产依赖 + {} 开发依赖", dep_count, dev_count));
                }
            }
        }

        deps
    }
}
