// ── 自动化积木运行引擎（F1b）──
// 图解释器：从 start 块沿流程线 walk，执行 command / condition / delay / var-set，
// 维护运行时变量与上一块输出（last），携带登录凭据（env 注入），
// 通过 automation-event / pty-output 事件向前端汇报进度（契约见 docs/plans §7.2）。
// 说明：F1b 为本地执行版；平台适配（Docker/GitLab/...）+ 凭据加密鉴权在 F3/F5 接入。

use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::process::Command as StdCommand;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::Emitter;

use crate::ai::agent_manager::AgentManager;
use crate::system_notify;

/// 事件回调（解耦 AppHandle，便于单测；必须 Sync 以便并行分支线程共享）
type Emit<'a> = &'a (dyn Fn(&str, &Value) + Sync + 'a);
/// 实时输出回调（pty-output 分块流）
type OnOut<'a> = Option<&'a (dyn Fn(&[u8]) + Sync)>;

/// 运行中的任务取消标志（runId → cancel）。automation_stop 置位，引擎每步检查。
static ACTIVE_RUNS: OnceLock<StdMutex<HashMap<String, Arc<AtomicBool>>>> = OnceLock::new();
fn active_runs() -> &'static StdMutex<HashMap<String, Arc<AtomicBool>>> {
    ACTIVE_RUNS.get_or_init(|| StdMutex::new(HashMap::new()))
}

/// 请求停止某次运行（置取消位；正在执行的命令也会被 kill）
pub fn stop_automation(run_id: &str) -> Result<(), String> {
    let map = active_runs().lock().map_err(|_| "运行表锁失败".to_string())?;
    match map.get(run_id) {
        Some(flag) => {
            flag.store(true, Ordering::SeqCst);
            Ok(())
        }
        None => Err("未找到运行中的任务（可能已结束）".into()),
    }
}

/// 停止标记错误（run_automation 识别后发 stopped 事件而非 error）
const STOPPED_ERR: &str = "__jc9_stopped__";

fn check_cancel(cancel: &Arc<AtomicBool>) -> Result<(), String> {
    if cancel.load(Ordering::SeqCst) {
        return Err(STOPPED_ERR.into());
    }
    Ok(())
}

/// 通用子进程执行：可中断（cancel 置位即 kill）+ 超时 kill；返回 (code, stdout, stderr)。
/// on_out 非空时 stdout 实时分块回调（仿终端实时输出，长命令执行中即可看到进度）。
fn run_child(
    cmd: &mut StdCommand,
    timeout_secs: u64,
    cancel: &Arc<AtomicBool>,
    on_out: OnOut,
) -> Result<(i32, String, String), String> {
    let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("命令启动失败: {}", e))?;
    let mut so = child.stdout.take();
    let mut se = child.stderr.take();
    let out_buf: Arc<StdMutex<Vec<u8>>> = Arc::new(StdMutex::new(Vec::new()));
    let err_buf: Arc<StdMutex<Vec<u8>>> = Arc::new(StdMutex::new(Vec::new()));
    let wait_err: Arc<StdMutex<Option<String>>> = Arc::new(StdMutex::new(None));
    let mut status: Option<std::process::ExitStatus> = None;
    {
        let ob = Arc::clone(&out_buf);
        let eb = Arc::clone(&err_buf);
        let we = Arc::clone(&wait_err);
        std::thread::scope(|sc| {
            // stdout：分块读 + 累积 + 实时回调（供 pty-output 流）
            let t = sc.spawn(move || {
                use std::io::Read;
                if let Some(mut s) = so.take() {
                    let mut buf = [0u8; 4096];
                    loop {
                        match s.read(&mut buf) {
                            Ok(0) => break,
                            Ok(n) => {
                                if let Ok(mut g) = ob.lock() {
                                    g.extend_from_slice(&buf[..n]);
                                }
                                if let Some(f) = on_out {
                                    f(&buf[..n]);
                                }
                            }
                            Err(_) => break,
                        }
                    }
                }
            });
            // stderr：分块读 + 累积 + 实时回调（与 stdout 同流输出，错误也实时可见）
            let t2 = sc.spawn(move || {
                use std::io::Read;
                if let Some(mut s) = se.take() {
                    let mut buf = [0u8; 4096];
                    loop {
                        match s.read(&mut buf) {
                            Ok(0) => break,
                            Ok(n) => {
                                if let Ok(mut g) = eb.lock() {
                                    g.extend_from_slice(&buf[..n]);
                                }
                                if let Some(f) = on_out {
                                    f(&buf[..n]);
                                }
                            }
                            Err(_) => break,
                        }
                    }
                }
            });
            let start = Instant::now();
            loop {
                if cancel.load(Ordering::SeqCst) {
                    let _ = child.kill();
                    break;
                }
                if timeout_secs > 0 && start.elapsed().as_secs() >= timeout_secs {
                    let _ = child.kill();
                    break;
                }
                match child.try_wait() {
                    Ok(Some(st)) => {
                        status = Some(st);
                        break;
                    }
                    Ok(None) => {}
                    Err(e) => {
                        if let Ok(mut g) = we.lock() {
                            *g = Some(format!("命令执行失败: {}", e));
                        }
                        break;
                    }
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            if status.is_none() {
                let _ = child.wait();
            }
            let _ = t.join();
            let _ = t2.join();
        });
    }
    if let Some(e) = wait_err.lock().map_err(|_| "输出锁失败".to_string())?.take() {
        return Err(e);
    }
    let code = status.and_then(|s| s.code()).unwrap_or(-1);
    let out_l = out_buf.lock().map_err(|_| "输出锁失败".to_string())?;
    let err_l = err_buf.lock().map_err(|_| "输出锁失败".to_string())?;
    let stdout = String::from_utf8_lossy(&out_l).to_string();
    let stderr = String::from_utf8_lossy(&err_l).to_string();
    Ok((code, stdout, stderr))
}

/// 上一块输出（{{last.*}} 插值来源）
#[derive(Clone)]
pub struct LastResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub struct Ctx {
    pub vars: HashMap<String, Value>,
    pub last: Option<LastResult>,
    pub step: usize,
    /// 链路工作目录（workspace 环境块设置，下游命令块未指定 cwd 时继承）
    pub cwd: String,
    /// 链路环境变量（env 环境块设置，下游命令块继承；命令块自身 env 叠加覆盖）
    pub envs: HashMap<String, String>,
}

impl Ctx {
    /// 并行分支用的上下文副本（变量/上一步输出/工作目录各自独立）
    fn fork(&self) -> Ctx {
        Ctx { vars: self.vars.clone(), last: self.last.clone(), step: self.step, cwd: self.cwd.clone(), envs: self.envs.clone() }
    }
}

/// 单个积木执行日志（结构化，每个积木执行都记录；供流程分析/回放/排查）
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepLog {
    pub block_id: String,
    pub block_type: String,
    pub name: String,
    pub index: usize,
    pub status: String,            // ok / fail
    pub started_at: u64,
    pub ended_at: u64,
    pub duration_ms: u64,
    pub exit_code: Option<i32>,
    pub stdout_tail: String,
    /// 实际执行内容（已插值）：命令 / git 参数 / curl 参数 / 打开的网址 / 启动的程序等
    pub detail: String,
    pub cwd: String,
    /// 鉴权信息（凭据名，不含明文）
    pub auth: String,
    /// 循环迭代序号（loop 块内）
    pub iteration: Option<usize>,
    /// 并行分支序号（parallel 块内，0 起）
    pub branch: Option<usize>,
}

/// 一次运行的结构化日志（写入 ~/.jc9/data/automation_logs.json）
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct RunLog {
    id: String,
    automation_id: String,
    automation_name: String,
    entry: String,
    status: String,            // done / failed / stopped
    started_at: u64,
    ended_at: u64,
    duration_ms: u64,
    error: Option<String>,
    steps: Vec<StepLog>,
}

fn get_str<'a>(m: &'a Map<String, Value>, key: &str) -> &'a str {
    m.get(key).and_then(|v| v.as_str()).unwrap_or("")
}

/// 数字字段读取：支持 number 与字符串（插值用）
fn get_num_str(m: &Map<String, Value>, key: &str) -> String {
    m.get(key)
        .map(|v| v.as_str().map(String::from).unwrap_or_else(|| v.to_string()))
        .unwrap_or_default()
}

fn config_of(node: &Map<String, Value>) -> Map<String, Value> {
    node.get("config")
        .and_then(|c| c.as_object())
        .cloned()
        .unwrap_or_default()
}

/// 统一插值：{{var}} / {{last.stdout}} / {{last.exitCode}} / {{last.stderr}}
fn interpolate(s: &str, ctx: &Ctx) -> String {
    let mut out = s.to_string();
    // 变量
    for (k, v) in &ctx.vars {
        let pat = format!("{{{{{}}}}}", k);
        let val = v.as_str().map(|x| x.to_string()).unwrap_or_else(|| v.to_string());
        out = out.replace(&pat, &val);
    }
    if let Some(last) = &ctx.last {
        out = out
            .replace("{{last.stdout}}", &last.stdout)
            .replace("{{last.stderr}}", &last.stderr)
            .replace("{{last.exitCode}}", &last.exit_code.to_string());
    }
    out
}

/// 本地执行命令（Windows 默认 PowerShell；其他平台 sh -c）；可超时 / 可取消
fn exec_command(command: &str, cwd: &str, shell: &str, envs: &[(String, String)], timeout_secs: u64, cancel: &Arc<AtomicBool>, on_out: OnOut) -> Result<(i32, String, String), String> {
    #[cfg(target_os = "windows")]
    let (bin, arg, full) = match shell {
        "cmd" => ("cmd", "/C", command.to_string()),
        "bash" => ("bash", "-c", command.to_string()),
        "python" => ("python", "-c", command.to_string()),
        "node" => ("node", "-e", command.to_string()),
        _ => (
            "powershell",
            "-Command",
            format!("[Console]::OutputEncoding=[Text.Encoding]::UTF8; $ErrorActionPreference='Stop'; {}", command),
        ),
    };
    #[cfg(not(target_os = "windows"))]
    let (bin, arg, full) = match shell {
        "cmd" => ("cmd", "/C", command.to_string()),
        "bash" => ("bash", "-c", command.to_string()),
        "python" => ("python", "-c", command.to_string()),
        "node" => ("node", "-e", command.to_string()),
        "sh" => ("sh", "-c", command.to_string()),
        _ => ("sh", "-c", command.to_string()),
    };

    let mut cmd = StdCommand::new(bin);
    cmd.args([arg, &full]);
    if !cwd.is_empty() {
        cmd.current_dir(cwd);
    }
    for (k, v) in envs {
        cmd.env(k, v);
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    run_child(&mut cmd, timeout_secs, cancel, on_out)
}

/// 条件求值（插值后比较；数字优先数值比较，否则字符串）
fn eval_condition(left: &str, op: &str, right: &str) -> bool {
    let parse_num = |s: &str| -> Option<f64> { s.trim().parse::<f64>().ok() };
    match op {
        "==" => match (parse_num(left), parse_num(right)) {
            (Some(a), Some(b)) => (a - b).abs() < f64::EPSILON,
            _ => left == right,
        },
        "!=" => match (parse_num(left), parse_num(right)) {
            (Some(a), Some(b)) => (a - b).abs() >= f64::EPSILON,
            _ => left != right,
        },
        ">" => match (parse_num(left), parse_num(right)) {
            (Some(a), Some(b)) => a > b,
            _ => left > right,
        },
        "<" => match (parse_num(left), parse_num(right)) {
            (Some(a), Some(b)) => a < b,
            _ => left < right,
        },
        "contains" => left.contains(right),
        _ => left == right,
    }
}

/// 解析目标块的凭据连线：凭据积木(cred-out) → 本块(cred-in)，取其 credentialId 注入环境变量
/// （F1b 注入可见字段；F3 改为解密后的真实凭据 + 平台适配层鉴权）
fn login_envs_for_node(nodes: &[Value], edges: &[Value], node_id: &str) -> Vec<(String, String)> {
    // 找到指向本块 cred-in 的边 → 源块为凭据积木
    let mut cred_id: Option<String> = None;
    for e in edges {
        let eo = match e.as_object() {
            Some(o) => o,
            None => continue,
        };
        if eo.get("toBlock").and_then(|v| v.as_str()) == Some(node_id)
            && eo.get("toPort").and_then(|v| v.as_str()) == Some("cred-in")
        {
            if let Some(src) = eo.get("fromBlock").and_then(|v| v.as_str()) {
                if let Some(src_node) = nodes.iter().find(|n| n.get("id").and_then(|i| i.as_str()) == Some(src)) {
                    if src_node.get("type").and_then(|t| t.as_str()) == Some("credential") {
                        if let Some(cfg) = src_node.get("config").and_then(|c| c.as_object()) {
                            if let Some(id) = cfg.get("credentialId").and_then(|v| v.as_str()) {
                                cred_id = Some(id.to_string());
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
    let cred_id = match cred_id {
        Some(id) => id,
        None => return Vec::new(),
    };
    // 从 credentials.json 注入（AES-GCM 字段解密后注入 env）
    let mut envs: Vec<(String, String)> = Vec::new();
    let key = crate::credential_crypto::load_or_create_key().ok();
    if let Ok(content) = std::fs::read_to_string(dirs_data().join("credentials.json")) {
        if let Ok(arr) = serde_json::from_str::<Value>(&content) {
            if let Some(list) = arr.as_array() {
                for c in list {
                    if c.get("id").and_then(|i| i.as_str()) == Some(&cred_id) {
                        let src = c.get("fields").and_then(|f| f.as_object());
                        let src = src.or_else(|| c.get("masked").and_then(|m| m.as_object()));
                        if let Some(o) = src {
                            let dec = |v: &str| -> String {
                                key.as_ref().map(|k| crate::credential_crypto::decrypt_field(k, v).unwrap_or_else(|_| v.to_string())).unwrap_or_else(|| v.to_string())
                            };
                            if let Some(t) = o.get("token").and_then(|v| v.as_str()) {
                                let t = dec(t);
                                envs.push(("CI_TOKEN".into(), t.clone()));
                                envs.push(("AUTOMATION_CREDENTIAL_TOKEN".into(), t));
                            }
                            if let Some(u) = o.get("username").and_then(|v| v.as_str()) {
                                envs.push(("AUTOMATION_CREDENTIAL_USERNAME".into(), dec(u)));
                            }
                            if let Some(p) = o.get("password").and_then(|v| v.as_str()) {
                                envs.push(("AUTOMATION_CREDENTIAL_PASSWORD".into(), dec(p)));
                            }
                            if let Some(k) = o.get("kubeconfig").and_then(|v| v.as_str()) {
                                envs.push(("AUTOMATION_CREDENTIAL_KUBECONFIG".into(), dec(k)));
                            }
                            if let Some(u) = o.get("url").and_then(|v| v.as_str()) {
                                envs.push(("AUTOMATION_CREDENTIAL_URL".into(), u.to_string()));
                            }
                        }
                        envs.push(("AUTOMATION_CREDENTIAL_ID".into(), cred_id.clone()));
                        break;
                    }
                }
            }
        }
    }
    envs
}

/// 打开网址：Windows 用 cmd start（可指定浏览器），其他平台 xdg-open
fn exec_open_url(url: &str, browser: &str) -> Result<(i32, String, String), String> {
    #[cfg(target_os = "windows")]
    {
        let args: Vec<&str> = match browser {
            "chrome" => vec!["/C", "start", "", "chrome", url],
            "edge" => vec!["/C", "start", "", "msedge", url],
            "firefox" => vec!["/C", "start", "", "firefox", url],
            _ => vec!["/C", "start", "", url], // 系统默认
        };
        let out = StdCommand::new("cmd")
            .args(&args)
            .output()
            .map_err(|e| format!("打开网址失败: {}", e))?;
        Ok((
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).to_string(),
            String::from_utf8_lossy(&out.stderr).to_string(),
        ))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let out = StdCommand::new("xdg-open")
            .arg(url)
            .output()
            .map_err(|e| format!("打开网址失败: {}", e))?;
        Ok((
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).to_string(),
            String::from_utf8_lossy(&out.stderr).to_string(),
        ))
    }
}

/// 在工作区目录执行 git 命令（可取消）
fn run_git(cwd: &str, args: &[String], cancel: &Arc<AtomicBool>, on_out: OnOut) -> Result<(i32, String, String), String> {
    let mut cmd = StdCommand::new("git");
    cmd.args(args);
    if !cwd.is_empty() {
        cmd.current_dir(cwd);
    }
    run_child(&mut cmd, 0, cancel, on_out)
}

/// 由 GIT 积木类型与配置构造 git 参数（插值后）
fn git_args(kind: &str, config: &Map<String, Value>, ctx: &Ctx) -> Vec<String> {
    match kind {
        "git-clone" => {
            let mut a = vec!["clone".to_string(), interpolate(get_str(config, "repo"), ctx)];
            let dir = interpolate(get_str(config, "dir"), ctx);
            if !dir.is_empty() {
                a.push(dir);
            }
            let branch = interpolate(get_str(config, "branch"), ctx);
            if !branch.is_empty() {
                a.push("-b".into());
                a.push(branch);
            }
            a
        }
        "git-status" => {
            // 查看变更：默认 status --short 简洁清单；可选 diff / log；可选路径过滤
            let action = get_str(config, "action");
            let mut a = match action {
                "diff" => vec!["diff".to_string()],
                "log" => vec!["log".to_string(), "-5".into(), "--oneline".into()],
                _ => vec!["status".to_string(), "--short".into()],
            };
            let path = interpolate(get_str(config, "path"), ctx);
            if !path.is_empty() {
                a.push(path);
            }
            a
        }
        "git-push" => {
            let mut a = vec!["push".to_string()];
            let remote = interpolate(get_str(config, "remote"), ctx);
            let branch = interpolate(get_str(config, "branch"), ctx);
            if !remote.is_empty() {
                a.push(remote);
            }
            if !branch.is_empty() {
                a.push(branch);
            }
            a
        }
        "git-pull" => {
            let mut a = vec!["pull".to_string()];
            let remote = interpolate(get_str(config, "remote"), ctx);
            let branch = interpolate(get_str(config, "branch"), ctx);
            if !remote.is_empty() {
                a.push(remote);
            }
            if !branch.is_empty() {
                a.push(branch);
            }
            a
        }
        "git-branch" => {
            let action = get_str(config, "action");
            let name = interpolate(get_str(config, "name"), ctx);
            match action {
                "create" => vec!["checkout".into(), "-b".into(), name],
                "delete" => vec!["branch".into(), "-D".into(), name],
                "list" => vec!["branch".into()],
                "merge" => vec!["merge".into(), name],
                _ => vec!["checkout".into(), name],
            }
        }
        "git-tag" => {
            let action = get_str(config, "action");
            let tag = interpolate(get_str(config, "tag"), ctx);
            let msg = interpolate(get_str(config, "message"), ctx);
            match action {
                "delete" => vec!["tag".into(), "-d".into(), tag],
                "list" => vec!["tag".into()],
                _ => {
                    let mut a = vec!["tag".into(), tag];
                    if !msg.is_empty() {
                        a.push("-m".into());
                        a.push(msg);
                    }
                    a
                }
            }
        }
        _ => Vec::new(),
    }
}

/// 查询目标块「凭据端口」连线的凭据明文 fields（AES-GCM 字段解密后返回）
fn cred_for_node(nodes: &[Value], edges: &[Value], node_id: &str) -> Option<Map<String, Value>> {
    for e in edges {
        let eo = e.as_object()?;
        if eo.get("toBlock")?.as_str()? != node_id || eo.get("toPort")?.as_str()? != "cred-in" {
            continue;
        }
        let src = eo.get("fromBlock")?.as_str()?;
        let src_node = nodes.iter().find(|n| n.get("id").and_then(|i| i.as_str()) == Some(src))?;
        if src_node.get("type").and_then(|t| t.as_str()) != Some("credential") {
            continue;
        }
        let cred_id = src_node.get("config")?.get("credentialId")?.as_str()?;
        if let Ok(content) = std::fs::read_to_string(dirs_data().join("credentials.json")) {
            if let Ok(arr) = serde_json::from_str::<Value>(&content) {
                if let Some(list) = arr.as_array() {
                    for c in list {
                        if c.get("id").and_then(|i| i.as_str()) == Some(cred_id) {
                            if let Some(f) = c.get("fields").and_then(|x| x.as_object()) {
                                let key = crate::credential_crypto::load_or_create_key().ok();
                                let mut m = Map::new();
                                for (k, v) in f {
                                    let val = v.as_str()
                                        .map(|s| key.as_ref().map(|kk| crate::credential_crypto::decrypt_field(kk, s).unwrap_or_else(|_| s.to_string())).unwrap_or_else(|| s.to_string()))
                                        .unwrap_or_else(|| v.to_string());
                                    m.insert(k.clone(), Value::String(val));
                                }
                                return Some(m);
                            }
                        }
                    }
                }
            }
        }
        break;
    }
    None
}

/// 执行 curl（返回 stdout/stderr；可取消）
fn run_curl(args: &[String], cancel: &Arc<AtomicBool>, on_out: OnOut) -> Result<(i32, String, String), String> {
    let mut cmd = StdCommand::new("curl");
    cmd.args(args);
    run_child(&mut cmd, 0, cancel, on_out)
}

/// Jenkins：触发构建 / 查状态 / 控制台输出（Basic Auth + Crumb CSRF）；可取消
fn exec_jenkins(url: &str, job: &str, action: &str, build: &str, user: &str, token: &str, cancel: &Arc<AtomicBool>, on_out: OnOut) -> Result<(i32, String, String), String> {
    let root = url.trim_end_matches('/');
    let auth = format!("{}:{}", user, token);
    match action {
        "trigger" => {
            // 1) 取 Crumb
            let crumb_api = format!("{}/crumbIssuer/api/json", root);
            let out = run_curl(&["-s".into(), "-u".into(), auth.clone(), crumb_api], cancel, on_out)?;
            let crumb = serde_json::from_str::<Value>(&out.1)
                .ok()
                .and_then(|v| v.get("crumb").and_then(|c| c.as_str()).map(String::from))
                .unwrap_or_default();
            // 2) POST build
            let api = format!("{}/job/{}/build", root, job);
            let mut args = vec!["-s".into(), "-u".into(), auth.clone(), "-X".into(), "POST".into()];
            if !crumb.is_empty() {
                args.push("-H".into());
                args.push(format!("Jenkins-Crumb: {}", crumb));
            }
            args.push(api);
            run_curl(&args, cancel, on_out)
        }
        "status" => {
            let api = format!("{}/job/{}/lastBuild/api/json", root, job);
            run_curl(&["-s".into(), "-u".into(), auth, api], cancel, on_out)
        }
        _ => {
            let b = if build.is_empty() { "lastBuild" } else { build };
            let api = format!("{}/job/{}/{}/consoleText", root, job, b);
            run_curl(&["-s".into(), "-u".into(), auth, api], cancel, on_out)
        }
    }
}

/// Harbor：登录 + 构建 + 推送镜像（docker CLI）；可取消
fn exec_harbor(url: &str, project: &str, repo: &str, tag: &str, context: &str, dockerfile: &str, user: &str, pwd: &str, cancel: &Arc<AtomicBool>, on_out: OnOut) -> Result<(i32, String, String), String> {
    let registry = url.trim_end_matches('/');
    let image = format!("{}/{}/{}:{}", registry, project, repo, if tag.is_empty() { "latest" } else { tag });
    let mut all_out = String::new();
    let mut all_err = String::new();
    // 1) login
    let mut login_cmd = StdCommand::new("docker");
    login_cmd.args(["login", registry, "-u", user, "-p", pwd]);
    let (lcode, lout, lerr) = run_child(&mut login_cmd, 0, cancel, on_out)?;
    all_out.push_str(&lout);
    all_err.push_str(&lerr);
    if lcode != 0 {
        return Ok((lcode, all_out, all_err));
    }
    // 2) build
    let ctx = if context.is_empty() { "." } else { context };
    let mut bargs = vec!["build".to_string(), "-t".to_string(), image.clone()];
    if !dockerfile.is_empty() {
        bargs.push("-f".into());
        bargs.push(dockerfile.into());
    }
    bargs.push(ctx.into());
    let mut build_cmd = StdCommand::new("docker");
    build_cmd.args(&bargs);
    let (bcode, bout, berr) = run_child(&mut build_cmd, 0, cancel, on_out)?;
    all_out.push_str(&bout);
    all_err.push_str(&berr);
    if bcode != 0 {
        return Ok((bcode, all_out, all_err));
    }
    // 3) push
    let mut push_cmd = StdCommand::new("docker");
    push_cmd.arg("push").arg(&image);
    let (pcode, pout, perr) = run_child(&mut push_cmd, 0, cancel, on_out)?;
    all_out.push_str(&pout);
    all_err.push_str(&perr);
    Ok((pcode, all_out, all_err))
}

/// K8S：kubectl（kubeconfig 内容写临时文件，经 KUBECONFIG 注入）；可取消
fn exec_k8s(action: &str, file: &str, kind: &str, name: &str, ns: &str, kubeconfig: &str, cancel: &Arc<AtomicBool>, on_out: OnOut) -> Result<(i32, String, String), String> {
    // 写 kubeconfig 临时文件
    let tmp_path = std::env::temp_dir().join(format!("jc9-kube-{}.yaml", now_ts()));
    std::fs::write(&tmp_path, kubeconfig).map_err(|e| format!("写 kubeconfig 失败: {}", e))?;
    let args: Vec<String> = match action {
        "apply" => {
            let mut a = vec!["apply".into(), "-f".into(), file.into()];
            if !ns.is_empty() { a.push("-n".into()); a.push(ns.into()); }
            a
        }
        "rollout" => {
            let k: String = if kind.is_empty() { "deployment".to_string() } else { kind.to_string() };
            let mut a = vec!["rollout".into(), "status".into(), format!("{}/{}", k, name)];
            if !ns.is_empty() { a.push("-n".into()); a.push(ns.into()); }
            a
        }
        "get" => {
            let k: String = if kind.is_empty() { "pods".to_string() } else { kind.to_string() };
            let mut a = vec!["get".into(), k];
            if !name.is_empty() { a.push(name.into()); }
            if !ns.is_empty() { a.push("-n".into()); a.push(ns.into()); }
            a
        }
        _ => {
            let mut a = vec!["logs".into(), name.into()];
            if !ns.is_empty() { a.push("-n".into()); a.push(ns.into()); }
            a
        }
    };
    let mut sc = StdCommand::new("kubectl");
    sc.args(&args);
    sc.env("KUBECONFIG", &tmp_path);
    let (code, out, err) = run_child(&mut sc, 0, cancel, on_out)?;
    let _ = std::fs::remove_file(&tmp_path);
    Ok((code, out, err))
}

/// Docker：通用本地操作（build/pull/run/compose/images/ps/logs/exec/stop/rm）；可取消
fn exec_docker(action: &str, image: &str, tag: &str, context: &str, dockerfile: &str, container: &str, service: &str, cmd: &str, cancel: &Arc<AtomicBool>, on_out: OnOut) -> Result<(i32, String, String), String> {
    let full = if image.is_empty() { String::new() } else { format!("{}:{}", image, if tag.is_empty() { "latest" } else { tag }) };
    let mut args: Vec<String> = Vec::new();
    match action {
        "build" => {
            args.push("build".into());
            if !dockerfile.is_empty() { args.push("-f".into()); args.push(dockerfile.into()); }
            if !full.is_empty() { args.push("-t".into()); args.push(full); }
            args.push(if context.is_empty() { ".".into() } else { context.into() });
        }
        "pull" => { args.push("pull".into()); args.push(full); }
        "run" => {
            args.push("run".into());
            if !container.is_empty() { args.push("--name".into()); args.push(container.into()); }
            args.push("-d".into());
            args.push(full);
            if !cmd.is_empty() { args.extend(cmd.split_whitespace().map(String::from)); }
        }
        "compose" => {
            args.push("compose".into());
            args.push("up".into());
            args.push("-d".into());
            if !service.is_empty() { args.push(service.into()); }
        }
        "images" => { args.push("images".into()); if !full.is_empty() { args.push(full); } }
        "ps" => { args.push("ps".into()); args.push("-a".into()); }
        "logs" => { args.push("logs".into()); args.push(container.into()); }
        "exec" => {
            args.push("exec".into());
            args.push(container.into());
            if !cmd.is_empty() { args.extend(cmd.split_whitespace().map(String::from)); }
        }
        "stop" => { args.push("stop".into()); args.push(container.into()); }
        _ => { args.push("rm".into()); args.push("-f".into()); args.push(container.into()); }
    }
    let mut sc = StdCommand::new("docker");
    sc.args(&args);
    if (action == "build" || action == "compose") && !context.is_empty() {
        sc.current_dir(context);
    }
    run_child(&mut sc, 0, cancel, on_out)
}

/// GitLab：服务操作（触发流水线/查状态/任务日志/建 MR），PRIVATE-TOKEN 鉴权；可取消
fn exec_gitlab(url: &str, project: &str, action: &str, ref_: &str, job_id: &str, src: &str, target: &str, title: &str, token: &str, cancel: &Arc<AtomicBool>, on_out: OnOut) -> Result<(i32, String, String), String> {
    let root = url.trim_end_matches('/');
    let api = format!("{}/api/v4/projects/{}", root, project);
    let auth = format!("PRIVATE-TOKEN: {}", token);
    let args: Vec<String> = match action {
        "pipeline-trigger" => {
            let mut a = vec!["-s".into(), "-H".into(), auth, "-X".into(), "POST".into()];
            if !ref_.is_empty() { a.push("-d".into()); a.push(format!("ref={}", ref_)); }
            a.push(format!("{}/pipelines", api));
            a
        }
        "pipeline-status" => {
            vec!["-s".into(), "-H".into(), auth, format!("{}/pipelines/latest", api)]
        }
        "job-log" => {
            vec!["-s".into(), "-H".into(), auth, format!("{}/jobs/{}/trace", api, job_id)]
        }
        _ => {
            let mut a = vec!["-s".into(), "-H".into(), auth, "-X".into(), "POST".into()];
            a.push("-d".into());
            a.push(format!("source_branch={}&target_branch={}&title={}", src, target, title));
            a.push(format!("{}/merge_requests", api));
            a
        }
    };
    run_curl(&args, cancel, on_out)
}

fn dirs_data() -> std::path::PathBuf {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".into());
    std::path::PathBuf::from(home).join(".jc9").join("data")
}

fn now_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 生成新运行 ID（后台线程启动前分配，供前端 stop / 事件匹配）
pub fn new_run_id() -> String {
    format!("run-{}", now_ts())
}

/// 可中断等待（分段 sleep，cancel 置位立即返回停止错误）
fn wait_ms(ms: u64, cancel: &Arc<AtomicBool>) -> Result<(), String> {
    let mut left = ms;
    while left > 0 {
        check_cancel(cancel)?;
        let step = left.min(100);
        std::thread::sleep(Duration::from_millis(step));
        left -= step;
    }
    Ok(())
}

/// 判断某边是否指向循环块的回边（to 为 loop 块且连到 loop-in 端口）
fn is_loop_back(nodes: &[Value], to_block: &str, to_port: Option<&str>) -> bool {
    if to_port != Some("loop-in") {
        return false;
    }
    nodes
        .iter()
        .find(|n| n.get("id").and_then(|i| i.as_str()) == Some(to_block))
        .map(|n| n.get("type").and_then(|t| t.as_str()) == Some("loop"))
        .unwrap_or(false)
}

/// 连到本块 cred-in 端口的凭据积木名（日志「鉴权」列，不含明文）
fn cred_name_for_node(nodes: &[Value], edges: &[Value], node_id: &str) -> String {
    for e in edges {
        let eo = match e.as_object() {
            Some(o) => o,
            None => continue,
        };
        if eo.get("toBlock").and_then(|v| v.as_str()) == Some(node_id)
            && eo.get("toPort").and_then(|v| v.as_str()) == Some("cred-in")
        {
            if let Some(src) = eo.get("fromBlock").and_then(|v| v.as_str()) {
                if let Some(n) = nodes.iter().find(|n| n.get("id").and_then(|i| i.as_str()) == Some(src)) {
                    if let Some(cfg) = n.get("config").and_then(|c| c.as_object()) {
                        if let Some(nm) = cfg.get("credentialName").and_then(|v| v.as_str()) {
                            return nm.to_string();
                        }
                    }
                }
            }
        }
    }
    String::new()
}

/// 记录一个积木的执行日志（结构化为 StepLog），并推送 step_log 事件供前端实时显示
fn log_step(
    run_id: &str,
    emit: Emit,
    steps: &mut Vec<StepLog>,
    block_id: &str,
    block_type: &str,
    name: &str,
    index: usize,
    status: &str,
    started_at: u64,
    exit_code: Option<i32>,
    stdout_tail: &str,
    detail: &str,
    cwd: &str,
    auth: &str,
    iteration: Option<usize>,
    branch: Option<usize>,
) {
    let ended = now_ts();
    let sl = StepLog {
        block_id: block_id.to_string(),
        block_type: block_type.to_string(),
        name: name.to_string(),
        index,
        status: status.to_string(),
        started_at,
        ended_at: ended,
        duration_ms: ended.saturating_sub(started_at),
        exit_code,
        stdout_tail: stdout_tail.to_string(),
        detail: detail.to_string(),
        cwd: cwd.to_string(),
        auth: auth.to_string(),
        iteration,
        branch,
    };
    let mut v = serde_json::to_value(&sl).unwrap_or(Value::Null);
    if let Some(o) = v.as_object_mut() {
        o.insert("runId".to_string(), Value::String(run_id.to_string()));
    }
    let _ = emit("step_log", &v);
    steps.push(sl);
}

fn logs_path() -> std::path::PathBuf {
    dirs_data().join("automation_logs.json")
}

/// 读取全部运行日志（最新在前）
pub fn list_automation_logs() -> Result<String, String> {
    let p = logs_path();
    if !p.exists() {
        return Ok("[]".into());
    }
    std::fs::read_to_string(&p).map_err(|e| format!("读取日志失败: {e}"))
}

/// 追加一条运行日志（保留最近 200 条）
fn append_run_log(log: &RunLog) {
    let max = 200usize;
    let mut logs: Vec<Value> = match std::fs::read_to_string(logs_path()) {
        Ok(s) => serde_json::from_str::<Value>(&s)
            .ok()
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    };
    logs.insert(0, serde_json::to_value(log).unwrap_or(Value::Null));
    logs.truncate(max);
    if let Ok(s) = serde_json::to_string(&logs) {
        let _ = std::fs::write(logs_path(), s);
    }
}

/// 运行自动化（按 id 读取 → 校验 → walk 执行），返回 runId。
/// entry：可选入口块 id（手动触发）；缺省找「开始」，无「开始」则找第一个「手动触发」。
/// 子工作流嵌套调用最大深度（防 A→B→A 循环调用）
const MAX_CALL_DEPTH: usize = 8;

/// 解析自动化（纯函数，可从内存列表解析；顶层运行与「调用工作流」子调用共用）
/// 返回 (name, nodes, edges, 入口块 id, 默认变量表)
fn resolve_automation_from_list(
    list: &[Value],
    id: &str,
    entry: Option<&str>,
) -> Result<(String, Vec<Value>, Vec<Value>, String, HashMap<String, Value>), String> {
    let automation = list
        .iter()
        .find(|a| a.get("id").and_then(|i| i.as_str()) == Some(id))
        .ok_or_else(|| format!("未找到该自动化（id={}）", id))?;
    let name = automation.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();

    let nodes = automation.get("nodes").and_then(|n| n.as_array()).cloned().unwrap_or_default();
    let edges = automation.get("edges").and_then(|e| e.as_array()).cloned().unwrap_or_default();
    if nodes.is_empty() {
        return Err("自动化为空，请先添加积木".into());
    }

    // 起始变量（automation.variables 的 value 展开）
    let mut vars: HashMap<String, Value> = HashMap::new();
    if let Some(vars_obj) = automation.get("variables").and_then(|v| v.as_object()) {
        for (k, v) in vars_obj {
            let val = v.get("value").cloned().unwrap_or(Value::Null);
            vars.insert(k.clone(), val);
        }
    }

    // 入口：显式 entry → start → 第一个 manual-trigger
    let entry_id: Option<String> = entry
        .filter(|s| !s.is_empty())
        .map(String::from)
        .or_else(|| {
            nodes
                .iter()
                .find(|n| n.get("type").and_then(|t| t.as_str()) == Some("start"))
                .and_then(|n| n.get("id").and_then(|i| i.as_str()).map(String::from))
        })
        .or_else(|| {
            nodes
                .iter()
                .find(|n| n.get("type").and_then(|t| t.as_str()) == Some("manual-trigger"))
                .and_then(|n| n.get("id").and_then(|i| i.as_str()).map(String::from))
        });
    let start_id = match entry_id {
        Some(s) => s,
        None => return Err("缺少「开始」或「手动触发」积木".into()),
    };

    Ok((name, nodes, edges, start_id, vars))
}

/// 按 id 读取并解析自动化（读 ~/.jc9/data/automations.json）
fn resolve_automation(id: &str, entry: Option<&str>) -> Result<(String, Vec<Value>, Vec<Value>, String, HashMap<String, Value>), String> {
    let content = std::fs::read_to_string(dirs_data().join("automations.json"))
        .map_err(|e| format!("读取 automations 失败: {e}"))?;
    let arr: Value = serde_json::from_str(&content).map_err(|e| format!("解析 automations 失败: {e}"))?;
    let list = arr.as_array().ok_or("automations 格式错误")?;
    resolve_automation_from_list(list, id, entry)
}

pub fn run_automation(app: &tauri::AppHandle, id: &str, entry: Option<String>, ai: Option<Arc<AgentManager>>, run_id: &str) -> Result<String, String> {
    let run_id = run_id.to_string();
    let (name0, nodes, edges, start_id, vars) = resolve_automation(id, entry.as_deref())?;
    let total = nodes.len();

    let mut ctx = Ctx { vars, last: None, step: 0, cwd: String::new(), envs: HashMap::new() };
    let mut visited: HashSet<String> = HashSet::new();
    let cancel = Arc::new(AtomicBool::new(false));
    active_runs()
        .lock()
        .map_err(|_| "运行表锁失败".to_string())?
        .insert(run_id.clone(), Arc::clone(&cancel));

    let emit: Emit = &|ev: &str, payload: &Value| {
        app.emit(ev, payload.clone()).ok();
    };

    emit("automation-event", &json!({
        "type": "started", "runId": run_id, "automationId": id, "name": name0,
        "step": 0, "total": total, "ts": now_ts()
    }));

    let started_at = now_ts();
    let mut steps: Vec<StepLog> = Vec::new();
    let result = walk(&run_id, &nodes, &edges, &start_id, &mut ctx, &mut visited, total, &cancel, emit, &mut steps, ai.as_ref(), Some(app), 0);
    active_runs()
        .lock()
        .map_err(|_| "运行表锁失败".to_string())?
        .remove(&run_id);

    let (status, err) = match result {
        Ok(()) => ("done".to_string(), None),
        Err(e) if e == STOPPED_ERR => ("stopped".to_string(), None),
        Err(e) => ("failed".to_string(), Some(e)),
    };
    let run_log = RunLog {
        id: run_id.clone(),
        automation_id: id.to_string(),
        automation_name: name0.clone(),
        entry: start_id.clone(),
        status: status.clone(),
        started_at,
        ended_at: now_ts(),
        duration_ms: now_ts().saturating_sub(started_at),
        error: err.clone(),
        steps,
    };
    append_run_log(&run_log);

    match status.as_str() {
        "done" => {
            emit("automation-event", &json!({
                "type": "done", "runId": run_id, "automationId": id, "step": ctx.step, "total": total,
                "vars": ctx.vars, "ts": now_ts()
            }));
            Ok(run_id)
        }
        "stopped" => {
            emit("automation-event", &json!({
                "type": "stopped", "runId": run_id, "automationId": id, "step": ctx.step, "total": total,
                "vars": ctx.vars, "ts": now_ts()
            }));
            Ok(run_id)
        }
        _ => {
            let e = err.unwrap_or_default();
            emit("automation-event", &json!({
                "type": "error", "runId": run_id, "automationId": id, "step": ctx.step, "total": total,
                "error": e, "vars": ctx.vars, "ts": now_ts()
            }));
            Err(e)
        }
    }
}

pub fn walk(
    run_id: &str,
    nodes: &[Value],
    edges: &[Value],
    node_id: &str,
    ctx: &mut Ctx,
    visited: &mut HashSet<String>,
    total: usize,
    cancel: &Arc<AtomicBool>,
    emit: Emit,
    steps: &mut Vec<StepLog>,
    ai: Option<&Arc<AgentManager>>,
    app: Option<&tauri::AppHandle>,
    depth: usize,
) -> Result<(), String> {
    if !visited.insert(node_id.to_string()) {
        return Ok(()); // 防环（循环体单独传新 visited，见 loop 分支）
    }
    check_cancel(cancel)?;
    let node = match nodes.iter().find(|n| n.get("id").and_then(|i| i.as_str()) == Some(node_id)) {
        Some(n) => n.as_object().ok_or("节点格式错误")?,
        None => return Ok(()),
    };
    let name = get_str(node, "type");
    let node_label = get_str(node, "type").to_string();
    ctx.step += 1;
    let started_at = now_ts();
    let auth = cred_name_for_node(nodes, edges, node_id);

    emit("automation-event", &json!({
        "type": "step_start", "runId": run_id, "blockId": node_id, "name": node_label,
        "step": ctx.step, "total": total, "ts": started_at
    }));

    let config = config_of(node);

    // 按类型执行
    let mut branch: Option<String> = None; // condition 结果端口
    match name {
        "command" => {
            let raw = get_str(&config, "command").to_string();
            if raw.is_empty() {
                return Err(format!("命令块 {}：缺少命令", node_id));
            }
            let command = interpolate(&raw, ctx);
            // 工作目录统一由「工作区」块设置（链路 ctx.cwd），命令块无独立 cwd
            let cwd = ctx.cwd.clone();
            let shell = get_str(&config, "shell").to_string();
            // 环境变量：链路环境块 ctx.envs + 命令块自身 config.env（叠加，后者覆盖）+ 凭据注入
            let mut envs: Vec<(String, String)> = Vec::new();
            for (k, v) in &ctx.envs {
                envs.push((k.clone(), v.clone()));
            }
            let env_raw = get_str(&config, "env");
            for line in env_raw.lines() {
                if let Some((k, v)) = line.split_once('=') {
                    envs.push((k.trim().to_string(), interpolate(v.trim(), ctx)));
                }
            }
            envs.extend(login_envs_for_node(nodes, edges, node_id));
            let timeout = get_num_str(&config, "timeoutSecs").parse::<u64>().unwrap_or(0);

            // 实时输出（pty-output 分块流，仿终端）：长命令执行中即可看到进度
            let on_out: Option<&(dyn Fn(&[u8]) + Sync)> = Some(&|data: &[u8]| {
                let _ = emit("pty-output", &json!({ "processId": run_id, "data": data }));
            });
            let (code, out, err) = match exec_command(&command, &cwd, &shell, &envs, timeout, cancel, on_out) {
                Ok(v) => v,
                Err(msg) => {
                    emit("automation-event", &json!({
                        "type": "step_fail", "runId": run_id, "blockId": node_id, "name": node_label,
                        "exitCode": -1, "stdoutTail": "", "step": ctx.step, "total": total, "ts": now_ts()
                    }));
                    log_step(run_id, emit, steps, node_id, name, &node_label, ctx.step, "fail", started_at, None, "", &command, &cwd, &auth, None, None);
                    return Err(msg);
                }
            };
            // 输出上报（pty-output，processId=runId）
            let _ = emit("pty-output", &json!({ "processId": run_id, "data": out.as_bytes() }));
            if !err.is_empty() {
                let _ = emit("pty-output", &json!({ "processId": run_id, "data": err.as_bytes() }));
            }
            ctx.last = Some(LastResult { exit_code: code, stdout: out.clone(), stderr: err.clone() });
            let tail: String = out.chars().rev().take(200).collect::<Vec<_>>().into_iter().rev().collect();
            emit("automation-event", &json!({
                "type": if code == 0 { "step_done" } else { "step_fail" },
                "runId": run_id, "blockId": node_id, "name": node_label,
                "exitCode": code, "stdoutTail": tail, "vars": ctx.vars,
                "step": ctx.step, "total": total, "ts": now_ts()
            }));
            log_step(run_id, emit, steps, node_id, name, &node_label, ctx.step, if code == 0 { "ok" } else { "fail" }, started_at, Some(code), &tail, &command, &cwd, &auth, None, None);
            if code != 0 && get_str(&config, "onFail") != "continue" {
                return Err(format!("命令块 {} 失败，退出码 {}", node_id, code));
            }
        }
        "workspace" => {
            // 环境块：设置链路工作目录，下游命令块未指定 cwd 时继承
            let path = interpolate(get_str(&config, "path"), ctx);
            ctx.cwd = path.clone();
            ctx.last = Some(LastResult { exit_code: 0, stdout: format!("工作区: {}", path), stderr: String::new() });
            emit("automation-event", &json!({
                "type": "step_done", "runId": run_id, "blockId": node_id, "name": node_label,
                "exitCode": 0, "step": ctx.step, "total": total, "ts": now_ts()
            }));
            log_step(run_id, emit, steps, node_id, name, &node_label, ctx.step, "ok", started_at, Some(0), "", &path, &ctx.cwd, &auth, None, None);
        }
        "env" => {
            // 环境块：设置一组链路环境变量（KEY=VALUE 行，可多个），下游命令继承；值支持 {{var}} 插值
            let env_raw = get_str(&config, "env");
            let mut count = 0usize;
            for line in env_raw.lines() {
                if let Some((k, v)) = line.split_once('=') {
                    ctx.envs.insert(k.trim().to_string(), interpolate(v.trim(), ctx));
                    count += 1;
                }
            }
            ctx.last = Some(LastResult { exit_code: 0, stdout: format!("设置 {} 个环境变量", count), stderr: String::new() });
            emit("automation-event", &json!({
                "type": "step_done", "runId": run_id, "blockId": node_id, "name": node_label,
                "exitCode": 0, "step": ctx.step, "total": total, "ts": now_ts()
            }));
            let detail = format!("设置 {} 个环境变量", count);
            log_step(run_id, emit, steps, node_id, name, &node_label, ctx.step, "ok", started_at, Some(0), "", &detail, &ctx.cwd, &auth, None, None);
        }
        "open-url" => {
            // 打开网址：系统默认或指定浏览器（单一职责，不执行程序）
            let url = interpolate(get_str(&config, "url"), ctx);
            if url.is_empty() {
                return Err(format!("打开网址块 {}：缺少网址", node_id));
            }
            let browser = get_str(&config, "browser").to_string();
            let (code, out, err) = match exec_open_url(&url, &browser) {
                Ok(v) => v,
                Err(msg) => return Err(msg),
            };
            ctx.last = Some(LastResult { exit_code: code, stdout: out.clone(), stderr: err.clone() });
            emit("automation-event", &json!({
                "type": if code == 0 { "step_done" } else { "step_fail" },
                "runId": run_id, "blockId": node_id, "name": node_label,
                "exitCode": code, "stdoutTail": format!("打开 {}", url), "vars": ctx.vars,
                "step": ctx.step, "total": total, "ts": now_ts()
            }));
            log_step(run_id, emit, steps, node_id, name, &node_label, ctx.step, if code == 0 { "ok" } else { "fail" }, started_at, Some(code), &url, &url, &ctx.cwd, &auth, None, None);
            if code != 0 {
                return Err(format!("打开网址块 {} 失败，退出码 {}", node_id, code));
            }
        }
        "launch" => {
            // 启动程序：程序 + 参数 + 工作目录（默认不等待；wait=true 等待完成）
            let program = interpolate(get_str(&config, "program"), ctx);
            if program.is_empty() {
                return Err(format!("启动程序块 {}：缺少程序", node_id));
            }
            let args_raw = interpolate(get_str(&config, "args"), ctx);
            // 工作目录统一由「工作区」块设置（链路 ctx.cwd）
            let cwd = ctx.cwd.clone();
            let wait = config.get("wait").and_then(|w| w.as_bool()).unwrap_or(false);
            let args: Vec<&str> = args_raw.split_whitespace().collect();
            let mut cmd = StdCommand::new(&program);
            cmd.args(&args);
            if !cwd.is_empty() {
                cmd.current_dir(&cwd);
            }
            // wait=true 走可中断的 run_child（支持 stop + 实时输出）；否则 detach 启动
            let on_out: OnOut = Some(&|data: &[u8]| {
                let _ = emit("pty-output", &json!({ "processId": run_id, "data": data }));
            });
            let (code, out, err) = if wait {
                match run_child(&mut cmd, 0, cancel, on_out) {
                    Ok(v) => v,
                    Err(msg) => return Err(msg),
                }
            } else {
                // 不等待：detach 启动
                match cmd.spawn() {
                    Ok(_) => (0, format!("已启动 {}", program), String::new()),
                    Err(e) => return Err(format!("启动 {} 失败: {}", program, e)),
                }
            };
            ctx.last = Some(LastResult { exit_code: code, stdout: out.clone(), stderr: err.clone() });
            let detail = format!("{} {}", program, args_raw);
            emit("automation-event", &json!({
                "type": if code == 0 { "step_done" } else { "step_fail" },
                "runId": run_id, "blockId": node_id, "name": node_label,
                "exitCode": code, "stdoutTail": detail, "vars": ctx.vars,
                "step": ctx.step, "total": total, "ts": now_ts()
            }));
            log_step(run_id, emit, steps, node_id, name, &node_label, ctx.step, if code == 0 { "ok" } else { "fail" }, started_at, Some(code), "", &detail, &cwd, &auth, None, None);
        }
        gk @ ("git-clone" | "git-status" | "git-commit" | "git-push" | "git-pull" | "git-branch" | "git-tag") => {
            // 在工作区（ctx.cwd）执行 git 命令；git-commit 先 add 再 commit；实时输出（pty-output 流）
            let on_out: OnOut = Some(&|data: &[u8]| {
                let _ = emit("pty-output", &json!({ "processId": run_id, "data": data }));
            });
            let (code, out, err, detail) = if gk == "git-commit" {
                let add_all = config.get("addAll").and_then(|v| v.as_bool()).unwrap_or(true);
                if add_all {
                    match run_git(&ctx.cwd, &["add".to_string(), "-A".to_string()], cancel, on_out) {
                        Ok((c, _, _)) => { if c != 0 { return Err("git add 失败".into()); } }
                        Err(msg) => return Err(msg),
                    }
                }
                let msg = interpolate(get_str(&config, "message"), ctx);
                let detail = format!("git commit -m \"{}\"", msg);
                match run_git(&ctx.cwd, &["commit".to_string(), "-m".to_string(), msg], cancel, on_out) {
                    Ok(v) => (v.0, v.1, v.2, detail),
                    Err(msg) => return Err(msg),
                }
            } else {
                let args = git_args(gk, &config, ctx);
                let detail = format!("git {}", args.join(" "));
                match run_git(&ctx.cwd, &args, cancel, on_out) {
                    Ok(v) => (v.0, v.1, v.2, detail),
                    Err(msg) => return Err(msg),
                }
            };
            ctx.last = Some(LastResult { exit_code: code, stdout: out.clone(), stderr: err.clone() });
            let tail: String = out.chars().rev().take(200).collect::<Vec<_>>().into_iter().rev().collect();
            emit("automation-event", &json!({
                "type": if code == 0 { "step_done" } else { "step_fail" },
                "runId": run_id, "blockId": node_id, "name": node_label,
                "exitCode": code, "stdoutTail": tail, "vars": ctx.vars,
                "step": ctx.step, "total": total, "ts": now_ts()
            }));
            log_step(run_id, emit, steps, node_id, name, &node_label, ctx.step, if code == 0 { "ok" } else { "fail" }, started_at, Some(code), &tail, &detail, &ctx.cwd, &auth, None, None);
            if code != 0 {
                return Err(format!("{} 失败，退出码 {}", node_label, code));
            }
        }
        pk @ ("jenkins" | "harbor" | "k8s" | "docker" | "gitlab") => {
            // 平台积木：凭据经「凭据端口」连线注入
            let cred = cred_for_node(nodes, edges, node_id);
            // 实时输出（pty-output 流，docker build 等长操作可见进度）
            let on_out: OnOut = Some(&|data: &[u8]| {
                let _ = emit("pty-output", &json!({ "processId": run_id, "data": data }));
            });
            let cred_get = |k: &str| -> String {
                cred.as_ref().and_then(|f| f.get(k)).and_then(|v| v.as_str()).unwrap_or("").to_string()
            };
            let (code, out, err) = match pk {
                "docker" => {
                    let action = get_str(&config, "action").to_string();
                    let image = interpolate(get_str(&config, "image"), ctx);
                    let tag = interpolate(get_str(&config, "tag"), ctx);
                    let ctx_dir = interpolate(get_str(&config, "context"), ctx);
                    let df = interpolate(get_str(&config, "dockerfile"), ctx);
                    let container = interpolate(get_str(&config, "container"), ctx);
                    let service = interpolate(get_str(&config, "service"), ctx);
                    let cmd = interpolate(get_str(&config, "cmd"), ctx);
                    let cw = if ctx_dir.is_empty() { ctx.cwd.clone() } else { ctx_dir };
                    match exec_docker(&action, &image, &tag, &cw, &df, &container, &service, &cmd, cancel, on_out) {
                        Ok(v) => v,
                        Err(m) => return Err(m),
                    }
                }
                "gitlab" => {
                    let url = interpolate(get_str(&config, "url"), ctx);
                    let project = interpolate(get_str(&config, "project"), ctx);
                    let action = get_str(&config, "action").to_string();
                    let r = interpolate(get_str(&config, "ref"), ctx);
                    let job_id = interpolate(get_str(&config, "jobId"), ctx);
                    let src = interpolate(get_str(&config, "mrSource"), ctx);
                    let target = interpolate(get_str(&config, "mrTarget"), ctx);
                    let title = interpolate(get_str(&config, "mrTitle"), ctx);
                    let token = cred_get("token");
                    match exec_gitlab(&url, &project, &action, &r, &job_id, &src, &target, &title, &token, cancel, on_out) {
                        Ok(v) => v,
                        Err(m) => return Err(m),
                    }
                }
                "jenkins" => {
                    let url = interpolate(get_str(&config, "url"), ctx);
                    let job = interpolate(get_str(&config, "job"), ctx);
                    let action = get_str(&config, "action").to_string();
                    let build = interpolate(get_str(&config, "build"), ctx);
                    let user = cred_get("username");
                    let t = cred_get("token");
                    let token = if t.is_empty() { cred_get("password") } else { t };
                    match exec_jenkins(&url, &job, &action, &build, &user, &token, cancel, on_out) {
                        Ok(v) => v,
                        Err(m) => return Err(m),
                    }
                }
                "harbor" => {
                    let url = interpolate(get_str(&config, "url"), ctx);
                    let project = interpolate(get_str(&config, "project"), ctx);
                    let repo = interpolate(get_str(&config, "repo"), ctx);
                    let tag = interpolate(get_str(&config, "tag"), ctx);
                    let ctx_dir = interpolate(get_str(&config, "context"), ctx);
                    let df = interpolate(get_str(&config, "dockerfile"), ctx);
                    let user = cred_get("username");
                    let pwd = cred_get("password");
                    let cw = if ctx_dir.is_empty() { ctx.cwd.clone() } else { ctx_dir };
                    match exec_harbor(&url, &project, &repo, &tag, &cw, &df, &user, &pwd, cancel, on_out) {
                        Ok(v) => v,
                        Err(m) => return Err(m),
                    }
                }
                _ => {
                    let action = get_str(&config, "action").to_string();
                    let file = interpolate(get_str(&config, "file"), ctx);
                    let kind = interpolate(get_str(&config, "kind"), ctx);
                    let name = interpolate(get_str(&config, "name"), ctx);
                    let ns = interpolate(get_str(&config, "namespace"), ctx);
                    let kc = cred_get("kubeconfig");
                    match exec_k8s(&action, &file, &kind, &name, &ns, &kc, cancel, on_out) {
                        Ok(v) => v,
                        Err(m) => return Err(m),
                    }
                }
            };
            ctx.last = Some(LastResult { exit_code: code, stdout: out.clone(), stderr: err.clone() });
            let tail: String = out.chars().rev().take(200).collect::<Vec<_>>().into_iter().rev().collect();
            emit("automation-event", &json!({
                "type": if code == 0 { "step_done" } else { "step_fail" },
                "runId": run_id, "blockId": node_id, "name": node_label,
                "exitCode": code, "stdoutTail": tail, "vars": ctx.vars,
                "step": ctx.step, "total": total, "ts": now_ts()
            }));
            let act = get_str(&config, "action").to_string();
            let detail = if act.is_empty() { format!("{} 执行", pk) } else { format!("{} {}", pk, act) };
            log_step(run_id, emit, steps, node_id, name, &node_label, ctx.step, if code == 0 { "ok" } else { "fail" }, started_at, Some(code), &tail, &detail, &ctx.cwd, &auth, None, None);
            if code != 0 {
                return Err(format!("{} 失败，退出码 {}", node_label, code));
            }
        }
        "condition" => {
            let left = interpolate(get_str(&config, "left"), ctx);
            let right = interpolate(get_str(&config, "right"), ctx);
            let op = get_str(&config, "op").to_string();
            let yes = eval_condition(&left, &op, &right);
            branch = Some(if yes { "out-true" } else { "out-false" }.to_string());
            ctx.last = Some(LastResult { exit_code: if yes { 0 } else { 1 }, stdout: format!("条件 {} {} {} = {}", left, op, right, yes), stderr: String::new() });
            emit("automation-event", &json!({
                "type": "step_done", "runId": run_id, "blockId": node_id, "name": node_label,
                "exitCode": if yes { 0 } else { 1 }, "vars": ctx.vars,
                "step": ctx.step, "total": total, "ts": now_ts()
            }));
            let detail = format!("{} {} {} = {}", left, op, right, yes);
            log_step(run_id, emit, steps, node_id, name, &node_label, ctx.step, "ok", started_at, Some(if yes { 0 } else { 1 }), "", &detail, &ctx.cwd, &auth, None, None);
        }
        "delay" => {
            let secs = get_num_str(&config, "seconds").parse::<u64>().unwrap_or(0);
            wait_ms(secs * 1000, cancel)?;
            ctx.last = Some(LastResult { exit_code: 0, stdout: format!("延迟 {}s", secs), stderr: String::new() });
            emit("automation-event", &json!({
                "type": "step_done", "runId": run_id, "blockId": node_id, "name": node_label,
                "step": ctx.step, "total": total, "ts": now_ts()
            }));
            let detail = format!("等待 {}s", secs);
            log_step(run_id, emit, steps, node_id, name, &node_label, ctx.step, "ok", started_at, Some(0), "", &detail, &ctx.cwd, &auth, None, None);
        }
        "notify" => {
            let title = interpolate(get_str(&config, "title"), ctx);
            let body = interpolate(get_str(&config, "body"), ctx);
            let level = get_str(&config, "level").to_string();
            // 系统通知（统一封装，跨平台）——引擎直接调用，与前端其他模块走同一通道
            if let Some(app) = app {
                system_notify(app, &title, &body);
            }
            ctx.last = Some(LastResult { exit_code: 0, stdout: format!("通知: {}", title), stderr: String::new() });
            emit("automation-event", &json!({
                "type": "step_done", "runId": run_id, "blockId": node_id, "name": node_label,
                "step": ctx.step, "total": total, "ts": now_ts()
            }));
            let detail = format!("[{}] {} — {}", level, title, body);
            log_step(run_id, emit, steps, node_id, name, &node_label, ctx.step, "ok", started_at, Some(0), "", &detail, &ctx.cwd, &auth, None, None);
        }
        "loop" => {
            // 循环（F2）：out 端口 → 循环体；循环体末连回 loop-in 则重复；结束后沿 done 继续
            let body_edges: Vec<&Value> = edges
                .iter()
                .filter(|e| {
                    e.get("fromBlock").and_then(|b| b.as_str()) == Some(node_id)
                        && e.get("fromPort").and_then(|p| p.as_str()) == Some("out")
                })
                .collect();
            let mode = get_str(&config, "mode");
            let max_iter = 100_000usize; // 防死循环保护
            let mut iter = 0usize;
            loop {
                check_cancel(cancel)?;
                let more = if mode == "while" {
                    let left = interpolate(get_str(&config, "left"), ctx);
                    let right = interpolate(get_str(&config, "right"), ctx);
                    let op = get_str(&config, "op").to_string();
                    eval_condition(&left, &op, &right)
                } else {
                    let count = interpolate(&get_num_str(&config, "count"), ctx).parse::<usize>().unwrap_or(0);
                    iter < count
                };
                if !more || iter >= max_iter {
                    break;
                }
                emit("automation-event", &json!({
                    "type": "loop_iter", "runId": run_id, "blockId": node_id, "name": node_label,
                    "iteration": iter, "step": ctx.step, "total": total, "ts": now_ts()
                }));
                let mut body_visited: HashSet<String> = HashSet::new();
                for e in &body_edges {
                    if let Some(to) = e.get("toBlock").and_then(|t| t.as_str()) {
                        walk(run_id, nodes, edges, to, ctx, &mut body_visited, total, cancel, emit, steps, ai, app, depth)?;
                    }
                }
                iter += 1;
            }
            ctx.last = Some(LastResult { exit_code: 0, stdout: format!("循环执行 {} 次", iter), stderr: String::new() });
            emit("automation-event", &json!({
                "type": "step_done", "runId": run_id, "blockId": node_id, "name": node_label,
                "exitCode": 0, "step": ctx.step, "total": total, "ts": now_ts()
            }));
            let detail = format!("循环 {} 次（{}）", iter, mode);
            log_step(run_id, emit, steps, node_id, name, &node_label, ctx.step, "ok", started_at, Some(0), "", &detail, &ctx.cwd, &auth, Some(iter), None);
            branch = Some("done".to_string());
        }
        "parallel" => {
            // 并行组（F2）：branch 多出边=多分支并发；全部完成后沿 join 汇合继续
            let branch_edges: Vec<&Value> = edges
                .iter()
                .filter(|e| {
                    e.get("fromBlock").and_then(|b| b.as_str()) == Some(node_id)
                        && e.get("fromPort").and_then(|p| p.as_str()) == Some("branch")
                })
                .collect();
            if branch_edges.len() == 1 {
                let mut bv: HashSet<String> = HashSet::new();
                if let Some(to) = branch_edges[0].get("toBlock").and_then(|t| t.as_str()) {
                    walk(run_id, nodes, edges, to, ctx, &mut bv, total, cancel, emit, steps, ai, app, depth)?;
                }
            } else if !branch_edges.is_empty() {
                // 并发：每个分支独立 Ctx 副本 + 独立步骤列表，线程执行；全部完成后汇合
                let mut results: Vec<std::thread::Result<Result<Vec<StepLog>, String>>> = Vec::new();
                std::thread::scope(|s| {
                    let mut handles = Vec::new();
                    for e in &branch_edges {
                        let to = e.get("toBlock").and_then(|t| t.as_str()).unwrap_or("").to_string();
                        let mut bctx = ctx.fork();
                        let cancel = Arc::clone(cancel);
                        handles.push(s.spawn(move || -> Result<Vec<StepLog>, String> {
                            let mut bv: HashSet<String> = HashSet::new();
                            let mut bsteps: Vec<StepLog> = Vec::new();
                            walk(run_id, nodes, edges, &to, &mut bctx, &mut bv, total, &cancel, emit, &mut bsteps, ai, app, depth)?;
                            Ok(bsteps)
                        }));
                    }
                    for h in handles {
                        results.push(h.join());
                    }
                });
                for r in results {
                    match r {
                        Ok(Ok(mut bsteps)) => steps.append(&mut bsteps),
                        Ok(Err(e)) => return Err(e),
                        Err(_) => return Err("并行分支线程异常".into()),
                    }
                }
            }
            ctx.last = Some(LastResult { exit_code: 0, stdout: format!("并行 {} 分支完成", branch_edges.len()), stderr: String::new() });
            emit("automation-event", &json!({
                "type": "step_done", "runId": run_id, "blockId": node_id, "name": node_label,
                "exitCode": 0, "step": ctx.step, "total": total, "ts": now_ts()
            }));
            let detail = format!("并行 {} 个分支", branch_edges.len());
            log_step(run_id, emit, steps, node_id, name, &node_label, ctx.step, "ok", started_at, Some(0), "", &detail, &ctx.cwd, &auth, None, None);
            branch = Some("join".to_string());
        }
        "var-set" => {
            let var_name = interpolate(get_str(&config, "varName"), ctx);
            let raw_value = get_str(&config, "value").to_string();
            let value = if raw_value.is_empty() { Value::Null } else { Value::String(interpolate(&raw_value, ctx)) };
            if !var_name.is_empty() {
                ctx.vars.insert(var_name.clone(), value.clone());
            }
            ctx.last = Some(LastResult { exit_code: 0, stdout: format!("{} = {}", var_name, value), stderr: String::new() });
            emit("automation-event", &json!({
                "type": "var_change", "runId": run_id, "blockId": node_id, "name": node_label,
                "vars": ctx.vars, "step": ctx.step, "total": total, "ts": now_ts()
            }));
            let detail = format!("{} = {}", var_name, value);
            log_step(run_id, emit, steps, node_id, name, &node_label, ctx.step, "ok", started_at, Some(0), "", &detail, &ctx.cwd, &auth, None, None);
        }
        "call-automation" => {
            // 调用工作流（子自动化）：运行时把另一个工作积木作为子程序执行；v1 共享父 ctx（变量/工作目录/环境/上一步输出继承并写回）
            let target_id = interpolate(get_str(&config, "automationId"), ctx);
            if target_id.is_empty() {
                return Err(format!("调用工作流块 {}：缺少目标自动化 ID（可在列表/编辑器右键「复制 ID」获取）", node_id));
            }
            if depth >= MAX_CALL_DEPTH {
                return Err(format!("调用工作流嵌套过深（超过 {} 层），可能存在循环调用 A→B→A", MAX_CALL_DEPTH));
            }
            let entry_cfg = get_str(&config, "entry").to_string();
            let (_sub_name, sub_nodes, sub_edges, sub_start, _) = resolve_automation(&target_id, if entry_cfg.is_empty() { None } else { Some(&entry_cfg) })?;
            let sub_total = sub_nodes.len();
            // 子图独立 visited（本块可被多次调用；防环由 depth 限制）；子图日志并入父 RunLog
            let mut sub_visited: HashSet<String> = HashSet::new();
            walk(run_id, &sub_nodes, &sub_edges, &sub_start, ctx, &mut sub_visited, sub_total, cancel, emit, steps, ai, app, depth + 1)?;
            emit("automation-event", &json!({
                "type": "step_done", "runId": run_id, "blockId": node_id, "name": node_label,
                "exitCode": 0, "step": ctx.step, "total": total, "ts": now_ts()
            }));
            let detail = format!("调用工作流 {}（{} 块）", target_id, sub_nodes.len());
            log_step(run_id, emit, steps, node_id, name, &node_label, ctx.step, "ok", started_at, Some(0), "", &detail, &ctx.cwd, &auth, None, None);
        }
        "ai-generate" => {
            let prompt = interpolate(get_str(&config, "prompt"), ctx);
            let var_name = interpolate(get_str(&config, "varName"), ctx);
            if prompt.is_empty() {
                return Err(format!("AI 块 {}：缺少需求描述", node_id));
            }
            let ai = ai.ok_or("AI 未配置（请先在「设置 → AI」中配置模型）")?;
            if !ai.is_configured() {
                return Err("AI 未配置：请先在「设置 → AI」中配置模型（当前为本地 Mock，不会生成真实内容）".into());
            }
            let system = "你是一个自动化流程助手。请根据用户的描述生成精确的结果文本，直接输出结果本身，不要解释，不要使用 Markdown 代码块包裹，不要输出任何多余内容。";
            let last_out = ctx.last.as_ref().map(|l| l.stdout.clone()).unwrap_or_default();
            let user = format!("需求：{}\n\n当前工作目录：{}\n上一块输出：\n{}", prompt, ctx.cwd, last_out);
            let mgr = ai.clone();
            let text = std::thread::scope(|s| {
                s.spawn(move || {
                    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
                    rt.block_on(mgr.generate_text(system, &user))
                })
                .join()
                .map_err(|_| "AI 线程异常".to_string())?
            })
            .map_err(|e| format!("AI 调用失败: {}", e))?;
            let text = text.trim().to_string();
            if !var_name.is_empty() {
                ctx.vars.insert(var_name.clone(), Value::String(text.clone()));
            }
            ctx.last = Some(LastResult { exit_code: 0, stdout: text.clone(), stderr: String::new() });
            emit("automation-event", &json!({
                "type": "var_change", "runId": run_id, "blockId": node_id, "name": node_label,
                "vars": ctx.vars, "step": ctx.step, "total": total, "ts": now_ts()
            }));
            log_step(run_id, emit, steps, node_id, name, &node_label, ctx.step, "ok", started_at, Some(0), "", &text, &ctx.cwd, &auth, None, None);
        }
        _ => {
            ctx.last = Some(LastResult { exit_code: 0, stdout: String::new(), stderr: String::new() });
            emit("automation-event", &json!({
                "type": "step_done", "runId": run_id, "blockId": node_id, "name": node_label,
                "step": ctx.step, "total": total, "ts": now_ts()
            }));
            log_step(run_id, emit, steps, node_id, name, &node_label, ctx.step, "ok", started_at, Some(0), "", "", &ctx.cwd, &auth, None, None);
        }
    }

    // 找下一跳：condition/loop/parallel 用分支端口，其余用 fromPort == 'out'
    let target_port = branch.as_deref().unwrap_or("out");
    let next: Vec<&Value> = edges
        .iter()
        .filter(|e| {
            e.get("fromBlock").and_then(|b| b.as_str()) == Some(node_id)
                && e.get("fromPort").and_then(|p| p.as_str()) == Some(target_port)
        })
        .collect();
    // 多出边（扇出）：顺序执行每个分支；循环回边（loop-in）不进入（由 loop 块驱动下一轮）
    for e in next {
        if let Some(to) = e.get("toBlock").and_then(|t| t.as_str()) {
            if is_loop_back(nodes, to, e.get("toPort").and_then(|p| p.as_str())) {
                continue;
            }
            walk(run_id, nodes, edges, to, ctx, visited, total, cancel, emit, steps, ai, app, depth)?;
        }
    }
    Ok(())
}

// ── 单元测试（headless 引擎语义，不依赖 Tauri AppHandle）──
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn mk_ctx(vars: HashMap<String, Value>, last: Option<LastResult>) -> Ctx {
        Ctx { vars, last, step: 0, cwd: String::new(), envs: HashMap::new() }
    }

    #[test]
    fn test_eval_condition_numeric() {
        assert!(eval_condition("10", ">", "5"));
        assert!(!eval_condition("10", "<", "5"));
        assert!(eval_condition("10", "==", "10"));
        assert!(eval_condition("10", "!=", "11"));
    }

    #[test]
    fn test_eval_condition_string_and_contains() {
        assert!(eval_condition("abc", "==", "abc"));
        assert!(eval_condition("hello world", "contains", "world"));
        assert!(!eval_condition("hello world", "contains", "xyz"));
        // 数字型字符串按数值比较（"007" == "7"）
        assert!(eval_condition("007", "==", "7"));
    }

    #[test]
    fn test_interpolate_vars_and_last() {
        let mut vars = HashMap::new();
        vars.insert("BRANCH".into(), json!("main"));
        let last = Some(LastResult { exit_code: 0, stdout: "build ok".into(), stderr: String::new() });
        let ctx = mk_ctx(vars, last);
        assert_eq!(interpolate("git checkout {{BRANCH}}", &ctx), "git checkout main");
        assert_eq!(interpolate("out={{last.stdout}}", &ctx), "out=build ok");
        assert_eq!(interpolate("code={{last.exitCode}}", &ctx), "code=0");
        // 未匹配占位符原样保留
        assert_eq!(interpolate("{{unknown}}", &ctx), "{{unknown}}");
    }

    /// 收集事件的测试闭包（no-op 之外的验证用）
    fn count_event(evs: &[(String, Value)], typ: &str, id: Option<&str>) -> usize {
        evs.iter()
            .filter(|(n, p)| {
                if n != "automation-event" {
                    return false;
                }
                if p.get("type").and_then(|t| t.as_str()) != Some(typ) {
                    return false;
                }
                match id {
                    Some(i) => p.get("blockId").and_then(|b| b.as_str()) == Some(i),
                    None => true,
                }
            })
            .count()
    }

    #[test]
    fn test_loop_for_runs_body_count_times() {
        let nodes = json!([
            { "id": "loop", "type": "loop", "config": { "mode": "for", "count": 3 } },
            { "id": "body", "type": "var-set", "config": { "varName": "ITER", "value": "x" } },
            { "id": "end", "type": "end" },
        ]);
        let nodes = nodes.as_array().unwrap().clone();
        let edges = json!([
            { "fromBlock": "loop", "fromPort": "out", "toBlock": "body", "toPort": "in" },
            { "fromBlock": "body", "fromPort": "out", "toBlock": "loop", "toPort": "loop-in" },
            { "fromBlock": "loop", "fromPort": "done", "toBlock": "end", "toPort": "in" },
        ]);
        let edges = edges.as_array().unwrap().clone();
        let events: Arc<StdMutex<Vec<(String, Value)>>> = Arc::new(StdMutex::new(Vec::new()));
        let ev = Arc::clone(&events);
        let emit: Emit = &move |name: &str, payload: &Value| {
            ev.lock().unwrap().push((name.to_string(), payload.clone()));
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let mut ctx = mk_ctx(HashMap::new(), None);
        let mut visited = HashSet::new();
        walk("t", &nodes, &edges, "loop", &mut ctx, &mut visited, nodes.len(), &cancel, emit, &mut Vec::new(), None, None, 0).unwrap();
        let evs = events.lock().unwrap();
        // 3 轮迭代 + 循环体 var-set 执行 3 次 + 循环结束后 end 执行 1 次
        assert_eq!(count_event(&evs, "loop_iter", Some("loop")), 3);
        assert_eq!(count_event(&evs, "var_change", Some("body")), 3);
        assert_eq!(count_event(&evs, "step_start", Some("end")), 1);
    }

    #[test]
    fn test_loop_while_stops_when_condition_false() {
        let nodes = json!([
            { "id": "loop", "type": "loop", "config": { "mode": "while", "left": "{{I}}", "op": "==", "right": "5" } },
            { "id": "body", "type": "var-set", "config": { "varName": "I", "value": "0" } },
            { "id": "end", "type": "end" },
        ]);
        let nodes = nodes.as_array().unwrap().clone();
        let edges = json!([
            { "fromBlock": "loop", "fromPort": "out", "toBlock": "body", "toPort": "in" },
            { "fromBlock": "body", "fromPort": "out", "toBlock": "loop", "toPort": "loop-in" },
            { "fromBlock": "loop", "fromPort": "done", "toBlock": "end", "toPort": "in" },
        ]);
        let edges = edges.as_array().unwrap().clone();
        let events: Arc<StdMutex<Vec<(String, Value)>>> = Arc::new(StdMutex::new(Vec::new()));
        let ev = Arc::clone(&events);
        let emit: Emit = &move |name: &str, payload: &Value| {
            ev.lock().unwrap().push((name.to_string(), payload.clone()));
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let mut vars = HashMap::new();
        vars.insert("I".into(), json!("5"));
        let mut ctx = mk_ctx(vars, None);
        let mut visited = HashSet::new();
        walk("t", &nodes, &edges, "loop", &mut ctx, &mut visited, nodes.len(), &cancel, emit, &mut Vec::new(), None, None, 0).unwrap();
        let evs = events.lock().unwrap();
        // 第一轮 true → 循环体把 I 置 0 → 第二轮条件 false 退出 → 只执行 1 轮
        assert_eq!(count_event(&evs, "loop_iter", Some("loop")), 1);
        assert_eq!(count_event(&evs, "step_start", Some("end")), 1);
    }

    #[test]
    fn test_parallel_runs_all_branches_then_join() {
        let nodes = json!([
            { "id": "par", "type": "parallel" },
            { "id": "a", "type": "var-set", "config": { "varName": "A", "value": "1" } },
            { "id": "b", "type": "var-set", "config": { "varName": "B", "value": "2" } },
            { "id": "end", "type": "end" },
        ]);
        let nodes = nodes.as_array().unwrap().clone();
        let edges = json!([
            { "fromBlock": "par", "fromPort": "branch", "toBlock": "a", "toPort": "in" },
            { "fromBlock": "par", "fromPort": "branch", "toBlock": "b", "toPort": "in" },
            { "fromBlock": "par", "fromPort": "join", "toBlock": "end", "toPort": "in" },
        ]);
        let edges = edges.as_array().unwrap().clone();
        let events: Arc<StdMutex<Vec<(String, Value)>>> = Arc::new(StdMutex::new(Vec::new()));
        let ev = Arc::clone(&events);
        let emit: Emit = &move |name: &str, payload: &Value| {
            ev.lock().unwrap().push((name.to_string(), payload.clone()));
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let mut ctx = mk_ctx(HashMap::new(), None);
        let mut visited = HashSet::new();
        walk("t", &nodes, &edges, "par", &mut ctx, &mut visited, nodes.len(), &cancel, emit, &mut Vec::new(), None, None, 0).unwrap();
        let evs = events.lock().unwrap();
        // 两个分支各执行 1 次；汇合后 end 执行 1 次
        assert_eq!(count_event(&evs, "step_start", Some("a")), 1);
        assert_eq!(count_event(&evs, "step_start", Some("b")), 1);
        assert_eq!(count_event(&evs, "step_start", Some("end")), 1);
    }

    #[test]
    fn test_stop_cancels_running_walk() {
        let nodes = json!([
            { "id": "d", "type": "delay", "config": { "seconds": 60 } },
        ]);
        let nodes = nodes.as_array().unwrap().clone();
        let edges: Vec<Value> = Vec::new();
        let events: Arc<StdMutex<Vec<(String, Value)>>> = Arc::new(StdMutex::new(Vec::new()));
        let ev = Arc::clone(&events);
        let emit: Emit = &move |name: &str, payload: &Value| {
            ev.lock().unwrap().push((name.to_string(), payload.clone()));
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let mut ctx = mk_ctx(HashMap::new(), None);
        let mut visited = HashSet::new();
        let cancel2 = Arc::clone(&cancel);
        std::thread::scope(|s| {
            let h = s.spawn(move || {
                walk("t", &nodes, &edges, "d", &mut ctx, &mut visited, nodes.len(), &cancel2, emit, &mut Vec::new(), None, None, 0)
            });
            std::thread::sleep(Duration::from_millis(200));
            cancel.store(true, Ordering::SeqCst);
            let res = h.join().unwrap();
            assert!(res.is_err());
            assert_eq!(res.unwrap_err(), STOPPED_ERR);
        });
    }

    #[test]
    fn test_resolve_automation_from_list_finds_entry_and_vars() {
        let list = json!([
            {
                "id": "a1", "name": "子流程",
                "variables": { "X": { "type": "string", "value": "hi" } },
                "nodes": [
                    { "id": "start1", "type": "start" },
                    { "id": "m1", "type": "manual-trigger" },
                    { "id": "e1", "type": "end" },
                ],
                "edges": [],
            }
        ]);
        let list = list.as_array().unwrap().clone();
        // 默认入口：start
        let (name0, nodes, _, start_id, vars) = resolve_automation_from_list(&list, "a1", None).unwrap();
        assert_eq!(name0, "子流程");
        assert_eq!(start_id, "start1");
        assert_eq!(nodes.len(), 3);
        assert_eq!(vars.get("X").and_then(|v| v.as_str()), Some("hi"));
        // 显式 entry 优先（手动触发）
        let (_, _, _, start_id2, _) = resolve_automation_from_list(&list, "a1", Some("m1")).unwrap();
        assert_eq!(start_id2, "m1");
        // 找不到 id → 报错
        assert!(resolve_automation_from_list(&list, "nope", None).is_err());
    }

    #[test]
    fn test_call_automation_depth_guard_prevents_loop() {
        // call-automation 块在 depth 已达上限时，在读取磁盘前就返回「嵌套过深」——防 A→B→A 循环调用
        let nodes = json!([
            { "id": "call", "type": "call-automation", "config": { "automationId": "self" } },
        ]);
        let nodes = nodes.as_array().unwrap().clone();
        let edges: Vec<Value> = Vec::new();
        let events: Arc<StdMutex<Vec<(String, Value)>>> = Arc::new(StdMutex::new(Vec::new()));
        let ev = Arc::clone(&events);
        let emit: Emit = &move |name: &str, payload: &Value| {
            ev.lock().unwrap().push((name.to_string(), payload.clone()));
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let mut ctx = mk_ctx(HashMap::new(), None);
        let mut visited = HashSet::new();
        let err = walk("t", &nodes, &edges, "call", &mut ctx, &mut visited, nodes.len(), &cancel, emit, &mut Vec::new(), None, None, MAX_CALL_DEPTH).unwrap_err();
        assert!(err.contains("嵌套过深"), "got: {}", err);
    }
}

