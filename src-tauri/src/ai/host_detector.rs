use std::collections::HashMap;
use std::process::Command;
use super::types::*;

/// 宿主环境探测器 - 环境变量脱敏与系统信息采集
pub struct HostDetector;

impl HostDetector {
    pub fn new() -> Self { Self }

    pub fn detect(&self) -> HostEnvironment {
        let os = std::env::consts::OS.to_string();
        let os_version = self.get_os_version();
        let shell = self.get_shell();
        let arch = std::env::consts::ARCH.to_string();
        let env_vars = self.get_sanitized_env_vars();
        let cli_versions = self.detect_cli_versions();
        HostEnvironment { os, os_version, shell, arch, env_vars, cli_versions }
    }

    pub fn get_sanitized_env_vars(&self) -> Vec<EnvVarEntry> {
        let sensitive_keywords = ["PASSWORD", "SECRET", "TOKEN", "KEY", "PWD", "AUTH", "CREDENTIAL", "PRIVATE", "PASS", "PASSWD", "CERT", "CERTIFICATE", "SESSION", "COOKIE", "OTP", "MFA"];
        let allowed_keys = ["PATH", "HOME", "USERPROFILE", "SHELL", "LANG", "OS", "PROCESSOR_ARCHITECTURE", "TMP", "TEMP", "USER", "USERNAME"];
        let mut vars: Vec<(String, String)> = std::env::vars().collect();
        vars.sort_by(|a, b| a.0.cmp(&b.0));
        vars.into_iter()
            .filter(|(key, _)| allowed_keys.iter().any(|&k| key.eq_ignore_ascii_case(k)))
            .map(|(key, value)| {
                let is_sensitive = sensitive_keywords.iter().any(|kw| key.to_uppercase().contains(kw))
                    || (value.contains("://") && value.contains('@'));
                EnvVarEntry { key: key.clone(), value: if is_sensitive { "******".into() } else { value }, is_sensitive }
            }).collect()
    }

    pub fn generate_system_prompt(&self, env: &HostEnvironment) -> String {
        let mut prompt = String::new();
        prompt.push_str(&format!("## 宿主环境信息\n- OS: {} {}\n- Arch: {}\n- Shell: {}\n\n## 环境变量（已脱敏）\n", env.os, env.os_version, env.arch, env.shell));
        let env_map: HashMap<&str, &str> = env.env_vars.iter().map(|e| (e.key.as_str(), e.value.as_str())).collect();
        prompt.push_str(&serde_json::to_string_pretty(&env_map).unwrap_or_default());
        prompt.push_str("\n\n## CLI 工具版本\n");
        for cli in &env.cli_versions { prompt.push_str(&format!("- {}: {}\n", cli.name, cli.version)); }
        prompt
    }

    fn get_os_version(&self) -> String {
        #[cfg(target_os = "windows")]
        { if let Ok(o) = Command::new("cmd").args(["/C", "ver"]).output() { return String::from_utf8_lossy(&o.stdout).trim().into(); } }
        #[cfg(target_os = "macos")]
        { if let Ok(o) = Command::new("sw_vers").arg("-productVersion").output() { return format!("macOS {}", String::from_utf8_lossy(&o.stdout).trim()); } }
        #[cfg(target_os = "linux")]
        { if let Ok(c) = std::fs::read_to_string("/etc/os-release") { for l in c.lines() { if l.starts_with("PRETTY_NAME=") { return l.trim_start_matches("PRETTY_NAME=").trim_matches('"').into(); } } } }
        "unknown".into()
    }

    fn get_shell(&self) -> String {
        #[cfg(target_os = "windows")] { "powershell".into() }
        #[cfg(not(target_os = "windows"))] { std::env::var("SHELL").unwrap_or_else(|_| "bash".into()) }
    }

    fn detect_cli_versions(&self) -> Vec<CliVersion> {
        let tools = ["node", "npm", "yarn", "pnpm", "git", "cargo", "rustc", "go", "python", "python3"];
        tools.iter().filter_map(|t| self.get_cli_version(t).map(|v| CliVersion { name: t.to_string(), version: v })).collect()
    }

    fn get_cli_version(&self, tool: &str) -> Option<String> {
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd").args(["/C", &format!("{} --version", tool)]).output()
        } else { Command::new(tool).arg("--version").output() };
        match output {
            Ok(o) => { let t = String::from_utf8_lossy(&o.stdout).trim().to_string(); if t.is_empty() { None } else { Some(t.lines().next().unwrap_or("").into()) } }
            Err(_) => None,
        }
    }
}