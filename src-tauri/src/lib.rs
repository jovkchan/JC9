mod process;
mod database;
pub mod ai;

use process::ProcessManager;
use database::{Database, Project, Shortcut, NoteGroup, Note};
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use tauri::{State, Manager, Emitter};
use chrono::{DateTime, Utc};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

struct AppState {
    manager: ProcessManager,
    db: Database,
    ai_manager: std::sync::Arc<ai::agent_manager::AgentManager>,
    mcp_server: std::sync::Arc<tokio::sync::Mutex<ai::mcp_server::McpServer>>,
    startup_logs: Mutex<Vec<serde_json::Value>>,
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
async fn fetch_url_html(url: String) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
    let html = resp.text().await.map_err(|e| e.to_string())?;
    Ok(html)
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
fn get_db_path() -> Result<String, String> {
    database::get_db_path().map(|p| p.to_string_lossy().to_string())
}

/// 诊断：返回完整数据库状态 JSON
#[allow(dead_code)]
#[tauri::command]
fn db_debug(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let app = state.lock().map_err(|e| e.to_string())?;
    let path = database::get_db_path()?;
    let path_str = path.to_string_lossy().to_string();
    let exists = path.exists();
    let file_size = if exists { fs::metadata(&path).map(|m| m.len()).unwrap_or(0) } else { 0 };

    // 逐个查询，捕获各自的错误
    let projects_count = app.db.get_projects().map(|v| v.len()).unwrap_or(0);
    let notes_count = app.db.get_notes(None::<&str>, true).map(|v| v.len()).unwrap_or(0);
    let groups_count = app.db.get_note_groups().map(|v| v.len()).unwrap_or(0);

    Ok(serde_json::json!({
        "path": path_str,
        "exists": exists,
        "file_size": file_size,
        "projects": projects_count,
        "notes": notes_count,
        "groups": groups_count
    }).to_string())
}

/// 获取启动诊断日志（Rust 端 startup 阶段记录的）
#[tauri::command]
fn get_startup_logs(state: State<'_, Mutex<AppState>>) -> Vec<serde_json::Value> {
    let app = match state.lock() {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let logs = app.startup_logs.lock().unwrap_or_else(|e| e.into_inner());
    logs.clone()
}

/// 读取 AI 配置（存为 ~/.jc9/data/ai-config.json，跨 dev/build 共享）
#[tauri::command]
fn get_ai_config() -> Result<String, String> {
    database::get_ai_config()
}

/// 保存 AI 配置到 JSON 文件
#[tauri::command]
fn save_ai_config(config: String) -> Result<(), String> {
    database::save_ai_config(&config)
}

// ── 工作流（多命令顺序执行）──

#[tauri::command]
fn get_workflows() -> Result<String, String> {
    database::get_workflows()
}

#[tauri::command]
fn save_workflows(workflows_json: String) -> Result<(), String> {
    database::save_workflows_json(&workflows_json)
}

/// 执行工作流：按顺序在终端中执行多个命令，每个等上一个完成
#[tauri::command]
async fn run_workflow(app: tauri::AppHandle, tab_id: String, steps: Vec<database::WorkflowStep>) -> Result<(), String> {
    use std::io::Read;
    use tauri::Emitter;

    let total = steps.len();
    for (i, step) in steps.iter().enumerate() {
        let step_num = i + 1;

        // ── 终端输出步骤标题 ──
        let header = format!(
            "\x1b[1;33m=== 步骤 {}/{} ===\x1b[0m\n\x1b[1;36m> {}\x1b[0m\n",
            step_num, total, step.command
        );
        app.emit("pty-output", serde_json::json!({
            "processId": tab_id,
            "data": header.as_bytes()
        })).ok();

        app.emit("workflow-event", serde_json::json!({
            "type": "step_start", "step": step_num, "total": total, "name": step.name
        })).ok();

        // PowerShell 执行（默认 shell，比 cmd.exe 路径处理更强）
        #[cfg(target_os = "windows")]
        let (shell, arg, full_cmd) = ("powershell", "-Command", format!(
            "[Console]::OutputEncoding=[Text.Encoding]::UTF8; $ErrorActionPreference='Stop'; {}",
            step.command
        ));
        #[cfg(not(target_os = "windows"))]
        let (shell, arg, full_cmd) = ("sh", "-c", step.command.clone());

        let mut cmd = std::process::Command::new(shell);
        cmd.args([arg, &full_cmd]);
        cmd.current_dir(if step.working_dir.is_empty() { "." } else { &step.working_dir });
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        cmd.env("COLUMNS", "100");   // 限制终端宽度避免 vite 按超宽列渲染
        cmd.env("FORCE_COLOR", "1");  // 保留 ANSI 颜色

        // Windows 下隐藏 CMD 窗口（仅后台执行，输出走 PTY 管道）
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }

        let mut child = cmd.spawn()
            .map_err(|e| format!("步骤 {} 启动失败: {}", step_num, e))?;

        // stdout 线程
        let app2 = app.clone();
        let tid = tab_id.clone();
        if let Some(stdout) = child.stdout.take() {
            std::thread::spawn(move || {
                let mut reader = std::io::BufReader::new(stdout);
                let mut buf = [0u8; 4096];
                while let Ok(n) = reader.read(&mut buf) {
                    if n == 0 { break; }
                    let _ = app2.emit("pty-output", serde_json::json!({
                        "processId": tid,
                        "data": &buf[..n]
                    }));
                }
            });
        }

        // stderr 线程
        let app3 = app.clone();
        let tid3 = tab_id.clone();
        if let Some(stderr) = child.stderr.take() {
            std::thread::spawn(move || {
                let mut reader = std::io::BufReader::new(stderr);
                let mut buf = [0u8; 4096];
                while let Ok(n) = reader.read(&mut buf) {
                    if n == 0 { break; }
                    let _ = app3.emit("pty-output", serde_json::json!({
                        "processId": tid3,
                        "data": &buf[..n]
                    }));
                }
            });
        }

        let status = child.wait().map_err(|e| format!("步骤 {} 执行异常: {}", step_num, e))?;

        // ── 终端输出步骤结果 ──
        if status.success() {
            let ok = format!("\x1b[1;32m[OK]\x1b[0m 步骤 {}/{}\n", step_num, total);
            app.emit("pty-output", serde_json::json!({ "processId": tab_id, "data": ok.as_bytes() })).ok();
            app.emit("workflow-event", serde_json::json!({
                "type": "step_done", "step": step_num, "total": total, "name": step.name
            })).ok();
        } else {
            let code = status.code().unwrap_or(-1);
            let fail = format!("\x1b[1;31m[FAIL]\x1b[0m 步骤 {}/{} (退出码 {}):\n", step_num, total, code);
            app.emit("pty-output", serde_json::json!({ "processId": tab_id, "data": fail.as_bytes() })).ok();
            app.emit("workflow-event", serde_json::json!({
                "type": "step_fail", "step": step_num, "total": total, "name": step.name
            })).ok();
            return Err(format!("步骤 {} 失败，退出码 {}", step_num, code));
        }
    }
    app.emit("workflow-event", serde_json::json!({
        "type": "workflow_done", "step": total, "total": total
    })).ok();
    let done = format!("\n\x1b[1;32m工作流完成\x1b[0m\n");
    app.emit("pty-output", serde_json::json!({ "processId": tab_id, "data": done.as_bytes() })).ok();
    Ok(())
}

#[tauri::command]
fn get_projects(state: State<'_, Mutex<AppState>>) -> Result<Vec<Project>, String> {
    let app_state = state.lock().map_err(|e| e.to_string())?;
    app_state.db.get_projects()
}

#[tauri::command]
fn save_all_projects(state: State<'_, Mutex<AppState>>, projects: Vec<Project>) -> Result<(), String> {
    let app_state = state.lock().map_err(|e| e.to_string())?;
    app_state.db.save_projects(&projects)
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
        let output = Command::new("netstat")
            .arg("-ano")
            .creation_flags(0x08000000)
            .output()
            .map_err(|e| format!("执行失败: {e}"))?;

        let text = String::from_utf8_lossy(&output.stdout);
        let mut killed: Vec<String> = Vec::new();

        for line in text.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 5 {
                let local_addr = parts[1];
                if let Some(pos) = local_addr.rfind(':') {
                    if let Ok(p) = local_addr[pos + 1..].parse::<u16>() {
                        if p == port {
                            if let Some(&pid) = parts.last() {
                                let pid_str = pid.to_string();
                                if pid_str.chars().all(|c| c.is_ascii_digit()) && !killed.contains(&pid_str) {
                                    let _ = Command::new("taskkill")
                                        .args(["/F", "/PID", &pid_str])
                                        .creation_flags(0x08000000)
                                        .stdout(std::process::Stdio::null())
                                        .stderr(std::process::Stdio::null())
                                        .spawn();
                                    killed.push(pid_str);
                                }
                            }
                        }
                    }
                }
            }
        }

        if killed.is_empty() {
            Err(format!("未找到占用端口 {} 的进程", port))
        } else {
            Ok(format!("已杀掉占用端口 {} 的 {} 个进程 (PID: {})", port, killed.len(), killed.join(", ")))
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
fn get_shortcuts(state: State<'_, Mutex<AppState>>) -> Vec<Shortcut> {
    let app_state = match state.lock() { Ok(s) => s, Err(_) => return Vec::new() };
    app_state.db.load_shortcuts()
}

#[tauri::command]
fn save_shortcuts(state: State<'_, Mutex<AppState>>, shortcuts: Vec<Shortcut>) -> Result<(), String> {
    let app_state = state.lock().map_err(|e| e.to_string())?;
    app_state.db.save_shortcuts(&shortcuts)
}

fn get_cmd_help(cmd: &str) -> String {
    let cmd = cmd.trim();
    if cmd.is_empty() || !cmd.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return format!("命令名称 '{}' 包含非法字符，拒绝执行帮助查询。", cmd);
    }
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

// ── Notes ──

/// 将 Note 转换为知识库条目，用于同步笔记到知识库
fn note_to_kb_entry(note: &Note) -> ai::types::KbEntry {
    use ai::types::{KbEntry, KbEntryType};
    let parse_dt = |s: &str| -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s)
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now())
    };
    KbEntry {
        id: format!("note_{}", note.id),
        title: note.title.clone(),
        content: note.content.clone(),
        entry_type: KbEntryType::ConfigNote,
        tags: note.tags.clone(),
        source_session: None,
        confidence: 0.85,  // 用户笔记可信度较高
        is_draft: false,
        created_at: parse_dt(&note.created_at),
        updated_at: parse_dt(&note.updated_at),
        embedding: None,
    }
}

#[tauri::command]
fn get_note_groups(state: State<'_, Mutex<AppState>>) -> Result<Vec<NoteGroup>, String> {
    state.lock().map_err(|e| e.to_string())?.db.get_note_groups()
}

#[tauri::command]
fn save_note_group(state: State<'_, Mutex<AppState>>, group: NoteGroup) -> Result<(), String> {
    state.lock().map_err(|e| e.to_string())?.db.save_note_group(&group)
}

#[tauri::command]
fn delete_note_group(state: State<'_, Mutex<AppState>>, id: String) -> Result<(), String> {
    state.lock().map_err(|e| e.to_string())?.db.delete_note_group(&id)
}

#[tauri::command]
fn get_notes(state: State<'_, Mutex<AppState>>, group_id: Option<String>) -> Result<Vec<Note>, String> {
    state.lock().map_err(|e| e.to_string())?.db.get_notes(group_id.as_deref(), false)
}

#[tauri::command]
async fn save_note(state: State<'_, Mutex<AppState>>, note: Note) -> Result<(), String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.db.save_note(&note)?;
        app_state.ai_manager.clone()
    };
    // 归档 → 从知识库移除；未归档 → 同步到知识库
    let kb_id = format!("note_{}", note.id);
    let kb = ai_manager.knowledge_base().clone();
    if note.is_archived {
        tokio::spawn(async move { kb.remove_entry(&kb_id).await; });
    } else {
        let entry = note_to_kb_entry(&note);
        tokio::spawn(async move { kb.add_entry(entry).await; });
    }
    Ok(())
}

#[tauri::command]
async fn delete_note(state: State<'_, Mutex<AppState>>, id: String, permanent: Option<bool>) -> Result<(), String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        if permanent.unwrap_or(false) {
            app_state.db.permanently_delete_note(&id)?;
        } else {
            app_state.db.delete_note(&id)?;
        }
        // 软删除和硬删除都清理知识库向量
        app_state.ai_manager.clone()
    };
    ai_manager.knowledge_base().remove_entry(&format!("note_{}", id)).await;
    Ok(())
}

#[tauri::command]
fn search_notes(state: State<'_, Mutex<AppState>>, query: String) -> Result<Vec<Note>, String> {
    state.lock().map_err(|e| e.to_string())?.db.search_notes(&query)
}

#[tauri::command]
fn get_note_count(state: State<'_, Mutex<AppState>>) -> Result<i32, String> {
    state.lock().map_err(|e| e.to_string())?.db.get_note_count()
}

#[tauri::command]
fn get_note_by_id(state: State<'_, Mutex<AppState>>, id: String) -> Result<Option<Note>, String> {
    state.lock().map_err(|e| e.to_string())?.db.get_note_by_id(&id)
}

#[tauri::command]
#[allow(non_snake_case)]
fn move_note(state: State<'_, Mutex<AppState>>, noteId: String, groupId: Option<String>) -> Result<(), String> {
    state.lock().map_err(|e| e.to_string())?.db.move_note_to_group(&noteId, groupId.as_deref())
}

#[tauri::command]
fn write_file_binary(path: String, data: Vec<u8>) -> Result<(), String> {
    fs::write(&path, data).map_err(|e| e.to_string())
}

#[tauri::command]
fn read_file_string(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| e.to_string())
}


#[tauri::command]
fn get_chat_storage_dir() -> Result<String, String> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map_err(|_| "无法获取用户目录".to_string())?;
    let dir = format!("{}/.jc9/aichat", home);
    Ok(dir)
}

#[tauri::command]
fn write_text_file(path: String, content: String) -> Result<(), String> {
    if let Some(parent) = std::path::Path::new(&path).parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {}", e))?;
    }
    fs::write(&path, &content).map_err(|e| format!("写入文件失败: {}", e))?;
    Ok(())
}

#[tauri::command]
fn read_text_file(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| format!("读取文件失败: {}", e))
}

#[tauri::command]
fn delete_file(path: String) -> Result<(), String> {
    if std::path::Path::new(&path).exists() {
        fs::remove_file(&path).map_err(|e| format!("删除文件失败: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
fn show_in_folder(path: String) {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let _ = Command::new("explorer")
            .args(["/select,", &path])
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let _ = Command::new("open")
            .args(["-R", &path])
            .spawn();
    }
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        if let Some(parent) = std::path::Path::new(&path).parent() {
            let _ = Command::new("xdg-open")
                .arg(parent)
                .spawn();
        }
    }
}

#[tauri::command]
fn get_env_vars() -> Vec<(String, String)> {
    let mut vars: Vec<(String, String)> = std::env::vars().collect();
    vars.sort_by(|a, b| a.0.cmp(&b.0));
    vars
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshKeyPair {
    pub private_key: String,
    pub public_key: String,
}

#[tauri::command]
fn generate_ssh_key(
    algorithm: String,
    bits: u32,
    passphrase: String,
    comment: String,
) -> Result<SshKeyPair, String> {
    use std::fs;
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let temp_dir = std::env::temp_dir();
    let key_path = temp_dir.join(format!("jc9_ssh_key_{}", timestamp));
    let key_path_str = key_path.to_string_lossy().to_string();

    let mut args = vec![
        "-t".to_string(),
        algorithm.clone(),
        "-N".to_string(),
        passphrase,
        "-f".to_string(),
        key_path_str.clone(),
    ];

    if !comment.is_empty() {
        args.push("-C".to_string());
        args.push(comment);
    }

    if algorithm == "rsa" || algorithm == "ecdsa" {
        args.push("-b".to_string());
        args.push(bits.to_string());
    }

    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = Command::new("ssh-keygen");
        c.creation_flags(0x08000000);
        c
    };
    #[cfg(not(target_os = "windows"))]
    let mut cmd = Command::new("ssh-keygen");

    let output = cmd.args(&args)
        .output()
        .map_err(|e| format!("执行 ssh-keygen 失败，请确认系统是否配置了 ssh-keygen 工具: {e}"))?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("生成失败: {}", err_msg.trim()));
    }

    let private_key = fs::read_to_string(&key_path)
        .map_err(|e| format!("读取私钥失败: {e}"))?;
    let public_key = fs::read_to_string(format!("{}.pub", key_path_str))
        .map_err(|e| format!("读取公钥失败: {e}"))?;

    let _ = fs::remove_file(&key_path);
    let _ = fs::remove_file(format!("{}.pub", key_path_str));

    Ok(SshKeyPair { private_key, public_key })
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SslCertResult {
    pub server_key: String,
    pub server_cert: String,
    // 证书链模式下的额外字段（单证书模式下为空）
    pub ca_cert: Option<String>,
    pub client_key: Option<String>,
    pub client_cert: Option<String>,
}

#[tauri::command]
fn generate_ssl_cert(
    mode: String, // "single" 还是 "chain"
    common_name: String,
    sans: Vec<String>,
    days: u32,
    algo: String,
    bits: u32,
    curve: String,
) -> Result<SslCertResult, String> {
    use std::fs;
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let temp_dir = std::env::temp_dir();

    // 临时文件定义
    let key_path = temp_dir.join(format!("jc9_ssl_key_{}.key", timestamp));
    let cert_path = temp_dir.join(format!("jc9_ssl_cert_{}.crt", timestamp));
    let cnf_path = temp_dir.join(format!("jc9_ssl_cnf_{}.cnf", timestamp));
    let csr_path = temp_dir.join(format!("jc9_ssl_csr_{}.csr", timestamp));

    let ca_key_path = temp_dir.join(format!("jc9_ssl_ca_key_{}.key", timestamp));
    let ca_cert_path = temp_dir.join(format!("jc9_ssl_ca_cert_{}.crt", timestamp));
    
    let client_key_path = temp_dir.join(format!("jc9_ssl_client_key_{}.key", timestamp));
    let client_cert_path = temp_dir.join(format!("jc9_ssl_client_cert_{}.crt", timestamp));
    let client_csr_path = temp_dir.join(format!("jc9_ssl_client_csr_{}.csr", timestamp));
    let srl_path = temp_dir.join(format!("jc9_ssl_ca_cert_{}.srl", timestamp));

    // 路径转字符串
    let key_path_str = key_path.to_string_lossy().to_string();
    let cert_path_str = cert_path.to_string_lossy().to_string();
    let cnf_path_str = cnf_path.to_string_lossy().to_string();
    let csr_path_str = csr_path.to_string_lossy().to_string();
    
    let ca_key_path_str = ca_key_path.to_string_lossy().to_string();
    let ca_cert_path_str = ca_cert_path.to_string_lossy().to_string();
    
    let client_key_path_str = client_key_path.to_string_lossy().to_string();
    let client_cert_path_str = client_cert_path.to_string_lossy().to_string();
    let client_csr_path_str = client_csr_path.to_string_lossy().to_string();

    // 自适应探测 openssl 可执行文件路径
    let mut openssl_exe = "openssl".to_string();
    let possible_paths = vec![
        "C:\\Program Files\\Git\\usr\\bin\\openssl.exe",
        "C:\\Program Files\\OpenSSL-Win64\\bin\\openssl.exe",
        "C:\\Program Files\\OpenSSL\\bin\\openssl.exe",
    ];
    for p in possible_paths {
        if std::path::Path::new(p).exists() {
            openssl_exe = p.to_string();
            break;
        }
    }

    // 准备 SAN 配置
    let cn_val = if common_name.trim().is_empty() { "localhost" } else { common_name.trim() };
    let mut cnf_content = format!(
        "[req]\ndistinguished_name = req_distinguished_name\nx509_extensions = v3_req\nprompt = no\n\n[req_distinguished_name]\nC = CN\nST = BJ\nL = BJ\nO = jc9\nCN = {}\n\n[v3_req]\nkeyUsage = keyEncipherment, dataEncipherment\nextendedKeyUsage = serverAuth\nsubjectAltName = @alt_names\n\n[alt_names]\n",
        cn_val
    );

    let mut alt_names = sans.clone();
    if alt_names.is_empty() {
        alt_names.push("localhost".to_string());
        alt_names.push("127.0.0.1".to_string());
    }
    for (idx, name) in alt_names.iter().enumerate() {
        let name_trimmed = name.trim();
        if name_trimmed.is_empty() { continue; }
        if name_trimmed.parse::<std::net::IpAddr>().is_ok() {
            cnf_content.push_str(&format!("IP.{} = {}\n", idx + 1, name_trimmed));
        } else {
            cnf_content.push_str(&format!("DNS.{} = {}\n", idx + 1, name_trimmed));
        }
    }
    fs::write(&cnf_path, cnf_content).map_err(|e| format!("写入临时配置文件失败: {e}"))?;

    #[cfg(target_os = "windows")]
    let new_cmd = || {
        let mut c = Command::new(&openssl_exe);
        c.creation_flags(0x08000000);
        c
    };
    #[cfg(not(target_os = "windows"))]
    let new_cmd = || Command::new(&openssl_exe);

    // 判断是 RSA 还是 EC 算法指令片段
    let get_key_arg = |a: &str, b: u32, cv: &str| -> String {
        if a == "rsa" { format!("rsa:{}", b) } else { format!("ec:{}", cv) }
    };
    let key_spec = get_key_arg(&algo, bits, &curve);

    let result = if mode == "chain" {
        // ==================== 链模式生成 ====================
        // 1. 生成 CA 自签名证书
        let out = new_cmd().args(&[
            "req", "-x509", "-new", "-nodes",
            "-newkey", &key_spec,
            "-keyout", &ca_key_path_str,
            "-out", &ca_cert_path_str,
            "-days", &days.to_string(),
            "-subj", "/CN=jc9-Root-CA",
            "-sha256"
        ]).output().map_err(|e| format!("生成 CA 失败: {e}"))?;
        if !out.status.success() {
            return Err(format!("生成 CA 根证书失败: {}", String::from_utf8_lossy(&out.stderr).trim()));
        }

        // 2. 生成服务器 CSR 和 私钥
        let out = new_cmd().args(&[
            "req", "-new", "-nodes",
            "-newkey", &key_spec,
            "-keyout", &key_path_str,
            "-out", &csr_path_str,
            "-subj", &format!("/CN={}", cn_val),
            "-sha256"
        ]).output().map_err(|e| format!("生成服务器 CSR 失败: {e}"))?;
        if !out.status.success() {
            return Err(format!("生成服务器 CSR 失败: {}", String::from_utf8_lossy(&out.stderr).trim()));
        }

        // 3. 用 CA 签署服务器证书并应用 SAN
        let out = new_cmd().args(&[
            "x509", "-req", "-in", &csr_path_str,
            "-CA", &ca_cert_path_str,
            "-CAkey", &ca_key_path_str,
            "-CAcreateserial",
            "-out", &cert_path_str,
            "-days", &days.to_string(),
            "-extfile", &cnf_path_str,
            "-extensions", "v3_req",
            "-sha256"
        ]).output().map_err(|e| format!("签署服务器证书失败: {e}"))?;
        if !out.status.success() {
            return Err(format!("签署服务器证书失败: {}", String::from_utf8_lossy(&out.stderr).trim()));
        }

        // 4. 生成客户端 CSR 和 私钥
        let out = new_cmd().args(&[
            "req", "-new", "-nodes",
            "-newkey", &key_spec,
            "-keyout", &client_key_path_str,
            "-out", &client_csr_path_str,
            "-subj", "/CN=jc9-client-dev",
            "-sha256"
        ]).output().map_err(|e| format!("生成客户端 CSR 失败: {e}"))?;
        if !out.status.success() {
            return Err(format!("生成客户端 CSR 失败: {}", String::from_utf8_lossy(&out.stderr).trim()));
        }

        // 5. 用 CA 签署客户端证书
        let out = new_cmd().args(&[
            "x509", "-req", "-in", &client_csr_path_str,
            "-CA", &ca_cert_path_str,
            "-CAkey", &ca_key_path_str,
            "-CAcreateserial",
            "-out", &client_cert_path_str,
            "-days", &days.to_string(),
            "-sha256"
        ]).output().map_err(|e| format!("签署客户端证书失败: {e}"))?;
        if !out.status.success() {
            return Err(format!("签署客户端证书失败: {}", String::from_utf8_lossy(&out.stderr).trim()));
        }

        // 读取结果
        let server_key = fs::read_to_string(&key_path).map_err(|e| format!("读取服务器私钥失败: {e}"))?;
        let server_cert = fs::read_to_string(&cert_path).map_err(|e| format!("读取服务器证书失败: {e}"))?;
        let ca_cert = Some(fs::read_to_string(&ca_cert_path).map_err(|e| format!("读取 CA 证书失败: {e}"))?);
        let client_key = Some(fs::read_to_string(&client_key_path).map_err(|e| format!("读取客户端私钥失败: {e}"))?);
        let client_cert = Some(fs::read_to_string(&client_cert_path).map_err(|e| format!("读取客户端证书失败: {e}"))?);

        // 清理文件
        let _ = fs::remove_file(&ca_key_path);
        let _ = fs::remove_file(&ca_cert_path);
        let _ = fs::remove_file(&client_key_path);
        let _ = fs::remove_file(&client_cert_path);
        let _ = fs::remove_file(&client_csr_path);
        let _ = fs::remove_file(&csr_path);
        let _ = fs::remove_file(&srl_path);

        SslCertResult {
            server_key,
            server_cert,
            ca_cert,
            client_key,
            client_cert,
        }
    } else {
        // ==================== 单证书模式生成 ====================
        let out = new_cmd().args(&[
            "req", "-x509", "-nodes",
            "-days", &days.to_string(),
            "-out", &cert_path_str,
            "-config", &cnf_path_str,
            "-sha256",
            "-newkey", &key_spec,
            "-keyout", &key_path_str
        ]).output().map_err(|e| format!("生成证书失败: {e}"))?;

        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr).to_string();
            return Err(format!("生成失败(openssl): {}", err.trim()));
        }

        let server_key = fs::read_to_string(&key_path).map_err(|e| format!("读取私钥失败: {e}"))?;
        let server_cert = fs::read_to_string(&cert_path).map_err(|e| format!("读取证书失败: {e}"))?;

        SslCertResult {
            server_key,
            server_cert,
            ca_cert: None,
            client_key: None,
            client_cert: None,
        }
    };

    // 清理基础临时文件
    let _ = fs::remove_file(&key_path);
    let _ = fs::remove_file(&cert_path);
    let _ = fs::remove_file(&cnf_path);

    Ok(result)
}

#[tauri::command]
fn set_env_var(key: String, value: String) -> Result<(), String> {
    let key_trimmed = key.trim();
    if key_trimmed.is_empty() {
        return Err("变量名 (Key) 不能为空".into());
    }
    std::env::set_var(key_trimmed, value);
    Ok(())
}

#[tauri::command]
fn remove_env_var(key: String) -> Result<(), String> {
    let key_trimmed = key.trim();
    if key_trimmed.is_empty() {
        return Err("变量名 (Key) 不能为空".into());
    }
    std::env::remove_var(key_trimmed);
    Ok(())
}

// ══════════════════════════════════════════════════════════════
// AI Agent 相关的 Tauri 命令
// ══════════════════════════════════════════════════════════════

#[tauri::command]
async fn ai_list_sessions(state: State<'_, Mutex<AppState>>) -> Result<Vec<ai::types::AiSession>, String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.ai_manager.clone()
    };
    Ok(ai_manager.get_sessions().await)
}

#[tauri::command]
async fn ai_create_session(state: State<'_, Mutex<AppState>>, title: String) -> Result<String, String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.ai_manager.clone()
    };
    Ok(ai_manager.create_session(title).await)
}

#[tauri::command]
async fn ai_delete_session(state: State<'_, Mutex<AppState>>, session_id: String) -> Result<bool, String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.ai_manager.clone()
    };
    Ok(ai_manager.delete_session(&session_id).await)
}

#[tauri::command]
async fn ai_plan_task(
    state: State<'_, Mutex<AppState>>,
    session_id: String,
    request: String,
) -> Result<Vec<ai::types::TaskNode>, String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.ai_manager.clone()
    };
    Ok(ai_manager.plan_task(session_id, request).await)
}

#[tauri::command]
async fn ai_spawn_worker(
    state: State<'_, Mutex<AppState>>,
    session_id: String,
    task: ai::types::TaskNode,
    system_prompt: String,
) -> Result<String, String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.ai_manager.clone()
    };
    let result = ai_manager.worker_manager().read().await.spawn_worker(session_id, task, system_prompt).await;
    result
}

#[tauri::command]
async fn ai_register_frontend_tool(
    state: State<'_, Mutex<AppState>>,
    name: String,
    description: String,
    parameters: serde_json::Value,
) -> Result<bool, String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.ai_manager.clone()
    };
    let definition = ai::tools::ToolDefinition {
        name,
        description,
        parameters,
        risk_level: ai::types::RiskLevel::Low,
    };
    ai_manager.worker_manager().read().await.register_frontend_tool(definition).await;
    Ok(true)
}

#[tauri::command]
async fn ai_submit_frontend_tool_result(
    call_id: String,
    success: bool,
    output: String,
    error: Option<String>,
) -> Result<bool, String> {
    if let Some(tx) = ai::frontend_tool::pending_calls().write().await.remove(&call_id) {
        let res = ai::tools::ToolResult {
            success,
            output,
            error,
        };
        let _ = tx.send(res);
        Ok(true)
    } else {
        Err("找不到对应的调用 ID，可能已超时".into())
    }
}

#[tauri::command]
async fn ai_list_workers(state: State<'_, Mutex<AppState>>) -> Result<Vec<ai::types::WorkerState>, String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.ai_manager.clone()
    };
    let result = ai_manager.worker_manager().read().await.list_workers().await;
    Ok(result)
}

#[tauri::command]
async fn ai_kill_worker(state: State<'_, Mutex<AppState>>, worker_id: String) -> Result<bool, String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.ai_manager.clone()
    };
    let result = ai_manager.worker_manager().read().await.kill_worker(&worker_id).await;
    Ok(result)
}

#[tauri::command]
async fn ai_get_pending_approvals(state: State<'_, Mutex<AppState>>) -> Result<Vec<ai::types::ApprovalRequest>, String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.ai_manager.clone()
    };
    Ok(ai_manager.approval_queue().get_pending().await)
}

#[tauri::command]
async fn ai_approve_request(state: State<'_, Mutex<AppState>>, request_id: String) -> Result<bool, String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.ai_manager.clone()
    };
    Ok(ai_manager.approval_queue().approve(&request_id).await)
}

#[tauri::command]
async fn ai_deny_request(state: State<'_, Mutex<AppState>>, request_id: String) -> Result<bool, String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.ai_manager.clone()
    };
    Ok(ai_manager.approval_queue().deny(&request_id).await)
}

#[tauri::command]
async fn ai_deny_all_approvals(state: State<'_, Mutex<AppState>>) -> Result<(), String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.ai_manager.clone()
    };
    ai_manager.approval_queue().deny_all().await;
    Ok(())
}

#[tauri::command]
async fn ai_search_knowledge(
    state: State<'_, Mutex<AppState>>,
    query: String,
    limit: usize,
) -> Result<Vec<ai::types::KbEntry>, String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.ai_manager.clone()
    };
    Ok(ai_manager.knowledge_base().search(&query, limit).await)
}

/// 向量语义搜索
#[tauri::command]
async fn ai_semantic_search(
    state: State<'_, Mutex<AppState>>,
    query: String,
    limit: usize,
) -> Result<Vec<serde_json::Value>, String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.ai_manager.clone()
    };
    let results = ai_manager.knowledge_base().semantic_search(&query, limit).await;
    Ok(results.into_iter().map(|(id, score, content)| {
        serde_json::json!({ "id": id, "score": score, "content": content })
    }).collect())
}

/// 检查 sqlite-vec 扩展是否已加载
#[tauri::command]
async fn ai_vec_status(state: State<'_, Mutex<AppState>>) -> Result<bool, String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.ai_manager.clone()
    };
    Ok(ai_manager.knowledge_base().using_sqlite_vec())
}

#[tauri::command]
async fn ai_add_knowledge(
    state: State<'_, Mutex<AppState>>,
    entry: ai::types::KbEntry,
) -> Result<String, String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.ai_manager.clone()
    };
    Ok(ai_manager.knowledge_base().add_entry(entry).await)
}

#[tauri::command]
async fn ai_connect_mcp_server(
    state: State<'_, Mutex<AppState>>,
    name: String,
    url: String,
) -> Result<(), String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.ai_manager.clone()
    };
    ai_manager.mcp_client().connect(name, url).await
}

#[tauri::command]
async fn ai_list_mcp_servers(
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<ai::mcp_client::McpServerInfo>, String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.ai_manager.clone()
    };
    Ok(ai_manager.mcp_client().list_servers().await)
}

#[tauri::command]
async fn ai_connect_mcp_stdio(
    state: State<'_, Mutex<AppState>>,
    name: String,
    command: String,
    args: Vec<String>,
) -> Result<(), String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.ai_manager.clone()
    };
    ai_manager.mcp_client().connect_stdio(name, command, args).await
}

#[tauri::command]
async fn ai_disconnect_mcp_server(
    state: State<'_, Mutex<AppState>>,
    name: String,
) -> Result<(), String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.ai_manager.clone()
    };
    ai_manager.mcp_client().disconnect(&name).await;
    Ok(())
}

/// 保存 MCP 服务器配置到数据库
#[tauri::command]
async fn ai_save_mcp_server_config(
    state: State<'_, Mutex<AppState>>,
    id: String,
    name: String,
    transport: String,
    url: Option<String>,
    command: Option<String>,
    args: Option<String>,
) -> Result<(), String> {
    let app_state = state.lock().map_err(|e| e.to_string())?;
    app_state.db.save_mcp_server(&id, &name, &transport, url.as_deref(), command.as_deref(), args.as_deref())
}

/// 删除 MCP 服务器配置
#[tauri::command]
async fn ai_delete_mcp_server_config(
    state: State<'_, Mutex<AppState>>,
    id: String,
) -> Result<(), String> {
    let app_state = state.lock().map_err(|e| e.to_string())?;
    app_state.db.delete_mcp_server(&id)
}

/// 列出所有已保存的 MCP 服务器配置
#[tauri::command]
async fn ai_list_mcp_server_configs(
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<(String, String, String, Option<String>, Option<String>, Option<String>, bool)>, String> {
    let app_state = state.lock().map_err(|e| e.to_string())?;
    app_state.db.list_mcp_servers()
}

/// 重启时自动重连所有已启用的 MCP 服务器
#[tauri::command]
async fn ai_reconnect_mcp_servers(
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<String>, String> {
    let (db, ai_manager) = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        (app_state.db.clone(), app_state.ai_manager.clone())
    };
    let rows: Vec<(String, String, String, Option<String>, Option<String>, Option<String>)> = 
        db.get_enabled_mcp_servers().map_err(|e: String| e)?;
    let mut results: Vec<String> = Vec::new();
    for (_id, name, transport, url, command, args) in &rows {
        match transport.as_str() {
            "sse" => {
                if let Some(u) = url {
                    match ai_manager.mcp_client().connect(name.clone(), u.clone()).await {
                        Ok(()) => results.push(format!("{}: 已连接", name)),
                        Err(e) => results.push(format!("{}: 连接失败 - {}", name, e)),
                    }
                }
            }
            "stdio" => {
                if let (Some(cmd), Some(args_str)) = (command, args) {
                    let arg_list: Vec<String> = serde_json::from_str(args_str).unwrap_or_default();
                    match ai_manager.mcp_client().connect_stdio(name.clone(), cmd.clone(), arg_list).await {
                        Ok(()) => results.push(format!("{}: 已连接", name)),
                        Err(e) => results.push(format!("{}: 连接失败 - {}", name, e)),
                    }
                }
            }
            _ => results.push(format!("{}: 未知传输类型 {}", name, transport)),
        }
    }
    Ok(results)
}

// ══════════════════════════════════════════════════════════════
// JC9 MCP Server 管理命令（让其他 AI Agent 连接）
// ══════════════════════════════════════════════════════════════

/// 获取 MCP Server 配置
#[tauri::command]
async fn ai_get_mcp_server_config(state: State<'_, Mutex<AppState>>) -> Result<ai::mcp_server::McpServerConfig, String> {
    let mcp_server = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.mcp_server.clone()
    };
    let server = mcp_server.lock().await;
    Ok(server.get_config().await)
}

/// 更新并应用 MCP Server 配置
#[tauri::command]
async fn ai_set_mcp_server_config(
    state: State<'_, Mutex<AppState>>,
    config: ai::mcp_server::McpServerConfig,
) -> Result<String, String> {
    let mcp_server = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        (app_state.mcp_server.clone(), app_state.db.conn.clone())
    };
    let (mcp_server, db_conn) = mcp_server;
    let mut server = mcp_server.lock().await;
    let was_enabled = server.get_config().await.enabled;
    server.update_config(config.clone()).await;

    // 持久化到数据库
    ai::mcp_config::save_mcp_config(&db_conn, &config)
        .map_err(|e| format!("保存配置失败: {}", e))?;

    // 根据配置变更自动启停
    if config.enabled && !was_enabled {
        server.start().await?;
        Ok(format!("✅ MCP Server 已启动 (端口 {})", config.port))
    } else if !config.enabled && was_enabled {
        server.stop().await;
        Ok("✅ MCP Server 已停止".into())
    } else if config.enabled {
        server.restart().await?;
        Ok(format!("✅ MCP Server 已重启 (端口 {})", config.port))
    } else {
        Ok("✅ 配置已保存".into())
    }
}

/// 启动 MCP Server
#[tauri::command]
async fn ai_start_mcp_server(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let (mcp_server, db_conn) = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        (app_state.mcp_server.clone(), app_state.db.conn.clone())
    };
    let mut server = mcp_server.lock().await;
    let mut config = server.get_config().await;
    config.enabled = true;
    server.update_config(config.clone()).await;
    server.start().await?;
    ai::mcp_config::save_mcp_config(&db_conn, &config)
        .map_err(|e| format!("保存配置失败: {}", e))?;
    Ok(format!("✅ MCP Server 已启动 (端口 {})", config.port))
}

/// 停止 MCP Server
#[tauri::command]
async fn ai_stop_mcp_server(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let (mcp_server, db_conn) = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        (app_state.mcp_server.clone(), app_state.db.conn.clone())
    };
    let mut server = mcp_server.lock().await;
    let mut config = server.get_config().await;
    config.enabled = false;
    server.update_config(config.clone()).await;
    server.stop().await;
    ai::mcp_config::save_mcp_config(&db_conn, &config)
        .map_err(|e| format!("保存配置失败: {}", e))?;
    Ok("✅ MCP Server 已停止".into())
}

/// 重建全部知识条目向量嵌入（同步写入 embeddings + vec_embeddings）
#[tauri::command]
async fn ai_reindex_knowledge(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let kb = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.ai_manager.knowledge_base().clone()
    };
    match kb.reindex_all().await {
        Ok(n) => Ok(format!("✅ 重建完成: {} 条向量已生成到 embeddings 和 vec_embeddings", n)),
        Err(e) => Err(format!("重建失败: {}", e)),
    }
}

/// 获取 MCP Server 运行状态
#[tauri::command]
async fn ai_get_mcp_server_status(state: State<'_, Mutex<AppState>>) -> Result<serde_json::Value, String> {
    let mcp_server = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.mcp_server.clone()
    };
    let server = mcp_server.lock().await;
    let config = server.get_config().await;
    Ok(serde_json::json!({
        "running": server.is_running(),
        "enabled": config.enabled,
        "port": config.port,
        "host": config.host,
    }))
}

#[tauri::command]
async fn ai_list_drafts(state: State<'_, Mutex<AppState>>) -> Result<Vec<ai::types::KbEntry>, String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.ai_manager.clone()
    };
    Ok(ai_manager.knowledge_base().list_drafts().await)
}

/// 查询会话追踪事件链
#[tauri::command]
async fn ai_get_trace_events(state: State<'_, Mutex<AppState>>, session_id: String) -> Result<Vec<ai::tracer::TraceEvent>, String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.ai_manager.clone()
    };
    Ok(ai_manager.tracer().get_session_events(&session_id).await)
}

/// 按事件类型过滤查询
#[tauri::command]
async fn ai_get_trace_events_by_type(state: State<'_, Mutex<AppState>>, session_id: String, event_type: String) -> Result<Vec<ai::tracer::TraceEvent>, String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.ai_manager.clone()
    };
    Ok(ai_manager.tracer().get_events_by_type(&session_id, &event_type).await)
}

/// 获取最近的 N 条追踪事件（全局）
#[tauri::command]
async fn ai_get_recent_trace_events(state: State<'_, Mutex<AppState>>, limit: usize) -> Result<Vec<ai::tracer::TraceEvent>, String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.ai_manager.clone()
    };
    Ok(ai_manager.tracer().get_recent_events(limit).await)
}

/// 打开浏览器窗口（手动入口）
#[tauri::command]
async fn ai_browser_navigate(state: State<'_, Mutex<AppState>>, url: String) -> Result<String, String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.ai_manager.clone()
    };
    ai_manager.browser_manager().navigate(&url).await
}

#[tauri::command]
async fn ai_promote_knowledge(state: State<'_, Mutex<AppState>>, entry_id: String) -> Result<bool, String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.ai_manager.clone()
    };
    Ok(ai_manager.knowledge_base().promote(&entry_id).await)
}

#[tauri::command]
async fn ai_update_cost_config(
    state: State<'_, Mutex<AppState>>,
    config: ai::types::CostConfig,
) -> Result<(), String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.ai_manager.clone()
    };
    let mut current_config = ai_manager.cost_config().write().await;
    *current_config = config;
    Ok(())
}

/// 运行时设置 DS 思维强度
#[tauri::command]
async fn ai_set_reasoning_effort(
    state: State<'_, Mutex<AppState>>,
    effort: String,
) -> Result<(), String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.ai_manager.clone()
    };
    ai_manager.set_reasoning_effort(effort).await;
    Ok(())
}

/// Agent 模式激活时，从配置注入 LLM Provider
#[tauri::command]
async fn ai_configure_llm(
    state: State<'_, Mutex<AppState>>,
    provider: String,
    #[allow(non_snake_case)]
    apiKey: String,
    #[allow(non_snake_case)]
    baseUrl: String,
    model: String,
) -> Result<(), String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.ai_manager.clone()
    };
    ai_manager.reconfigure_llm(&provider, &apiKey, &baseUrl, &model).await;
    Ok(())
}

#[tauri::command]
async fn ai_get_workspace_root(
    state: State<'_, Mutex<AppState>>,
) -> Result<String, String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.ai_manager.clone()
    };
    let root = ai_manager.workspace_root().read().await.clone();
    Ok(root.to_string_lossy().to_string())
}

#[tauri::command]
async fn ai_update_workspace_root(
    state: State<'_, Mutex<AppState>>,
    new_path: String,
) -> Result<(), String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.ai_manager.clone()
    };
    ai_manager.update_workspace_root(std::path::PathBuf::from(new_path)).await;
    Ok(())
}

#[tauri::command]
async fn ai_select_workspace_dialog(
    app: tauri::AppHandle,
    state: State<'_, Mutex<AppState>>,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = tokio::sync::oneshot::channel::<Option<String>>();
    
    app.dialog()
        .file()
        .pick_folder(move |folder| {
            let path_str = folder.and_then(|f| {
                match f {
                    tauri_plugin_dialog::FilePath::Path(p) => Some(p.to_string_lossy().to_string()),
                    tauri_plugin_dialog::FilePath::Url(u) => u.to_file_path().ok().map(|p| p.to_string_lossy().to_string()),
                }
            });
            let _ = tx.send(path_str);
        });

    let selected_path = rx.await.map_err(|e| e.to_string())?;
    
    if let Some(ref path) = selected_path {
        let ai_manager = {
            let app_state = state.lock().map_err(|e| e.to_string())?;
            app_state.ai_manager.clone()
        };
        ai_manager.update_workspace_root(std::path::PathBuf::from(path)).await;
    }

    Ok(selected_path)
}

/// 技能信息（全局 ~/.agents/skills/ + 项目 .jc9/skills/）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSkillInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub path: String,
    pub file_size: u64,
    pub enabled: bool,
    /// "system" = 全局 ~/.agents/skills/，"project" = 项目 .jc9/skills/
    pub source: String,
}

#[tauri::command]
fn list_system_skills(workspace_root: String) -> Result<Vec<SystemSkillInfo>, String> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."));

    let mut skills = Vec::new();

    // 1. 全局技能：~/.agents/skills/
    let global_dir = home.join(".agents").join("skills");
    scan_skills_dir(&global_dir, "system", &mut skills);

    // 2. 项目技能：<workspace>/.jc9/skills/
    let project_dir = std::path::PathBuf::from(&workspace_root).join(".jc9").join("skills");
    scan_skills_dir(&project_dir, "project", &mut skills);

    skills.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(skills)
}

/// 扫描指定目录下的技能
fn scan_skills_dir(dir: &std::path::Path, source: &str, skills: &mut Vec<SystemSkillInfo>) {
    if !dir.exists() || !dir.is_dir() {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let skill_md = path.join("SKILL.md");
            if !skill_md.exists() {
                continue;
            }
            let content = match std::fs::read_to_string(&skill_md) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let (name, version, description, fm_enabled) = parse_skill_frontmatter(&content);

            let dir_name = path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            let file_size = std::fs::metadata(&skill_md)
                .map(|m| m.len())
                .unwrap_or(0);

            let disabled_marker = path.join(".disabled");
            let enabled = fm_enabled && !disabled_marker.exists();

            skills.push(SystemSkillInfo {
                id: dir_name.clone(),
                name: if name.is_empty() { dir_name } else { name },
                version,
                description,
                path: path.to_string_lossy().to_string(),
                file_size,
                enabled,
                source: source.to_string(),
            });
        }
    }
}

/// 解析 SKILL.md 的 YAML frontmatter，提取 name、version、description 和 enabled 状态
fn parse_skill_frontmatter(content: &str) -> (String, String, String, bool) {
    let text = content.trim_start();
    if !text.starts_with("---") {
        return (String::new(), String::new(), String::new(), true);
    }
    let after_first = &text[3..];
    if let Some(end) = after_first.find("\n---") {
        let fm = &after_first[..end];
        let mut name = String::new();
        let mut version = String::new();
        let mut description = String::new();
        let mut enabled = true;
        for line in fm.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("name:") {
                name = value.trim().trim_matches('"').to_string();
            } else if let Some(value) = trimmed.strip_prefix("version:") {
                version = value.trim().trim_matches('"').to_string();
            } else if let Some(value) = trimmed.strip_prefix("description:") {
                description = value.trim().trim_matches('"').to_string();
                if description == ">" || description == "|" {
                    description = String::new();
                }
            } else if let Some(value) = trimmed.strip_prefix("enabled:") {
                let v = value.trim().to_lowercase();
                enabled = v != "false" && v != "0" && v != "no";
            }
        }
        (name, version, description, enabled)
    } else {
        (String::new(), String::new(), String::new(), true)
    }
}

/// 后端代理 AI 请求：绕过前端 CSP/CORS 限制
#[tauri::command]
async fn proxy_ai_request(url: String, method: String, headers: Vec<(String, String)>, body: Option<String>) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

    let mut req = match method.to_uppercase().as_str() {
        "POST" => client.post(&url),
        "GET" => client.get(&url),
        "PUT" => client.put(&url),
        "DELETE" => client.delete(&url),
        _ => client.post(&url),
    };

    for (key, value) in &headers {
        req = req.header(key.as_str(), value.as_str());
    }

    if let Some(b) = body {
        req = req.body(b);
    }

    let resp = req.send().await.map_err(|e| format!("请求失败: {e}"))?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| format!("读取响应失败: {e}"))?;

    if status.is_success() || status.is_redirection() {
        Ok(text)
    } else {
        Err(format!("请求失败 ({status}): {text}"))
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db = Database::new().expect("无法初始化数据库");

    // 工作区路径：无论 dev 还是 build 模式，统一使用用户主目录
    // 确保所有数据（数据库、AI对话记录、配置、技能等）都存放在 ~/.jc9 下
    let workspace = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
        });
    let db_conn = db.conn.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            // 主窗口关闭时彻底退出（防止后台线程残留）
            if let Some(win) = app.get_webview_window("main") {
                let w = win.clone();
                win.on_window_event(move |ev| {
                    if let tauri::WindowEvent::CloseRequested { .. } = ev {
                        let _ = w.destroy();
                        std::process::exit(0);
                    }
                });
            }
            let ai_manager = std::sync::Arc::new(ai::agent_manager::AgentManager::new(
                workspace.clone(),
                db_conn.clone(),
                Some(app.handle().clone()),
            ));
            // ── MCP Server ──
            let mcp_server = std::sync::Arc::new(tokio::sync::Mutex::new(
                ai::mcp_server::McpServer::new()
            ));

            app.manage(Mutex::new(AppState {
                manager: ProcessManager::new(),
                db,
                ai_manager: ai_manager.clone(),
                mcp_server,
                startup_logs: Mutex::new(Vec::new()),
            }));

            // ── 将知识库注入 MCP Server（后台异步启动）──
            {
                let state = app.state::<Mutex<AppState>>();
                let guard = state.lock().unwrap();
                let server_clone = guard.mcp_server.clone();
                let kb = ai_manager.knowledge_base().clone();
                let db_clone = guard.db.conn.clone();
                drop(guard);

                tauri::async_runtime::spawn(async move {
                    let mut server = server_clone.lock().await;
                    server.set_knowledge_base(kb);
                    server.set_db_conn(db_clone.clone());

                    // 从数据库读取配置
                    let saved_config = ai::mcp_config::load_mcp_config(&db_clone);
                    match saved_config {
                        Ok(Some(config)) => {
                            server.update_config(config).await;
                            if server.get_config().await.enabled {
                                match server.start().await {
                                    Ok(()) => println!("🧠 JC9 MCP Server 已自动启动 (端口 {})", server.get_config().await.port),
                                    Err(e) => println!("⚠️  MCP Server 启动失败: {}", e),
                                }
                            } else {
                                println!("🧠 MCP Server 未启用（可在设置中开启）");
                            }
                        }
                        Ok(None) => {
                            // 首次：持久化默认配置
                            let default_config = server.get_config().await;
                            let _ = ai::mcp_config::save_mcp_config(&db_clone, &default_config);
                            println!("🧠 MCP Server 默认配置已生成（未启用）");
                        }
                        Err(e) => {
                            println!("❌ 读取 MCP Server 配置失败: {}", e);
                        }
                    }
                });
            }

            // ── 启动诊断：逐条记录到 AppState.startup_logs ──
            let state = app.state::<Mutex<AppState>>();
            let guard = state.lock().unwrap();

            macro_rules! log_startup {
                ($step:expr, $level:expr, $msg:expr) => {
                    let entry = serde_json::json!({
                        "step": $step,
                        "message": $msg,
                        "level": $level
                    });
                    if let Ok(mut logs) = guard.startup_logs.lock() {
                        logs.push(entry);
                    }
                };
            }

            // 1. 数据库路径
            let path_str = database::get_db_path()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| "未知".into());
            let file_size = std::path::Path::new(&path_str)
                .metadata()
                .map(|m| m.len())
                .unwrap_or(0);
            log_startup!("db_path", "info", format!("📁 数据库路径: {} ({}KB)", path_str, file_size / 1024));

            // 2. 查询项目
            match guard.db.get_projects() {
                Ok(projects) => {
                    log_startup!("projects",
                        if projects.is_empty() { "warn" } else { "success" },
                        format!("📦 项目查询: {} 个项目", projects.len()));
                    // 额外推送带 count 的
                    if let Ok(mut logs) = guard.startup_logs.lock() {
                        if let Some(last) = logs.last_mut() {
                            if let Some(obj) = last.as_object_mut() {
                                obj.insert("count".into(), serde_json::json!(projects.len()));
                            }
                        }
                    }
                }
                Err(e) => {
                    log_startup!("projects", "error", format!("❌ 项目查询失败: {}", e));
                }
            }

            // 3. 查询笔记分组
            match guard.db.get_note_groups() {
                Ok(groups) => {
                    log_startup!("groups",
                        if groups.is_empty() { "warn" } else { "success" },
                        format!("📂 分组查询: {} 个分组", groups.len()));
                    if let Ok(mut logs) = guard.startup_logs.lock() {
                        if let Some(last) = logs.last_mut() {
                            if let Some(obj) = last.as_object_mut() {
                                obj.insert("count".into(), serde_json::json!(groups.len()));
                            }
                        }
                    }
                }
                Err(e) => {
                    log_startup!("groups", "error", format!("❌ 分组查询失败: {}", e));
                }
            }

            // 4. 查询笔记
            match guard.db.get_notes(None::<&str>, true) {
                Ok(notes) => {
                    let active = notes.iter().filter(|n| !n.is_deleted).count();
                    log_startup!("notes",
                        if notes.is_empty() { "warn" } else { "success" },
                        format!("📝 笔记查询: 共 {} 条 (活跃 {} 条)", notes.len(), active));
                    if let Ok(mut logs) = guard.startup_logs.lock() {
                        if let Some(last) = logs.last_mut() {
                            if let Some(obj) = last.as_object_mut() {
                                obj.insert("count".into(), serde_json::json!(notes.len()));
                            }
                        }
                    }
                }
                Err(e) => {
                    log_startup!("notes", "error", format!("❌ 笔记查询失败: {}", e));
                }
            }

            // 5. 直接 SQL 查询 projects 表原始数据（诊断 build 模式为何返回空）
            {
                let conn = guard.db.conn.lock().map_err(|e| e.to_string());
                if let Ok(ref conn) = conn {
                    // 检查 projects 表行数
                    let project_count: i64 = conn.query_row(
                        "SELECT COUNT(*) FROM projects WHERE user_id = 'local'",
                        [],
                        |row| row.get(0)
                    ).unwrap_or(-1);
                    log_startup!("db_raw_count", "info",
                        format!("🔍 SQL COUNT(projects WHERE user_id='local') = {}", project_count));

                    // 列出 projects 表所有行
                    let db_raw_projects_result: Result<Vec<Vec<String>>, String> = (|| {
                        let mut stmt = conn.prepare("SELECT id, name, user_id, created_at FROM projects LIMIT 10")
                            .map_err(|e| e.to_string())?;
                        let rows = stmt.query_map([], |row| {
                            Ok(vec![
                                row.get::<_, String>(0).unwrap_or_default(),
                                row.get::<_, String>(1).unwrap_or_default(),
                                row.get::<_, String>(2).unwrap_or_default(),
                                row.get::<_, String>(3).unwrap_or_default(),
                            ])
                        }).map_err(|e| e.to_string())?;
                        let collected: Vec<Vec<String>> = rows.filter_map(|r| r.ok()).collect();
                        Ok(collected)
                    })();
                    match db_raw_projects_result {
                        Ok(rows) => {
                            let log_entry = serde_json::json!({
                                "step": "db_raw_projects",
                                "message": format!("🔍 projects 表数据: {} 行", rows.len()),
                                "level": "info",
                                "rows": rows
                            });
                            if let Ok(mut logs) = guard.startup_logs.lock() {
                                logs.push(log_entry);
                            }
                        }
                        Err(e) => {
                            log_startup!("db_raw_projects", "error", format!("❌ projects 表查询失败: {}", e));
                        }
                    }

                    // 列出 note_groups 表
                    let group_count: i64 = conn.query_row(
                        "SELECT COUNT(*) FROM note_groups WHERE user_id = 'local'",
                        [],
                        |row| row.get(0)
                    ).unwrap_or(-1);
                    log_startup!("db_raw_groups", "info",
                        format!("🔍 SQL COUNT(note_groups WHERE user_id='local') = {}", group_count));

                    // 检查 user 表
                    let user_count: i64 = conn.query_row(
                        "SELECT COUNT(*) FROM users",
                        [],
                        |row| row.get(0)
                    ).unwrap_or(-1);
                    log_startup!("db_users", "info",
                        format!("🔍 users 表行数: {}", user_count));
                } else {
                    log_startup!("db_raw_query", "error", "无法获取数据库连接锁");
                }
            }

            log_startup!("complete", "success", "✅ Rust 端启动诊断完成");
            drop(guard);

            // 启动时同步技能 + 自动重连 MCP 服务器（带事件通知）
            let app_handle2 = app.handle().clone();
            let db_conn_clone = db_conn.clone();
            let skills_dir = workspace.join(".jc9");
            tauri::async_runtime::spawn(async move {
                // 1. 同步技能文件到知识库
                let loader = ai::skill_loader::SkillLoader::new(skills_dir, db_conn_clone);
                let skill_count = loader.sync_all().await;
                if skill_count > 0 {
                    let _ = app_handle2.emit("startup-log", serde_json::json!({
                        "step": "skills",
                        "message": format!("🧠 已同步 {} 个技能到知识库", skill_count),
                        "level": "success"
                    }));
                }

                // 2. 自动重连已保存的 MCP 服务器
                let state = app_handle2.state::<Mutex<AppState>>();
                let (mcp_configs, mcp_client) = {
                    let guard = state.lock().unwrap();
                    let configs = guard.db.get_enabled_mcp_servers().unwrap_or_default();
                    let client = guard.ai_manager.mcp_client();
                    (configs, client.clone())
                };
                for (_, name, transport, url, command, args) in &mcp_configs {
                    match transport.as_str() {
                        "sse" => {
                            if let Some(u) = url {
                                match mcp_client.connect(name.clone(), u.clone()).await {
                                    Ok(()) => {
                                        let _ = app_handle2.emit("startup-log", serde_json::json!({
                                            "step": "mcp",
                                            "message": format!("🔗 MCP [{}] SSE 已自动重连", name),
                                            "level": "success"
                                        }));
                                    }
                                    Err(e) => {
                                        let _ = app_handle2.emit("startup-log", serde_json::json!({
                                            "step": "mcp",
                                            "message": format!("⚠️ MCP [{}] 自动重连失败: {}", name, e),
                                            "level": "warn"
                                        }));
                                    }
                                }
                            }
                        }
                        "stdio" => {
                            if let (Some(cmd), Some(args_str)) = (command, args) {
                                let arg_list: Vec<String> = serde_json::from_str(args_str).unwrap_or_default();
                                match mcp_client.connect_stdio(name.clone(), cmd.clone(), arg_list).await {
                                    Ok(()) => {
                                        let _ = app_handle2.emit("startup-log", serde_json::json!({
                                            "step": "mcp",
                                            "message": format!("🔗 MCP [{}] stdio 已自动重连", name),
                                            "level": "success"
                                        }));
                                    }
                                    Err(e) => {
                                        let _ = app_handle2.emit("startup-log", serde_json::json!({
                                            "step": "mcp",
                                            "message": format!("⚠️ MCP [{}] 自动重连失败: {}", name, e),
                                            "level": "warn"
                                        }));
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                if !mcp_configs.is_empty() {
                    let _ = app_handle2.emit("startup-log", serde_json::json!({
                        "step": "mcp_done",
                        "message": format!("✅ MCP 服务器自动重连完成 ({} 个)", mcp_configs.len()),
                        "level": "success"
                    }));
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_db_path,
            db_debug,
            get_startup_logs,
            get_ai_config,
            save_ai_config,
            get_workflows,
            save_workflows,
            run_workflow,
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
            get_env_vars,
            generate_ssh_key,
            generate_ssl_cert,
            set_env_var,
            remove_env_var,
            write_file_binary,
            read_file_string,
            show_in_folder,
            get_chat_storage_dir,
            write_text_file,
            read_text_file,
            delete_file,
            fetch_url_html,
            get_note_groups,
            save_note_group,
            delete_note_group,
            get_notes,
            save_note,
            delete_note,
            search_notes,
            get_note_count,
            get_note_by_id,
            move_note,
            // AI commands
            ai_list_sessions,
            ai_create_session,
            ai_delete_session,
            ai_plan_task,
            ai_spawn_worker,
            ai_list_workers,
            ai_kill_worker,
            ai_get_pending_approvals,
            ai_approve_request,
            ai_deny_request,
            ai_deny_all_approvals,
            ai_search_knowledge,
            ai_semantic_search,
            ai_vec_status,
            ai_add_knowledge,
            ai_connect_mcp_server,
            ai_list_mcp_servers,
            ai_connect_mcp_stdio,
            ai_disconnect_mcp_server,
            ai_save_mcp_server_config,
            ai_delete_mcp_server_config,
            ai_list_mcp_server_configs,
            ai_reconnect_mcp_servers,
            ai_list_drafts,
            ai_promote_knowledge,
            ai_get_trace_events,
            ai_get_trace_events_by_type,
            ai_get_recent_trace_events,
            ai_browser_navigate,
            ai_update_cost_config,
            ai_set_reasoning_effort,
            ai_configure_llm,
            ai_get_workspace_root,
            ai_update_workspace_root,
            ai_select_workspace_dialog,
            ai_register_frontend_tool,
            ai_submit_frontend_tool_result,
            list_system_skills,
            proxy_ai_request,
            // JC9 MCP Server (built-in)
            ai_get_mcp_server_config,
            ai_set_mcp_server_config,
            ai_start_mcp_server,
            ai_stop_mcp_server,
            ai_get_mcp_server_status,
            ai_reindex_knowledge,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
