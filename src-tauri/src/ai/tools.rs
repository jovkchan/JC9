use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::path::Path;
use super::types::RiskLevel;
use super::security::SecuritySandbox;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn definition(&self) -> ToolDefinition;
    async fn execute(&self, arguments: &Value, sandbox: &SecuritySandbox) -> ToolResult;
}

// ══════════════════════════════════════════════════════════════
// 1. 读文件工具 (read_file)
// ══════════════════════════════════════════════════════════════
pub struct ReadFileTool;

#[async_trait]
impl Tool for ReadFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "read_file".into(),
            description: "读取指定文件的内容。仅允许读取工作区内或只读白名单内的文件。".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "文件路径，支持绝对路径或相对于工作区根目录的相对路径"
                    }
                },
                "required": ["path"]
            }),
            risk_level: RiskLevel::Low,
        }
    }

    async fn execute(&self, arguments: &Value, sandbox: &SecuritySandbox) -> ToolResult {
        let path_str = match arguments["path"].as_str() {
            Some(p) => p,
            None => return ToolResult { success: false, output: "".into(), error: Some("缺失 'path' 参数或参数非字符串。".into()) }
        };

        match sandbox.validate_read_path(path_str) {
            Ok(verified_path) => {
                match std::fs::read_to_string(&verified_path) {
                    Ok(content) => ToolResult { success: true, output: content, error: None },
                    Err(e) => ToolResult { success: false, output: "".into(), error: Some(format!("读取文件失败: {}", e)) }
                }
            }
            Err(err_msg) => ToolResult { success: false, output: "".into(), error: Some(err_msg) }
        }
    }
}

// ══════════════════════════════════════════════════════════════
// 2. 写文件工具 (write_file) — 带自动备份+写入验证
// ══════════════════════════════════════════════════════════════
fn create_backup(path: &Path) -> Result<(), String> {
    if !path.exists() { return Ok(()); }
    let backup_dir = path.parent().unwrap().join(".jc9_backups");
    std::fs::create_dir_all(&backup_dir).map_err(|e| format!("创建备份目录失败: {}", e))?;
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S_%3f");
    // 用安全文件名替换路径分隔符
    let safe_name = path.to_string_lossy().replace('\\', "_").replace('/', "_").replace(':', "_");
    let backup_path = backup_dir.join(format!("{}_{}", timestamp, safe_name));
    std::fs::copy(path, &backup_path).map_err(|e| format!("备份失败: {}", e))?;
    Ok(())
}

pub struct WriteFileTool;

#[async_trait]
impl Tool for WriteFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "write_file".into(),
            description: "写入内容至指定文件。仅允许在工作区内写入，不允许覆盖只读白名单路径。写入前会自动备份原文件。".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "文件路径，支持绝对路径或工作区相对路径"
                    },
                    "content": {
                        "type": "string",
                        "description": "要写入的文件完整文本内容"
                    }
                },
                "required": ["path", "content"]
            }),
            risk_level: RiskLevel::High,
        }
    }

    async fn execute(&self, arguments: &Value, sandbox: &SecuritySandbox) -> ToolResult {
        let path_str = match arguments["path"].as_str() {
            Some(p) => p,
            None => return ToolResult { success: false, output: "".into(), error: Some("缺失 'path' 参数。".into()) }
        };
        let content = match arguments["content"].as_str() {
            Some(c) => c,
            None => return ToolResult { success: false, output: "".into(), error: Some("缺失 'content' 参数。".into()) }
        };

        match sandbox.validate_write_path(path_str) {
            Ok(verified_path) => {
                // 确保父目录存在
                if let Some(parent) = verified_path.parent() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        return ToolResult { success: false, output: "".into(), error: Some(format!("创建父文件夹失败: {}", e)) };
                    }
                }
                // 自动备份原文件
                if let Err(e) = create_backup(&verified_path) {
                    return ToolResult { success: false, output: "".into(), error: Some(e) };
                }
                // 写入临时文件后 rename，防止写入中断导致文件损坏
                let temp_path = verified_path.with_extension("tmp_write");
                match std::fs::write(&temp_path, content) {
                    Ok(_) => {
                        if let Err(e) = std::fs::rename(&temp_path, &verified_path) {
                            return ToolResult { success: false, output: "".into(), error: Some(format!("文件重命名失败: {}", e)) };
                        }
                        // 写入验证：读取并比较内容
                        match std::fs::read_to_string(&verified_path) {
                            Ok(verify_content) if verify_content == content => {
                                ToolResult { success: true, output: "文件写入成功（已备份原文件并验证通过）。".into(), error: None }
                            }
                            Ok(_) => ToolResult { success: false, output: "".into(), error: Some("文件写入校验失败：内容不一致。".into()) },
                            Err(e) => ToolResult { success: false, output: "".into(), error: Some(format!("写入验证读取失败: {}", e)) },
                        }
                    }
                    Err(e) => {
                        let _ = std::fs::remove_file(&temp_path);
                        ToolResult { success: false, output: "".into(), error: Some(format!("文件写入失败: {}", e)) }
                    }
                }
            }
            Err(err_msg) => ToolResult { success: false, output: "".into(), error: Some(err_msg) }
        }
    }
}

// ══════════════════════════════════════════════════════════════
// 3. 执行命令行工具 (run_command) — 带超时+输出截断
// ══════════════════════════════════════════════════════════════
use tokio::time::timeout as tokio_timeout;
use std::time::Duration;

fn truncate_output(output: &str, max_lines: usize, max_chars: usize) -> String {
    let lines: Vec<&str> = output.lines().collect();
    let mut result = String::new();
    if lines.len() > max_lines {
        for line in &lines[..max_lines] {
            result.push_str(line);
            result.push('\n');
        }
        result.push_str(&format!("...（输出过长，已截断，共 {} 行，仅显示前 {} 行）\n", lines.len(), max_lines));
    } else {
        result.push_str(output);
    }
    if result.len() > max_chars {
        result.truncate(max_chars);
        result.push_str(&format!("\n...（输出过长，已截断至 {} 字符）", max_chars));
    }
    result
}

pub struct RunCommandTool;

#[async_trait]
impl Tool for RunCommandTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "run_command".into(),
            description: "在指定的目录下运行系统命令。仅允许执行白名单内的安全命令。默认超时 30 秒，输出超过 2000 行自动截断。".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "要运行的命令（如 'cargo check'）"
                    },
                    "working_dir": {
                        "type": "string",
                        "description": "执行命令的目录路径（可选，默认工作区根目录）"
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "超时秒数（可选，默认 30，设为 0 表示不限时）"
                    }
                },
                "required": ["command"]
            }),
            risk_level: RiskLevel::Critical,
        }
    }

    async fn execute(&self, arguments: &Value, sandbox: &SecuritySandbox) -> ToolResult {
        let command_str = match arguments["command"].as_str() {
            Some(c) => c,
            None => return ToolResult { success: false, output: "".into(), error: Some("缺失 'command' 参数。".into()) }
        };
        let working_dir_str = arguments["working_dir"].as_str().unwrap_or(".");
        let timeout_secs = arguments["timeout_secs"].as_i64().unwrap_or(30);

        // 1. 命令安全性校验
        if !sandbox.validate_command(command_str) {
            return ToolResult {
                success: false,
                output: "".into(),
                error: Some(format!(
                    "【命令越权拦截】命令 '{}' 不在白名单中或包含黑名单模式，拒绝执行。",
                    command_str
                )),
            };
        }

        // 2. 工作路径安全性校验
        let exec_path = match sandbox.validate_read_path(working_dir_str) {
            Ok(p) => p,
            Err(e) => return ToolResult { success: false, output: "".into(), error: Some(format!("工作目录校验失败: {}", e)) }
        };

        // 3. 构建命令
        let is_windows = cfg!(target_os = "windows");
        let mut shell_cmd = if is_windows {
            let mut c = std::process::Command::new("powershell");
            c.args(["-NoProfile", "-Command", command_str]);
            c
        } else {
            let mut c = std::process::Command::new("sh");
            c.args(["-c", command_str]);
            c
        };
        shell_cmd.current_dir(&exec_path);

        // 4. 异步执行（带超时）
        let result: Result<Result<std::process::Output, std::io::Error>, tokio::time::error::Elapsed> = if timeout_secs > 0 {
            let dur = Duration::from_secs(timeout_secs as u64);
            tokio_timeout(dur, async {
                tokio::task::spawn_blocking(move || shell_cmd.output()).await.unwrap_or_else(|_| Err(std::io::Error::new(std::io::ErrorKind::Other, "task panicked")))
            }).await
        } else {
            let output = tokio::task::spawn_blocking(move || shell_cmd.output()).await.unwrap_or_else(|_| Err(std::io::Error::new(std::io::ErrorKind::Other, "task panicked")));
            Ok(output)
        };

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code();
                let success = output.status.success();
                
                // 输出截断
                let mut combined = stdout;
                if !stderr.is_empty() {
                    if !combined.is_empty() { combined.push_str("\n"); }
                    combined.push_str("【错误/标准错误输出】:\n");
                    combined.push_str(&stderr);
                }
                let truncated = truncate_output(&combined, 2000, 50000);

                if success {
                    ToolResult { success: true, output: truncated, error: None }
                } else {
                    let err_msg = format!("命令以退出码 {:?} 失败。", exit_code);
                    ToolResult { success: false, output: truncated, error: Some(err_msg) }
                }
            }
            Err(_) => {
                let msg = format!("命令执行超时（{} 秒），已自动终止。", timeout_secs);
                ToolResult { success: false, output: msg.clone(), error: Some(msg) }
            }
            Ok(Err(e)) => {
                ToolResult { success: false, output: "".into(), error: Some(format!("命令执行失败: {}", e)) }
            }
        }
    }
}

// ══════════════════════════════════════════════════════════════
// 4. 内容搜索工具 (grep)
// ══════════════════════════════════════════════════════════════
pub struct GrepTool;

#[async_trait]
impl Tool for GrepTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "grep".into(),
            description: "在指定目录的文本文件中搜索包含匹配字符串的行。".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "要搜索的关键字或文本"
                    },
                    "path": {
                        "type": "string",
                        "description": "要搜索的目标目录或文件（可选，默认是工作区根目录）"
                    }
                },
                "required": ["query"]
            }),
            risk_level: RiskLevel::Low,
        }
    }

    async fn execute(&self, arguments: &Value, sandbox: &SecuritySandbox) -> ToolResult {
        let query = match arguments["query"].as_str() {
            Some(q) => q.to_lowercase(),
            None => return ToolResult { success: false, output: "".into(), error: Some("缺失 'query' 参数。".into()) }
        };
        let path_str = arguments["path"].as_str().unwrap_or(".");

        let target_path = match sandbox.validate_read_path(path_str) {
            Ok(p) => p,
            Err(e) => return ToolResult { success: false, output: "".into(), error: Some(format!("搜索路径校验失败: {}", e)) }
        };

        let mut results = Vec::new();
        if target_path.is_file() {
            if let Ok(content) = std::fs::read_to_string(&target_path) {
                search_in_text(&content, &query, &target_path, &mut results);
            }
        } else {
            // 递归目录搜索，限制最大搜索数量（避免栈溢出或大量 IO）
            let mut file_count = 0;
            if let Err(e) = visit_dirs(&target_path, &query, &mut results, &mut file_count) {
                return ToolResult { success: false, output: "".into(), error: Some(format!("遍历目录失败: {}", e)) };
            }
        }

        if results.is_empty() {
            ToolResult { success: true, output: "未找到匹配项。".into(), error: None }
        } else {
            ToolResult { success: true, output: results.join("\n"), error: None }
        }
    }
}

fn search_in_text(content: &str, query: &str, file_path: &Path, results: &mut Vec<String>) {
    for (idx, line) in content.lines().enumerate() {
        if line.to_lowercase().contains(query) {
            results.push(format!("{}:{}: {}", file_path.to_string_lossy(), idx + 1, line.trim()));
            if results.len() > 100 {
                results.push("... (匹配项过多，已截断)".into());
                break;
            }
        }
    }
}

fn visit_dirs(dir: &Path, query: &str, results: &mut Vec<String>, file_count: &mut usize) -> std::io::Result<()> {
    if *file_count > 500 || results.len() > 100 {
        return Ok(());
    }

    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                // 忽略一些常见的非代码、大体积文件夹
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                if ["node_modules", ".git", "target", "dist", "build"].contains(&name.as_str()) {
                    continue;
                }
                visit_dirs(&path, query, results, file_count)?;
            } else {
                *file_count += 1;
                // 仅搜索文本类型或常见代码后缀
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();
                    if ["ts", "js", "vue", "rs", "json", "toml", "md", "go", "txt", "html", "css"].contains(&ext_str.as_str()) {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            search_in_text(&content, query, &path, results);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

// ══════════════════════════════════════════════════════════════
// 5. 精确编辑工具 (patch_file) — 带备份+模糊匹配回退
// ══════════════════════════════════════════════════════════════
/// 尝试模糊匹配：移除所有空白差异后比较
fn fuzzy_match(content: &str, target: &str) -> Option<usize> {
    let normalize = |s: &str| -> String {
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    };
    let norm_target = normalize(target);
    if norm_target.is_empty() { return None; }

    // 在每行组合中搜索规范化后的目标
    let lines: Vec<&str> = content.lines().collect();
    let target_lines: Vec<&str> = target.lines().collect();
    if target_lines.is_empty() { return None; }

    for i in 0..lines.len() {
        if i + target_lines.len() > lines.len() { break; }
        let window: Vec<&str> = lines[i..i + target_lines.len()].to_vec();
        let norm_window = normalize(&window.join("\n"));
        if norm_window == norm_target {
            // 重建原始窗口文本（带原始缩进）
            let original_slice = lines[i..i + target_lines.len()].join("\n");
            // 在 content 中找到这个原始片段的位置
            if let Some(pos) = content.find(&original_slice) {
                return Some(pos);
            }
        }
    }
    None
}

pub struct PatchFileTool;

#[async_trait]
impl Tool for PatchFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "patch_file".into(),
            description: "精确替换文件中的特定代码块。自动备份原文件。支持模糊匹配（忽略空白差异时的降级匹配）。".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "文件路径，支持绝对路径或相对于工作区根目录的相对路径"
                    },
                    "targetContent": {
                        "type": "string",
                        "description": "需要被替换的精确原代码行块，必须与文件中的内容完全匹配"
                    },
                    "replacementContent": {
                        "type": "string",
                        "description": "要替换进去的新代码行块"
                    }
                },
                "required": ["path", "targetContent", "replacementContent"]
            }),
            risk_level: RiskLevel::High,
        }
    }

    async fn execute(&self, arguments: &Value, sandbox: &SecuritySandbox) -> ToolResult {
        let path_str = match arguments["path"].as_str() {
            Some(p) => p,
            None => return ToolResult { success: false, output: "".into(), error: Some("缺失 'path' 参数。".into()) }
        };
        let target_content = match arguments["targetContent"].as_str().or_else(|| arguments["target_content"].as_str()) {
            Some(t) => t,
            None => return ToolResult { success: false, output: "".into(), error: Some("缺失 'targetContent' 参数。".into()) }
        };
        let replacement_content = match arguments["replacementContent"].as_str().or_else(|| arguments["replacement_content"].as_str()) {
            Some(r) => r,
            None => return ToolResult { success: false, output: "".into(), error: Some("缺失 'replacementContent' 参数。".into()) }
        };

        match sandbox.validate_write_path(path_str) {
            Ok(verified_path) => {
                match std::fs::read_to_string(&verified_path) {
                    Ok(original) => {
                        // 先尝试精确匹配
                        let count = original.matches(target_content).count();
                        let modified = if count == 1 {
                            // 精确匹配成功
                            create_backup(&verified_path).ok();
                            original.replacen(target_content, replacement_content, 1)
                        } else if count == 0 {
                            // 精确匹配失败，尝试模糊匹配
                            match fuzzy_match(&original, target_content) {
                                Some(pos) => {
                                    let end = pos + target_content.len();
                                    if original[pos..end] == *target_content {
                                        // 精确位置也匹配
                                        create_backup(&verified_path).ok();
                                        let mut result = original[..pos].to_string();
                                        result.push_str(replacement_content);
                                        result.push_str(&original[end..]);
                                        result
                                    } else {
                                        return ToolResult {
                                            success: false,
                                            output: "".into(),
                                            error: Some(format!(
                                                "精确匹配失败（未找到完全匹配的片段）。模糊匹配发现目标代码在位置 {}，\n\
                                                 但缩进/空白可能不一致。请提供更精确的 targetContent，\n\
                                                 包含完整的前导空格和换行符。\n\n\
                                                 提示：尝试从文件中直接复制目标代码块。",
                                                pos
                                            ))
                                        };
                                    }
                                }
                                None => {
                                    return ToolResult {
                                        success: false,
                                        output: "".into(),
                                        error: Some(
                                            "在文件中没有找到目标内容。请确认：\n\
                                             1. targetContent 的缩进和空白与文件完全一致\n\
                                             2. 目标代码块在当前文件中存在\n\
                                             3. 尝试复制文件中的原始内容作为 targetContent"
                                            .into()
                                        )
                                    };
                                }
                            }
                        } else {
                            return ToolResult {
                                success: false,
                                output: "".into(),
                                error: Some(format!(
                                    "目标内容在文件中不唯一（匹配到 {} 处）。\n\
                                     请提供更多上下文行以使目标唯一，\n\
                                     例如包含目标代码前后各 2-3 行。",
                                    count
                                ))
                            };
                        };

                        match std::fs::write(&verified_path, &modified) {
                            Ok(_) => {
                                let diff = generate_simple_diff(target_content, replacement_content);
                                ToolResult {
                                    success: true,
                                    output: format!("修改文件成功！差异 Diff 预览如下：\n{}", diff),
                                    error: None,
                                }
                            }
                            Err(e) => ToolResult { success: false, output: "".into(), error: Some(format!("修改写回失败: {}", e)) }
                        }
                    }
                    Err(e) => ToolResult { success: false, output: "".into(), error: Some(format!("读取文件失败: {}", e)) }
                }
            }
            Err(err_msg) => ToolResult { success: false, output: "".into(), error: Some(err_msg) }
        }
    }
}

pub fn generate_simple_diff(target: &str, replacement: &str) -> String {
    let mut diff = String::new();
    for line in target.lines() {
        diff.push_str(&format!("- {}\n", line));
    }
    for line in replacement.lines() {
        diff.push_str(&format!("+ {}\n", line));
    }
    diff
}

// ══════════════════════════════════════════════════════════════
// 6. 静态符号检索工具 (find_symbols) — 基于 Tree-sitter AST 解析
// ══════════════════════════════════════════════════════════════
pub struct FindSymbolsTool;

#[async_trait]
impl Tool for FindSymbolsTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "find_symbols".into(),
            description: "通过 Tree-sitter AST 精确提取目标代码文件的类、函数、方法与结构体大纲（支持 TS/JS/Vue/Rust）。".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "文件路径，支持绝对路径或相对于工作区根目录的相对路径"
                    }
                },
                "required": ["path"]
            }),
            risk_level: RiskLevel::Low,
        }
    }

    async fn execute(&self, arguments: &Value, sandbox: &SecuritySandbox) -> ToolResult {
        let path_str = match arguments["path"].as_str() {
            Some(p) => p,
            None => return ToolResult { success: false, output: "".into(), error: Some("缺失 'path' 参数。".into()) }
        };

        match sandbox.validate_read_path(path_str) {
            Ok(verified_path) => {
                let mut parser = super::ast_parser::AstParser::new();
                match parser.parse_file(&verified_path) {
                    Ok(symbols) => {
                        if symbols.is_empty() {
                            ToolResult { success: true, output: "未能在文件中检测到清晰的类/函数/方法定义符号。".into(), error: None }
                        } else {
                            let mut outline = String::from("文件中的符号大纲如下（Tree-sitter AST 解析）：\n");
                            for sym in &symbols {
                                let vis = sym.visibility.as_deref().unwrap_or("");
                                let parent = sym.parent.as_deref().map(|p| format!(" (∈ {})", p)).unwrap_or_default();
                                outline.push_str(&format!(
                                    "第 {} 行: {:?}{} {}{}\n",
                                    sym.line, sym.kind, vis, sym.name, parent
                                ));
                            }
                            ToolResult { success: true, output: outline, error: None }
                        }
                    }
                    Err(e) => ToolResult { success: false, output: "".into(), error: Some(format!("AST 解析失败: {}", e)) }
                }
            }
            Err(err_msg) => ToolResult { success: false, output: "".into(), error: Some(err_msg) }
        }
    }
}

// ══════════════════════════════════════════════════════════════
// 7. 列出目录内容工具 (list_dir)
// ══════════════════════════════════════════════════════════════
pub struct ListDirTool;

#[async_trait]
impl Tool for ListDirTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "list_dir".into(),
            description: "列出指定目录下的文件和子目录（名称、类型、大小）。不会递归子目录。".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "目录路径，支持绝对路径或相对于工作区根目录的相对路径"
                    }
                },
                "required": ["path"]
            }),
            risk_level: RiskLevel::Low,
        }
    }

    async fn execute(&self, arguments: &Value, sandbox: &SecuritySandbox) -> ToolResult {
        let path_str = match arguments["path"].as_str() {
            Some(p) => p,
            None => return ToolResult { success: false, output: "".into(), error: Some("缺失 'path' 参数。".into()) }
        };

        match sandbox.validate_read_path(path_str) {
            Ok(verified_path) => {
                if !verified_path.is_dir() {
                    return ToolResult { success: false, output: "".into(), error: Some("指定的路径不是目录。".into()) };
                }
                match std::fs::read_dir(&verified_path) {
                    Ok(entries) => {
                        let mut items: Vec<String> = Vec::new();
                        for entry in entries.flatten() {
                            let path = entry.path();
                            let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                            let file_type = if path.is_dir() { "📁 dir" } else { "📄 file" };
                            let size = if path.is_file() {
                                std::fs::metadata(&path).ok().map(|m| m.len()).unwrap_or(0)
                            } else { 0 };
                            let size_str = if size > 1024 * 1024 {
                                format!("{:.1}MB", size as f64 / (1024.0 * 1024.0))
                            } else if size > 1024 {
                                format!("{:.1}KB", size as f64 / 1024.0)
                            } else {
                                format!("{}B", size)
                            };
                            items.push(format!("{}  {}  {}", file_type, name, if path.is_file() { size_str } else { String::new() }));
                        }
                        items.sort();
                        let output = format!("📂 {} 下的内容（{} 项）:\n{}",
                            verified_path.to_string_lossy(),
                            items.len(),
                            items.join("\n")
                        );
                        ToolResult { success: true, output, error: None }
                    }
                    Err(e) => ToolResult { success: false, output: "".into(), error: Some(format!("读取目录失败: {}", e)) }
                }
            }
            Err(err_msg) => ToolResult { success: false, output: "".into(), error: Some(err_msg) }
        }
    }
}

// ══════════════════════════════════════════════════════════════
// 8. 文件名搜索工具 (file_search) — 按文件名关键词查找
// ══════════════════════════════════════════════════════════════
pub struct FileSearchTool;

#[async_trait]
impl Tool for FileSearchTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "file_search".into(),
            description: "按文件名关键词在指定目录中搜索文件（支持模糊匹配）。例如搜索 'command' 可找到 CommandDialog.vue。".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "文件名关键词（不区分大小写，支持部分匹配）"
                    },
                    "path": {
                        "type": "string",
                        "description": "搜索根目录（可选，默认工作区根目录）"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "最大返回数量（可选，默认 50）"
                    }
                },
                "required": ["name"]
            }),
            risk_level: RiskLevel::Low,
        }
    }

    async fn execute(&self, arguments: &Value, sandbox: &SecuritySandbox) -> ToolResult {
        let name_query = match arguments["name"].as_str() {
            Some(q) => q.to_lowercase(),
            None => return ToolResult { success: false, output: "".into(), error: Some("缺失 'name' 参数。".into()) }
        };
        let path_str = arguments["path"].as_str().unwrap_or(".");
        let max_results = arguments["max_results"].as_i64().unwrap_or(50) as usize;

        let target_path = match sandbox.validate_read_path(path_str) {
            Ok(p) => p,
            Err(e) => return ToolResult { success: false, output: "".into(), error: Some(format!("搜索路径校验失败: {}", e)) }
        };

        let exclude_dirs = ["node_modules", ".git", "target", "dist", "build", "out", "gen"];

        fn walk(dir: &Path, query: &str, max: usize, count: &mut usize, results: &mut Vec<String>, exclude: &[&str]) {
            if *count >= max || results.len() >= max {
                return;
            }
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    if *count >= max || results.len() >= max {
                        break;
                    }
                    let path = entry.path();
                    let name = path.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
                    if path.is_dir() {
                        if !exclude.contains(&name.as_str()) {
                            walk(&path, query, max, count, results, exclude);
                        }
                    } else if name.contains(query) {
                        *count += 1;
                        let size = std::fs::metadata(&path).ok().map(|m| m.len()).unwrap_or(0);
                        let size_str = if size > 1024 * 1024 {
                            format!("{:.1}MB", size as f64 / (1024.0 * 1024.0))
                        } else if size > 1024 {
                            format!("{:.1}KB", size as f64 / 1024.0)
                        } else {
                            format!("{}B", size)
                        };
                        results.push(format!("{}  ({} bytes {})", path.to_string_lossy(), size, size_str));
                    }
                }
            }
        }

        let mut results = Vec::new();
        let mut count = 0usize;
        walk(&target_path, &name_query, max_results, &mut count, &mut results, &exclude_dirs);

        if results.is_empty() {
            ToolResult { success: true, output: format!("未找到文件名包含 '{}' 的文件。", name_query), error: None }
        } else {
            let output = format!("找到 {} 个匹配的文件（最多显示 {} 个）:\n{}", count, max_results, results.join("\n"));
            ToolResult { success: true, output, error: None }
        }
    }
}

// ══════════════════════════════════════════════════════════════
// 9. 高级 grep 搜索工具 (grep_search) — 支持正则、文件过滤
// ══════════════════════════════════════════════════════════════
pub struct GrepSearchTool;

#[async_trait]
impl Tool for GrepSearchTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "grep_search".into(),
            description: "在文件内容中搜索文本或正则表达式。支持文件过滤和大小写控制。比 grep 工具更强大。".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "搜索模式（支持纯文本和正则表达式）"
                    },
                    "path": {
                        "type": "string",
                        "description": "要搜索的目录或文件路径（可选，默认工作区根目录）"
                    },
                    "include": {
                        "type": "string",
                        "description": "文件后缀过滤，如 '*.rs'、'*.ts,*.vue'（可选）"
                    },
                    "is_regex": {
                        "type": "boolean",
                        "description": "是否将 pattern 解释为正则表达式（可选，默认 false）"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "最大返回行数（可选，默认 50）"
                    }
                },
                "required": ["pattern"]
            }),
            risk_level: RiskLevel::Low,
        }
    }

    async fn execute(&self, arguments: &Value, sandbox: &SecuritySandbox) -> ToolResult {
        let pattern_str = match arguments["pattern"].as_str() {
            Some(p) => p,
            None => return ToolResult { success: false, output: "".into(), error: Some("缺失 'pattern' 参数。".into()) }
        };
        let path_str = arguments["path"].as_str().unwrap_or(".");
        let include_str = arguments["include"].as_str().unwrap_or("");
        let is_regex = arguments["is_regex"].as_bool().unwrap_or(false);
        let max_results = arguments["max_results"].as_i64().unwrap_or(50) as usize;

        let target_path = match sandbox.validate_read_path(path_str) {
            Ok(p) => p,
            Err(e) => return ToolResult { success: false, output: "".into(), error: Some(format!("搜索路径校验失败: {}", e)) }
        };

        // 解析文件包含扩展名
        let include_exts: Vec<&str> = if include_str.is_empty() {
            vec![]
        } else {
            include_str.split(',').map(|s| {
                let s = s.trim().trim_start_matches('*').trim_start_matches('.');
                s
            }).collect()
        };

        let exclude_dirs = ["node_modules", ".git", "target", "dist", "build", "out", "gen"];

        fn is_matching_ext(path: &Path, exts: &[&str]) -> bool {
            if exts.is_empty() { return true; }
            path.extension().map(|e| exts.contains(&e.to_string_lossy().as_ref())).unwrap_or(false)
        }

        let mut results = Vec::new();
        let total_count: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

        fn walk_search(
            dir: &Path, pattern: &str, is_regex: bool, max: usize,
            count: &std::sync::atomic::AtomicUsize,
            results: &mut Vec<String>, exclude: &[&str], exts: &[&str],
        ) {
            if count.load(std::sync::atomic::Ordering::Relaxed) >= max {
                return;
            }
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    if count.load(std::sync::atomic::Ordering::Relaxed) >= max {
                        break;
                    }
                    let path = entry.path();
                    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    if path.is_dir() {
                        if !exclude.contains(&name.as_str()) {
                            walk_search(&path, pattern, is_regex, max, count, results, exclude, exts);
                        }
                    } else if is_matching_ext(&path, exts) {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            for (idx, line) in content.lines().enumerate() {
                                if count.load(std::sync::atomic::Ordering::Relaxed) >= max {
                                    break;
                                }
                                let matched = if is_regex {
                                    regex::Regex::new(pattern).ok().map(|r| r.is_match(line)).unwrap_or(false)
                                } else {
                                    line.to_lowercase().contains(&pattern.to_lowercase())
                                };
                                if matched {
                                    count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                    results.push(format!("{}:{}: {}",
                                        path.to_string_lossy(), idx + 1, line.trim()));
                                }
                            }
                        }
                    }
                }
            }
        }

        walk_search(&target_path, pattern_str, is_regex, max_results, &total_count, &mut results, &exclude_dirs, &include_exts);

        if results.is_empty() {
            ToolResult { success: true, output: "未找到匹配项。".into(), error: None }
        } else {
            let count = total_count.load(std::sync::atomic::Ordering::Relaxed);
            let mut output = format!("找到 {} 个匹配（显示前 {} 个）:\n", count, max_results);
            output.push_str(&results.join("\n"));
            if count > max_results {
                output.push_str(&format!("\n...（还有 {} 个匹配未显示）", count - max_results));
            }
            ToolResult { success: true, output, error: None }
        }
    }
}

// ══════════════════════════════════════════════════════════════
// 10. 读取文件指定行范围工具 (read_file_range)
// ══════════════════════════════════════════════════════════════
pub struct ReadFileRangeTool;

#[async_trait]
impl Tool for ReadFileRangeTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "read_file_range".into(),
            description: "读取文件的指定行范围，适用于大文件分段读取。行号从 1 开始计数。".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "文件路径，支持绝对路径或相对于工作区根目录的相对路径"
                    },
                    "start_line": {
                        "type": "integer",
                        "description": "起始行号（从 1 开始，可选，默认 1）"
                    },
                    "end_line": {
                        "type": "integer",
                        "description": "结束行号（包含，可选，默认读取到文件末尾）"
                    }
                },
                "required": ["path"]
            }),
            risk_level: RiskLevel::Low,
        }
    }

    async fn execute(&self, arguments: &Value, sandbox: &SecuritySandbox) -> ToolResult {
        let path_str = match arguments["path"].as_str() {
            Some(p) => p,
            None => return ToolResult { success: false, output: "".into(), error: Some("缺失 'path' 参数。".into()) }
        };
        let start_line = arguments["start_line"].as_i64().unwrap_or(1).max(1) as usize;
        let end_line = arguments["end_line"].as_i64().map(|e| e as usize);

        match sandbox.validate_read_path(path_str) {
            Ok(verified_path) => {
                match std::fs::read_to_string(&verified_path) {
                    Ok(content) => {
                        let lines: Vec<&str> = content.lines().collect();
                        let total_lines = lines.len();
                        let end = end_line.unwrap_or(total_lines).min(total_lines);

                        if start_line > total_lines {
                            return ToolResult {
                                success: false,
                                output: "".into(),
                                error: Some(format!("起始行 {} 超出文件总行数 {}", start_line, total_lines))
                            };
                        }
                        if start_line > end {
                            return ToolResult {
                                success: false,
                                output: "".into(),
                                error: Some(format!("起始行 {} 大于结束行 {}", start_line, end))
                            };
                        }

                        let mut output = String::new();
                        for (i, line) in lines[start_line - 1..end].iter().enumerate() {
                            output.push_str(&format!("{:>6}  {}\n", start_line + i, line));
                        }
                        let summary = format!("📄 {} (行 {}-{}，共 {} 行):\n{}",
                            verified_path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default(),
                            start_line, end, total_lines, output);
                        ToolResult { success: true, output: summary, error: None }
                    }
                    Err(e) => ToolResult { success: false, output: "".into(), error: Some(format!("读取文件失败: {}", e)) }
                }
            }
            Err(err_msg) => ToolResult { success: false, output: "".into(), error: Some(err_msg) }
        }
    }
}

// ══════════════════════════════════════════════════════════════
// 11. 代码定义列表工具 (list_code_definitions) — 基于 Tree-sitter
// ══════════════════════════════════════════════════════════════
pub struct ListCodeDefinitionsTool;

#[async_trait]
impl Tool for ListCodeDefinitionsTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "list_code_definitions".into(),
            description: "使用 Tree-sitter AST 提取代码文件中的函数、类、接口、结构体定义列表。支持 TypeScript、Rust、Vue SFC。".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "文件路径，支持绝对路径或相对于工作区根目录的相对路径"
                    },
                    "kind": {
                        "type": "string",
                        "description": "过滤符号类型：function / class / interface / struct / all（可选，默认 all）"
                    }
                },
                "required": ["path"]
            }),
            risk_level: RiskLevel::Low,
        }
    }

    async fn execute(&self, arguments: &Value, sandbox: &SecuritySandbox) -> ToolResult {
        let path_str = match arguments["path"].as_str() {
            Some(p) => p,
            None => return ToolResult { success: false, output: "".into(), error: Some("缺失 'path' 参数。".into()) }
        };
        let filter_kind = arguments["kind"].as_str().unwrap_or("all").to_lowercase();

        match sandbox.validate_read_path(path_str) {
            Ok(verified_path) => {
                let mut parser = super::ast_parser::AstParser::new();
                match parser.parse_file(&verified_path) {
                    Ok(symbols) => {
                        let filtered: Vec<_> = symbols.iter()
                            .filter(|s| {
                                if filter_kind == "all" { true }
                                else {
                                    let kind_str = format!("{:?}", s.kind).to_lowercase();
                                    kind_str.contains(&filter_kind)
                                }
                            })
                            .collect();

                        if filtered.is_empty() {
                            ToolResult {
                                success: true,
                                output: format!("未找到{}匹配的代码定义。", if filter_kind != "all" { format!(" '{}' 类型", filter_kind) } else { "".into() }),
                                error: None,
                            }
                        } else {
                            let mut lines: Vec<String> = Vec::new();
                            lines.push(format!("📋 {} 中的代码定义（共 {} 个）:\n", verified_path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default(), filtered.len()));
                            lines.push(format!("{:<8} {:<12} {:<8} {}", "行号", "类型", "可见性", "名称"));
                            lines.push(format!("{}", "-".repeat(60)));

                            for sym in &filtered {
                                let vis = sym.visibility.as_deref().unwrap_or("");
                                let parent = sym.parent.as_deref().map(|p| format!(" (∈ {})", p)).unwrap_or_default();
                                lines.push(format!("{:<8} {:<12} {:<8} {}{}", sym.line, format!("{:?}", sym.kind), vis, sym.name, parent));
                            }
                            ToolResult { success: true, output: lines.join("\n"), error: None }
                        }
                    }
                    Err(e) => ToolResult { success: false, output: "".into(), error: Some(format!("AST 解析失败: {}", e)) }
                }
            }
            Err(err_msg) => ToolResult { success: false, output: "".into(), error: Some(err_msg) }
        }
    }
}

// ══════════════════════════════════════════════════════════════
// ToolRegistry 实现 — 注册表模式 + 调用统计 + 安全分级
// ══════════════════════════════════════════════════════════════
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallStats {
    pub tool_name: String,
    pub call_count: u64,
    pub success_count: u64,
    pub fail_count: u64,
    pub risk_level: RiskLevel,
}

pub struct ToolRegistry {
    tools: RwLock<HashMap<String, Arc<dyn Tool>>>,
    stats: RwLock<HashMap<String, ToolCallStats>>,
    sandbox: SecuritySandbox,
}

impl ToolRegistry {
    pub fn new(sandbox: SecuritySandbox) -> Self {
        let mut tools: HashMap<String, Arc<dyn Tool>> = HashMap::new();
        // 按类型分组注册
        let file_tools: Vec<(String, Arc<dyn Tool>, RiskLevel)> = vec![
            ("read_file".into(), Arc::new(ReadFileTool) as Arc<dyn Tool>, RiskLevel::Low),
            ("read_file_range".into(), Arc::new(ReadFileRangeTool) as Arc<dyn Tool>, RiskLevel::Low),
            ("write_file".into(), Arc::new(WriteFileTool) as Arc<dyn Tool>, RiskLevel::High),
            ("patch_file".into(), Arc::new(PatchFileTool) as Arc<dyn Tool>, RiskLevel::High),
            ("list_dir".into(), Arc::new(ListDirTool) as Arc<dyn Tool>, RiskLevel::Low),
            ("file_search".into(), Arc::new(FileSearchTool) as Arc<dyn Tool>, RiskLevel::Low),
        ];
        let search_tools: Vec<(String, Arc<dyn Tool>, RiskLevel)> = vec![
            ("grep".into(), Arc::new(GrepTool) as Arc<dyn Tool>, RiskLevel::Low),
            ("grep_search".into(), Arc::new(GrepSearchTool) as Arc<dyn Tool>, RiskLevel::Low),
            ("find_symbols".into(), Arc::new(FindSymbolsTool) as Arc<dyn Tool>, RiskLevel::Low),
            ("list_code_definitions".into(), Arc::new(ListCodeDefinitionsTool) as Arc<dyn Tool>, RiskLevel::Low),
        ];
        let exec_tools: Vec<(String, Arc<dyn Tool>, RiskLevel)> = vec![
            ("run_command".into(), Arc::new(RunCommandTool) as Arc<dyn Tool>, RiskLevel::Critical),
        ];

        for (name, tool, _level) in file_tools.into_iter().chain(search_tools).chain(exec_tools) {
            tools.insert(name, tool);
        }

        Self {
            tools: RwLock::new(tools),
            stats: RwLock::new(HashMap::new()),
            sandbox,
        }
    }

    /// 注册新工具（用于运行时动态添加，如 MCP 工具）
    pub async fn register(&self, name: String, tool: Arc<dyn Tool>) {
        self.tools.write().await.insert(name, tool);
    }

    /// 获取所有工具定义列表（用于 Agent System Prompt）
    pub async fn get_definitions(&self) -> Vec<ToolDefinition> {
        let tools = self.tools.read().await;
        tools.values().map(|t| t.definition()).collect()
    }

    pub async fn get_tool(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.read().await.get(name).cloned()
    }

    /// 执行工具并记录统计
    pub async fn execute(&self, name: &str, arguments: &Value) -> ToolResult {
        let tool = {
            let tools = self.tools.read().await;
            tools.get(name).cloned()
        };
        if let Some(tool) = tool {
            let result = tool.execute(arguments, &self.sandbox).await;
            // 记录统计
            let mut stats = self.stats.write().await;
            let entry = stats.entry(name.to_string()).or_insert(ToolCallStats {
                tool_name: name.to_string(),
                call_count: 0,
                success_count: 0,
                fail_count: 0,
                risk_level: tool.definition().risk_level,
            });
            entry.call_count += 1;
            if result.success {
                entry.success_count += 1;
            } else {
                entry.fail_count += 1;
            }
            result
        } else {
            ToolResult {
                success: false,
                output: "".into(),
                error: Some(format!("工具 '{}' 未在注册表中注册", name)),
            }
        }
    }

    /// 获取工具调用统计
    pub async fn get_stats(&self) -> Vec<ToolCallStats> {
        self.stats.read().await.values().cloned().collect()
    }

    /// 获取高风险工具列表（需审批队列拦截）
    pub async fn get_high_risk_tools(&self) -> Vec<String> {
        let tools = self.tools.read().await;
        tools.iter()
            .filter(|(_, t)| matches!(t.definition().risk_level, RiskLevel::High | RiskLevel::Critical))
            .map(|(name, _)| name.clone())
            .collect()
    }

    pub fn sandbox(&self) -> &SecuritySandbox {
        &self.sandbox
    }
}
