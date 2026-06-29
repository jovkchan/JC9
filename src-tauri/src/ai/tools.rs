use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::path::Path;
use std::process::Command;
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
// 2. 写文件工具 (write_file)
// ══════════════════════════════════════════════════════════════
pub struct WriteFileTool;

#[async_trait]
impl Tool for WriteFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "write_file".into(),
            description: "写入内容至指定文件。仅允许在工作区内写入，不允许覆盖只读白名单路径。".into(),
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
                match std::fs::write(&verified_path, content) {
                    Ok(_) => ToolResult { success: true, output: "文件写入成功。".into(), error: None },
                    Err(e) => ToolResult { success: false, output: "".into(), error: Some(format!("文件写入失败: {}", e)) }
                }
            }
            Err(err_msg) => ToolResult { success: false, output: "".into(), error: Some(err_msg) }
        }
    }
}

// ══════════════════════════════════════════════════════════════
// 3. 执行命令行工具 (run_command)
// ══════════════════════════════════════════════════════════════
pub struct RunCommandTool;

#[async_trait]
impl Tool for RunCommandTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "run_command".into(),
            description: "在指定的目录下运行系统命令。仅允许执行白名单内的安全命令，且工作目录必须在工作区内。".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "要运行的命令（如 'cargo check'）"
                    },
                    "working_dir": {
                        "type": "string",
                        "description": "执行命令的目录路径（默认是当前项目工作区根目录）"
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

        // 3. 执行命令
        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = Command::new("powershell");
            c.args(["-NoProfile", "-Command", command_str]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-c", command_str]);
            c
        };

        cmd.current_dir(exec_path);

        match cmd.output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let success = output.status.success();
                
                let mut combined_output = stdout;
                if !stderr.is_empty() {
                    if !combined_output.is_empty() {
                        combined_output.push_str("\n");
                    }
                    combined_output.push_str("【错误/标准错误输出】:\n");
                    combined_output.push_str(&stderr);
                }

                if success {
                    ToolResult { success: true, output: combined_output, error: None }
                } else {
                    ToolResult {
                        success: false,
                        output: combined_output,
                        error: Some(format!("命令以退出码 {:?} 失败。", output.status.code())),
                    }
                }
            }
            Err(e) => ToolResult { success: false, output: "".into(), error: Some(format!("执行进程启动失败: {}", e)) }
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
// 5. 精确编辑工具 (patch_file)
// ══════════════════════════════════════════════════════════════
pub struct PatchFileTool;

#[async_trait]
impl Tool for PatchFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "patch_file".into(),
            description: "精确替换文件中的特定代码块。这能有效避免全文件重写，提供精准的代码修改。".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "文件路径，支持绝对路径或相对于工作区根目录的相对路径"
                    },
                    "targetContent": {
                        "type": "string",
                        "description": "需要被替换的精确原代码行块，必须与文件中的内容完全匹配（包含前导空格和换行符）"
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
                        let count = original.matches(target_content).count();
                        if count == 0 {
                            return ToolResult {
                                success: false,
                                output: "".into(),
                                error: Some("在文件中没有找到目标 'targetContent'，请确认空格缩进和换行符是否一致。".into())
                            };
                        } else if count > 1 {
                            return ToolResult {
                                success: false,
                                output: "".into(),
                                error: Some(format!("目标 'targetContent' 在文件中不唯一（匹配到 {} 处），请提供更多行或更具唯一性的目标上下文。", count))
                            };
                        }

                        let modified = original.replacen(target_content, replacement_content, 1);
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
// ToolRegistry 实现
// ══════════════════════════════════════════════════════════════
pub struct ToolRegistry {
    tools: RwLock<HashMap<String, Arc<dyn Tool>>>,
    sandbox: SecuritySandbox,
}

impl ToolRegistry {
    pub fn new(sandbox: SecuritySandbox) -> Self {
        let mut tools = HashMap::new();
        tools.insert("read_file".into(), Arc::new(ReadFileTool) as Arc<dyn Tool>);
        tools.insert("write_file".into(), Arc::new(WriteFileTool) as Arc<dyn Tool>);
        tools.insert("run_command".into(), Arc::new(RunCommandTool) as Arc<dyn Tool>);
        tools.insert("grep".into(), Arc::new(GrepTool) as Arc<dyn Tool>);
        tools.insert("patch_file".into(), Arc::new(PatchFileTool) as Arc<dyn Tool>);
        tools.insert("find_symbols".into(), Arc::new(FindSymbolsTool) as Arc<dyn Tool>);

        Self {
            tools: RwLock::new(tools),
            sandbox,
        }
    }

    pub async fn register(&self, name: String, tool: Arc<dyn Tool>) {
        self.tools.write().await.insert(name, tool);
    }

    pub async fn get_definitions(&self) -> Vec<ToolDefinition> {
        let tools = self.tools.read().await;
        tools.values().map(|t| t.definition()).collect()
    }

    pub async fn get_tool(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.read().await.get(name).cloned()
    }

    pub async fn execute(&self, name: &str, arguments: &Value) -> ToolResult {
        let tool = {
            let tools = self.tools.read().await;
            tools.get(name).cloned()
        };
        if let Some(tool) = tool {
            tool.execute(arguments, &self.sandbox).await
        } else {
            ToolResult {
                success: false,
                output: "".into(),
                error: Some(format!("工具 '{}' 未在注册表中注册", name)),
            }
        }
    }

    pub fn sandbox(&self) -> &SecuritySandbox {
        &self.sandbox
    }
}
