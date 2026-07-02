use std::path::{Path, PathBuf};
use regex::Regex;

/// 安全边界沙箱 - 越界拦截与只读白名单
#[derive(Clone)]
pub struct SecuritySandbox {
    workspace_root: PathBuf,
    read_only_whitelist: Vec<PathBuf>,
    command_whitelist: Vec<String>,
    command_blacklist: Vec<Regex>,
}

impl SecuritySandbox {
    pub fn new(workspace_root: PathBuf) -> Self {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));

        let read_only_whitelist = vec![
            home.join(".gitconfig"),
            home.join(".npmrc"),
            home.join(".yarnrc"),
            home.join(".yarnrc.yml"),
            home.join(".cargo").join("config.toml"),
            home.join(".cargo").join("config"),
            home.join(".ssh").join("config"),
            home.join(".ssh").join("known_hosts"),
            home.join(".editorconfig"),
            home.join(".vimrc"),
            home.join(".bashrc"),
            home.join(".zshrc"),
            home.join(".profile"),
            home.join(".env"),
            home.join(".nvmrc"),
            home.join(".python-version"),
            home.join(".rustup").join("settings.toml"),
        ];

        let command_whitelist = vec![
            "git".into(), "npm".into(), "npx".into(), "yarn".into(), "pnpm".into(),
            "node".into(), "cargo".into(), "rustc".into(), "rustup".into(),
            "go".into(), "python".into(), "python3".into(), "pip".into(), "pip3".into(),
            "tsc".into(), "eslint".into(), "prettier".into(), "vue-tsc".into(),
            "vite".into(), "webpack".into(), "rollup".into(),
            "cat".into(), "ls".into(), "dir".into(), "echo".into(), "type".into(),
            "grep".into(), "findstr".into(), "find".into(),
            "mkdir".into(), "rmdir".into(), "del".into(), "rm".into(), "cp".into(), "copy".into(), "mv".into(), "move".into(),
            "cd".into(), "pwd".into(), "chdir".into(),
            "where".into(), "which".into(),
            "tasklist".into(), "ps".into(),
        ];

        let command_blacklist = vec![
            // rm -rf 危险变体：覆盖 / ~ . * 等目标，以及参数分开写的情况
            Regex::new(r"(?i)\brm\s+(-[a-z]*r[a-z]*f|--recursive\s+--force)\s+[/~.*]").unwrap(),
            Regex::new(r"(?i)\brm\s+-rf\s+[/~.*]").unwrap(),
            Regex::new(r"(?i)\brm\s+-[a-z]*r[a-z]*\s+-[a-z]*f\s+[/~.*]").unwrap(),
            // 格式化与系统破坏
            Regex::new(r"(?i)\bformat\s+[a-z]:").unwrap(),
            Regex::new(r"(?i)\bshutdown\b").unwrap(),
            Regex::new(r"(?i)\breboot\b").unwrap(),
            Regex::new(r"(?i)\bhalt\b").unwrap(),
            Regex::new(r"(?i)\bmkfs\b").unwrap(),
            Regex::new(r"(?i)\bdd\s+if=").unwrap(),
            // fork bomb
            Regex::new(r"(?i)\b:\(\)\s*\{").unwrap(),
            // 注册表与用户管理
            Regex::new(r"(?i)\breg\s+delete\b").unwrap(),
            Regex::new(r"(?i)\bregedit\b").unwrap(),
            Regex::new(r"(?i)\bnet\s+user\b").unwrap(),
            Regex::new(r"(?i)\bnet\s+localgroup\b").unwrap(),
            // 编码执行与管道注入
            Regex::new(r"(?i)\bpowershell.*-enc\b").unwrap(),
            Regex::new(r"(?i)\bcurl.*\|\s*sh\b").unwrap(),
            Regex::new(r"(?i)\bwget.*\|\s*sh\b").unwrap(),
            // 权限提升
            Regex::new(r"(?i)\bsudo\s+rm\b").unwrap(),
            Regex::new(r"(?i)\bchmod\s+777\s+/").unwrap(),
            Regex::new(r"(?i)\bchown\s+-R\b").unwrap(),
            // 危险写入系统目录
            Regex::new(r"(?i)\bcat\s+.+\s*>\s*/etc/").unwrap(),
            Regex::new(r"(?i)\becho\s+.+\s*>\s*/etc/").unwrap(),
            // 环境变量窃取
            Regex::new(r"(?i)\benv\s*\|\s*(curl|wget)\b").unwrap(),
            Regex::new(r"(?i)\bprintenv\s*\|\s*(curl|wget)\b").unwrap(),
        ];

        Self {
            workspace_root,
            read_only_whitelist,
            command_whitelist,
            command_blacklist,
        }
    }

    pub fn update_workspace_root(&mut self, new_root: PathBuf) {
        self.workspace_root = new_root;
    }

    /// 动态添加只读白名单路径
    pub fn add_read_only_path(&mut self, path: PathBuf) {
        if !self.read_only_whitelist.contains(&path) {
            self.read_only_whitelist.push(path);
        }
    }

    pub fn validate_read_path(&self, path: &str) -> Result<PathBuf, String> {
        let p = Path::new(path);
        let canonical = if p.is_absolute() {
            p.to_path_buf()
        } else {
            self.workspace_root.join(p)
        };

        let normalized = normalize_path(&canonical);

        for whitelist_path in &self.read_only_whitelist {
            if let Ok(wl_canonical) = whitelist_path.canonicalize() {
                if normalized == wl_canonical || normalized.starts_with(&wl_canonical) {
                    return Ok(normalized);
                }
            } else {
                if normalized == *whitelist_path || normalized.starts_with(whitelist_path) {
                    return Ok(normalized);
                }
            }
        }

        if normalized.starts_with(&self.workspace_root) {
            return Ok(normalized);
        }

        Err(format!("越界读取拦截：路径 '{}' 不在工作区或只读白名单内", path))
    }

    pub fn validate_write_path(&self, path: &str) -> Result<PathBuf, String> {
        let p = Path::new(path);
        let canonical = if p.is_absolute() {
            p.to_path_buf()
        } else {
            self.workspace_root.join(p)
        };

        let normalized = normalize_path(&canonical);

        for whitelist_path in &self.read_only_whitelist {
            let wl_normalized = normalize_path(whitelist_path);
            if normalized == wl_normalized || normalized.starts_with(&wl_normalized) {
                return Err(format!("写入拦截：路径 '{}' 在只读白名单中，禁止修改", path));
            }
        }

        if normalized.starts_with(&self.workspace_root) {
            return Ok(normalized);
        }

        Err(format!("越界写入拦截：路径 '{}' 不在工作区内", path))
    }

    pub fn validate_command(&self, command: &str) -> bool {
        let trimmed = command.trim();

        for pattern in &self.command_blacklist {
            if pattern.is_match(trimmed) {
                return false;
            }
        }

        // 使用 shell-words 正确解析带引号的参数（如 echo "hello world"）
        let parts: Vec<String> = match shell_words::split(trimmed) {
            Ok(p) => p,
            Err(_) => {
                // shell-words 解析失败时回退到简单空格分割
                trimmed.split_whitespace().map(|s| s.to_string()).collect()
            }
        };
        if parts.is_empty() {
            return false;
        }

        let main_cmd = if cfg!(target_os = "windows") {
            let start = if parts[0].eq_ignore_ascii_case("cmd") || parts[0].eq_ignore_ascii_case("powershell") || parts[0].eq_ignore_ascii_case("bash") || parts[0].eq_ignore_ascii_case("sh") {
                let mut idx = 1;
                while idx < parts.len() && (parts[idx].starts_with('-') || parts[idx].starts_with('/')) {
                    idx += 1;
                }
                if idx < parts.len() { parts[idx].to_lowercase() } else { return false; }
            } else {
                parts[0].to_lowercase()
            };
            start
        } else {
            let start = if parts[0] == "bash" || parts[0] == "sh" {
                let mut idx = 1;
                while idx < parts.len() && parts[idx].starts_with('-') {
                    idx += 1;
                }
                if idx < parts.len() { parts[idx].to_lowercase() } else { return false; }
            } else {
                parts[0].to_lowercase()
            };
            start
        };

        let path = std::path::Path::new(&main_cmd);
        if let Some(file_name) = path.file_name().and_then(|f| f.to_str()) {
            let base_name = file_name.strip_suffix(".exe").unwrap_or(file_name).to_lowercase();
            for allowed in &self.command_whitelist {
                if base_name == allowed.to_lowercase() {
                    return true;
                }
            }
        }

        false
    }

    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    pub fn read_only_whitelist(&self) -> &[PathBuf] {
        &self.read_only_whitelist
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                result.pop();
            }
            std::path::Component::CurDir => {}
            other => {
                result.push(other.as_os_str());
            }
        }
    }
    result
}