use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    pub id: String,
    pub name: String,
    pub command: String,
    pub working_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub commands: Vec<Command>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Shortcut {
    pub id: String,
    pub name: String,
    pub command: String,
    pub category: String,
    pub description: String,
}

fn get_storage_path() -> PathBuf {
    let mut path = dirs_next().unwrap_or_else(|| PathBuf::from("."));
    path.push("jc9-projects.json");
    path
}

fn dirs_next() -> Option<PathBuf> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| {
            // fallback：用 exe 所在目录
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| PathBuf::from("."))
                .to_string_lossy()
                .to_string()
        });
    Some(PathBuf::from(home).join(".jc9").join("data"))
}

pub fn load_projects() -> Result<Vec<Project>, String> {
    let path = get_storage_path();
    if !path.exists() {
        // ensure directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        // return empty
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(&content).map_err(|e| format!("解析数据失败: {}", e))
}

pub fn save_projects(projects: &[Project]) -> Result<(), String> {
    let path = get_storage_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(projects).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())
}

fn get_shortcuts_path() -> PathBuf {
    let mut path = dirs_next().unwrap_or_else(|| PathBuf::from("."));
    path.push("jc9-shortcuts.json");
    path
}

/// 从嵌入的 JSON 加载默认内置快捷命令 + 用户自定义的
pub fn load_shortcuts() -> Vec<Shortcut> {
    // 从编译时嵌入的 JSON 加载内置快捷命令
    let default_json: &str = include_str!("default-shortcuts.json");
    let mut shortcuts: Vec<Shortcut> = serde_json::from_str(default_json).unwrap_or_default();

    // 合并用户自定义的快捷命令
    let path = get_shortcuts_path();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(user) = serde_json::from_str::<Vec<Shortcut>>(&content) {
                shortcuts.extend(user);
            }
        }
    }
    shortcuts
}

pub fn save_shortcuts(shortcuts: &[Shortcut]) -> Result<(), String> {
    let path = get_shortcuts_path();
    if let Some(parent) = path.parent() { fs::create_dir_all(parent).map_err(|e| e.to_string())?; }
    let content = serde_json::to_string_pretty(shortcuts).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())
}