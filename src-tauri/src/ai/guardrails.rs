use serde_json::Value;
use std::path::Path;
use regex::Regex;
use super::security::SecuritySandbox;

/// Guardrail 校验结果等级
#[derive(Debug, Clone, PartialEq)]
pub enum GuardrailLevel {
    /// 完全通过
    Pass,
    /// 警告（仅日志记录，不阻塞）
    Warning(String),
    /// 错误（自动重试，提示 Agent 修正）
    Error(String),
    /// 严重（进入审批队列，由前端调用方自行处理）
    Critical(String),
}

impl GuardrailLevel {
    pub fn is_blocking(&self) -> bool {
        matches!(self, Self::Error(_) | Self::Critical(_))
    }

    pub fn message(&self) -> Option<&str> {
        match self {
            Self::Pass => None,
            Self::Warning(msg) | Self::Error(msg) | Self::Critical(msg) => Some(msg.as_str()),
        }
    }
}

/// Guardrails — Agent 工具调用参数预校验
///
/// 在每次工具执行前调用，拦截明显错误的参数，减少无效的 LLM 往返。
/// 校验覆盖：路径存在性、命令安全性、参数类型、正则合法性。
pub struct Guardrails<'a> {
    sandbox: &'a SecuritySandbox,
}

impl<'a> Guardrails<'a> {
    pub fn new(sandbox: &'a SecuritySandbox) -> Self {
        Self { sandbox }
    }

    /// 对工具调用进行完整预校验，返回最高严重等级的问题
    pub async fn validate(&self, tool_name: &str, arguments: &Value) -> GuardrailLevel {
        // 1. 路径存在性与父目录校验
        if let Some(level) = self.validate_paths(tool_name, arguments) {
            if level.is_blocking() { return level; }
        }

        // 2. 命令安全性校验
        if let Some(level) = self.validate_command_safety(tool_name, arguments) {
            if level.is_blocking() { return level; }
        }

        // 3. 正则合法性校验
        if let Some(level) = self.validate_regex(tool_name, arguments) {
            if level.is_blocking() { return level; }
        }

        // 4. 参数基本类型校验
        if let Some(level) = self.validate_arg_types(tool_name, arguments) {
            if level.is_blocking() { return level; }
        }

        GuardrailLevel::Pass
    }

    // ── 1. 路径校验 ──

    fn validate_paths(&self, tool_name: &str, arguments: &Value) -> Option<GuardrailLevel> {
        // 提取所有可能的路径参数
        let path_keys = ["path", "root", "working_dir", "workingDir", "file_path", "filePath"];
        let paths: Vec<String> = path_keys.iter()
            .filter_map(|k| arguments.get(k).and_then(|v| v.as_str()).map(|s| s.to_string()))
            .collect();

        if paths.is_empty() {
            return None;
        }

        for path_str in &paths {
            let p = Path::new(path_str);

            // 写操作：校验父目录必须存在
            let is_write_op = matches!(tool_name, "write_file" | "patch_file" | "write_file_binary");
            if is_write_op {
                if let Some(parent) = p.parent() {
                    if !parent.as_os_str().is_empty() && !parent.exists() {
                        return Some(GuardrailLevel::Error(
                            format!("父目录不存在: `{}`。请先创建目录后再写入。", parent.display())
                        ));
                    }
                }
            }

            // 读操作：校验文件必须存在（如果路径看起来像文件路径）
            let is_read_op = matches!(tool_name, "read_file" | "read_file_range"
                | "list_code_definitions" | "find_symbols" | "grep" | "grep_search");
            if is_read_op {
                // 如果路径有扩展名或看起来像文件，检查存在性
                if p.extension().is_some() || p.is_file() {
                    if !p.exists() {
                        return Some(GuardrailLevel::Error(
                            format!("文件不存在: `{}`。请检查路径是否正确。", path_str)
                        ));
                    }
                } else if !p.exists() {
                    return Some(GuardrailLevel::Warning(
                        format!("路径不存在: `{}`。Agent 可能会收到错误。", path_str)
                    ));
                }
            }
        }

        None
    }

    // ── 2. 命令安全性校验 ──

    fn validate_command_safety(&self, tool_name: &str, arguments: &Value) -> Option<GuardrailLevel> {
        if tool_name != "run_command" {
            return None;
        }

        let command = arguments.get("command").and_then(|v| v.as_str())?;

        // 安全检查（复用 SecuritySandbox 的 validate_command）
        if !self.sandbox.validate_command(command) {
            return Some(GuardrailLevel::Critical(
                format!("命令被安全沙箱拦截: `{}`。该命令含有黑名单模式或未在白名单中。", command)
            ));
        }

        // 额外检查：危险命令行参数
        let lower = command.to_lowercase();
        let dangerous_flags = [
            "rm -rf /", "rm -rf --no-preserve-root",
            "format ", "fdisk", "mkfs",
            "dd if=", "> /dev/sda", "> /dev/nvme",
            ":(){ :|:& };:", // fork bomb
        ];
        for flag in &dangerous_flags {
            if lower.contains(flag) {
                return Some(GuardrailLevel::Critical(
                    format!("命令包含危险模式 `{}`，已拦截。", flag)
                ));
            }
        }

        None
    }

    // ── 3. 正则合法性校验 ──

    fn validate_regex(&self, tool_name: &str, arguments: &Value) -> Option<GuardrailLevel> {
        let has_regex_param = matches!(tool_name, "grep" | "grep_search");

        // 获取 pattern 参数（grep_search 用 pattern，grep 可能用第一个参数）
        let pattern = if has_regex_param {
            arguments.get("pattern").and_then(|v| v.as_str())
                .or_else(|| arguments.get("query").and_then(|v| v.as_str()))
        } else {
            None
        };

        if let Some(pattern) = pattern {
            // 检查是否是正则（如果包含特殊字符或 is_regex 为 true）
            let is_regex = arguments.get("is_regex").and_then(|v| v.as_bool()).unwrap_or(false)
                || arguments.get("isRegexp").and_then(|v| v.as_bool()).unwrap_or(false)
                // 启发式检测：包含正则特殊字符
                || pattern.contains(|c: char| matches!(c, '\\' | '[' | ']' | '(' | ')' | '{' | '}' | '+' | '^' | '$'));

            if is_regex {
                match Regex::new(pattern) {
                    Ok(_) => {} // 合法
                    Err(e) => {
                        return Some(GuardrailLevel::Error(
                            format!("正则表达式不合法: `{}`。错误: {}。请修正后重试。", pattern, e)
                        ));
                    }
                }
            }
        }

        None
    }

    // ── 4. 参数类型校验 ──

    fn validate_arg_types(&self, tool_name: &str, arguments: &Value) -> Option<GuardrailLevel> {
        // 读取文件类工具：path 必须是字符串
        if matches!(tool_name, "read_file" | "read_file_range" | "write_file" | "patch_file") {
            if let Some(path) = arguments.get("path") {
                if !path.is_string() {
                    return Some(GuardrailLevel::Error(
                        format!("参数 `path` 应为字符串，但收到了类型 {}", type_name(path))
                    ));
                }
            }
        }

        // grep_search: pattern 必须是字符串
        if matches!(tool_name, "grep" | "grep_search") {
            if let Some(pattern) = arguments.get("pattern") {
                if !pattern.is_string() {
                    return Some(GuardrailLevel::Error(
                        format!("参数 `pattern` 应为字符串，但收到了类型 {}", type_name(pattern))
                    ));
                }
            }
        }

        // run_command: command 必须是字符串
        if tool_name == "run_command" {
            if let Some(command) = arguments.get("command") {
                if !command.is_string() {
                    return Some(GuardrailLevel::Error(
                        format!("参数 `command` 应为字符串，但收到了类型 {}", type_name(command))
                    ));
                }
            }
        }

        // write_file: content 必须是字符串
        if tool_name == "write_file" {
            if let Some(content) = arguments.get("content") {
                if !content.is_string() {
                    return Some(GuardrailLevel::Error(
                        format!("参数 `content` 应为字符串，但收到了类型 {}", type_name(content))
                    ));
                }
            }
        }

        None
    }
}

/// 获取 JSON 值的类型名称（用于错误提示）
fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}
