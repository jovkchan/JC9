mod process;
mod database;

use process::ProcessManager;
use database::{Database, Project, Shortcut, NoteGroup, Note};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use tauri::State;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

struct AppState {
    manager: ProcessManager,
    db: Database,
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
fn save_note(state: State<'_, Mutex<AppState>>, note: Note) -> Result<(), String> {
    state.lock().map_err(|e| e.to_string())?.db.save_note(&note)
}

#[tauri::command]
fn delete_note(state: State<'_, Mutex<AppState>>, id: String, permanent: Option<bool>) -> Result<(), String> {
    let db = &state.lock().map_err(|e| e.to_string())?.db;
    if permanent.unwrap_or(false) {
        db.permanently_delete_note(&id)
    } else {
        db.delete_note(&id)
    }
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
fn write_file_binary(path: String, data: Vec<u8>) -> Result<(), String> {
    fs::write(&path, data).map_err(|e| e.to_string())
}

#[tauri::command]
fn read_file_string(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| e.to_string())
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db = Database::new().expect("无法初始化数据库");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Mutex::new(AppState {
            manager: ProcessManager::new(),
            db,
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
            get_env_vars,
            generate_ssh_key,
            generate_ssl_cert,
            set_env_var,
            remove_env_var,
            write_file_binary,
            read_file_string,
            show_in_folder,
            get_note_groups,
            save_note_group,
            delete_note_group,
            get_notes,
            save_note,
            delete_note,
            search_notes,
            get_note_count,
            get_note_by_id,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
