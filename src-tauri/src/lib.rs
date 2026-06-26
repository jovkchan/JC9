mod process;
mod storage;

use process::ProcessManager;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use storage::{load_projects, save_projects, load_shortcuts, save_shortcuts as save_sc, Project, Shortcut};
use tauri::State;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

struct AppState {
    manager: ProcessManager,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestCommand {
    pub name: String,
    pub command: String,
    pub working_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub name: String,
    pub language: String,
    pub suggest_commands: Vec<SuggestCommand>,
}

#[tauri::command]
fn detect_project(dir: String) -> Result<ProjectInfo, String> {
    let path = Path::new(&dir);
    if !path.is_dir() {
        return Err("目录不存在".into());
    }
    let name = path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "未命名".into());

    let mut language = String::new();
    let mut suggest_commands: Vec<SuggestCommand> = Vec::new();

    let dir_str = dir.clone();

    // Check for Go project
    if path.join("go.mod").exists() {
        language = "Go".into();
        suggest_commands.push(SuggestCommand {
            name: "go run".into(),
            command: "go run .".into(),
            working_dir: dir_str.clone(),
        });
        suggest_commands.push(SuggestCommand {
            name: "go build".into(),
            command: "go build -o bin/app.exe .".into(),
            working_dir: dir_str.clone(),
        });
        suggest_commands.push(SuggestCommand {
            name: "go mod tidy".into(),
            command: "go mod tidy".into(),
            working_dir: dir_str.clone(),
        });
    }

    // Check for Vue / Tauri project
    if path.join("package.json").exists() {
        if !language.is_empty() { language.push_str(" + "); }
        
        // Read package.json for scripts
        if let Ok(content) = fs::read_to_string(path.join("package.json")) {
            if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(scripts) = pkg["scripts"].as_object() {
                    for (script_name, _) in scripts {
                        let sname = script_name.as_str();
                        suggest_commands.push(SuggestCommand {
                            name: format!("npm {}", sname),
                            command: format!("npm run {}", sname),
                            working_dir: dir_str.clone(),
                        });
                    }
                }
                if let Some(deps) = pkg["dependencies"].as_object() {
                    if deps.contains_key("@tauri-apps/api") || deps.contains_key("vue") {
                        if !language.contains("Vue") { language.push_str("Vue"); }
                    }
                }
                if let Some(dev_deps) = pkg["devDependencies"].as_object() {
                    if dev_deps.contains_key("@tauri-apps/cli") {
                        if !language.contains("Tauri") { 
                            if !language.is_empty() { language.push_str(" + "); }
                            language.push_str("Tauri");
                        }
                        suggest_commands.push(SuggestCommand {
                            name: "tauri dev".into(),
                            command: "npx tauri dev".into(),
                            working_dir: dir_str.clone(),
                        });
                        suggest_commands.push(SuggestCommand {
                            name: "tauri build".into(),
                            command: "npx tauri build".into(),
                            working_dir: dir_str.clone(),
                        });
                    }
                }
            }
        } else {
            language.push_str("Node.js");
        }
    }

    // Check for Cargo project
    if path.join("Cargo.toml").exists() {
        if !language.is_empty() { language.push_str(" + "); }
        language.push_str("Rust");
        suggest_commands.push(SuggestCommand {
            name: "cargo build".into(),
            command: "cargo build".into(),
            working_dir: dir_str.clone(),
        });
        suggest_commands.push(SuggestCommand {
            name: "cargo run".into(),
            command: "cargo run".into(),
            working_dir: dir_str.clone(),
        });
        suggest_commands.push(SuggestCommand {
            name: "cargo check".into(),
            command: "cargo check".into(),
            working_dir: dir_str.clone(),
        });
    }

    if language.is_empty() {
        language = "未知".into();
    }

    Ok(ProjectInfo { name, language, suggest_commands })
}

#[tauri::command]
fn get_projects() -> Result<Vec<Project>, String> {
    load_projects()
}

#[tauri::command]
fn save_all_projects(projects: Vec<Project>) -> Result<(), String> {
    save_projects(&projects)
}

#[tauri::command]
fn start_command(
    state: State<'_, Mutex<AppState>>,
    app: tauri::AppHandle,
    process_id: String,
    project_id: String,
    command_id: String,
    working_dir: String,
    command: String,
) -> Result<(), String> {
    let app_state = state.lock().map_err(|e| e.to_string())?;
    app_state.manager.start(
        process_id,
        project_id,
        command_id,
        working_dir,
        command,
        app,
    )
}

#[tauri::command]
fn stop_command(
    state: State<'_, Mutex<AppState>>,
    process_id: String,
) -> Result<(), String> {
    let app_state = state.lock().map_err(|e| e.to_string())?;
    app_state.manager.stop(&process_id)
}

#[tauri::command]
fn stop_command_by_ids(
    state: State<'_, Mutex<AppState>>,
    project_id: String,
    command_id: String,
) -> Result<(), String> {
    let app_state = state.lock().map_err(|e| e.to_string())?;
    app_state
        .manager
        .stop_all_for_command(&project_id, &command_id)
}

#[tauri::command]
fn is_command_running(
    state: State<'_, Mutex<AppState>>,
    project_id: String,
    command_id: String,
) -> bool {
    let app_state = match state.lock() {
        Ok(s) => s,
        Err(_) => return false,
    };
    let process_id = format!("{}::{}", project_id, command_id);
    app_state.manager.is_running(&process_id)
}

#[tauri::command]
fn pty_write(
    state: State<'_, Mutex<AppState>>,
    process_id: String,
    data: Vec<u8>,
) -> Result<(), String> {
    let app_state = state.lock().map_err(|e| e.to_string())?;
    app_state.manager.write(&process_id, &data)
}

#[tauri::command]
fn pty_resize(
    state: State<'_, Mutex<AppState>>,
    process_id: String,
    rows: u16,
    cols: u16,
) -> Result<(), String> {
    let app_state = state.lock().map_err(|e| e.to_string())?;
    app_state.manager.resize(&process_id, rows, cols)
}

#[tauri::command]
fn kill_port(port: u16) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        // netstat -ano | findstr :PORT
        let output = Command::new("cmd")
            .args(["/C", &format!("netstat -ano | findstr :{}", port)])
            .creation_flags(0x08000000)
            .output()
            .map_err(|e| format!("执行失败: {e}"))?;

        let text = String::from_utf8_lossy(&output.stdout);
        if text.trim().is_empty() {
            return Err(format!("未找到占用端口 {} 的进程", port));
        }

        // Parse PIDs and kill
        let mut killed: Vec<String> = Vec::new();
        for line in text.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(&pid) = parts.last() {
                let pid_str = pid.to_string();
                if pid_str.chars().all(|c| c.is_ascii_digit()) && !killed.contains(&pid_str) {
                    let _ = Command::new("taskkill")
                        .args(["/F", "/PID", pid])
                        .creation_flags(0x08000000)
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .spawn();
                    killed.push(pid_str);
                }
            }
        }
        if killed.is_empty() {
            Err("未能解析PID".into())
        } else {
            Ok(format!("已杀掉 {} 个进程 (PID: {})", killed.len(), killed.join(", ")))
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        use std::process::Command;
        let output = Command::new("sh")
            .args(["-c", &format!("lsof -ti:{} | xargs kill -9", port)])
            .output()
            .map_err(|e| format!("执行失败: {e}"))?;
        let text = String::from_utf8_lossy(&output.stdout);
        if text.trim().is_empty() { Err("未找到进程".into()) }
        else { Ok(format!("已杀掉端口 {} 占用的进程", port)) }
    }
}

#[tauri::command]
fn get_shortcuts() -> Vec<Shortcut> { load_shortcuts() }

#[tauri::command]
fn save_shortcuts(shortcuts: Vec<Shortcut>) -> Result<(), String> { save_sc(&shortcuts) }

fn get_cmd_help(cmd: &str) -> String {
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &format!("{} --help 2>&1", cmd)])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output();
    match output {
        Ok(o) => {
            let s = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
            if s.trim().is_empty() || s.len() < 10 {
                let o2 = std::process::Command::new("powershell")
                    .args(["-NoProfile", "-Command", &format!("{} -h 2>&1", cmd)])
                    .stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped())
                    .output().unwrap_or_else(|_| std::process::Output { status: Default::default(), stdout: vec![], stderr: vec![] });
                let s2 = format!("{}{}", String::from_utf8_lossy(&o2.stdout), String::from_utf8_lossy(&o2.stderr));
                if s2.trim().is_empty() { format!("命令 '{}' 无可用文档", cmd) } else { s2 }
            } else { s }
        }
        Err(e) => format!("获取文档失败: {}", e),
    }
}

#[tauri::command]
fn fetch_doc(command: String) -> String { get_cmd_help(&command) }

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Mutex::new(AppState {
            manager: ProcessManager::new(),
        }))
        .invoke_handler(tauri::generate_handler![
            get_projects,
            save_all_projects,
            start_command,
            stop_command,
            stop_command_by_ids,
            is_command_running,
            detect_project,
            pty_write,
            pty_resize,
            kill_port,
            get_shortcuts,
            save_shortcuts,
            fetch_doc,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
