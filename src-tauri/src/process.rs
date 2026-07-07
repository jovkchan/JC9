use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Mutex;
use std::thread;
use tauri::{AppHandle, Emitter};

struct Session {
    child: Box<dyn portable_pty::Child + Send + Sync>,
    writer: Mutex<Option<Box<dyn Write + Send>>>,
    _master: Box<dyn portable_pty::MasterPty + Send>, // keep alive to hold ConPTY handle
}

pub struct ProcessManager {
    sessions: Mutex<HashMap<String, Session>>,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
        }
    }

    pub fn start(
        &self,
        id: String,
        _p: String,
        _c: String,
        wd: String,
        cmd: String,
        app: AppHandle,
    ) -> Result<(), String> {
        eprintln!("[JC9] PTY start id={id} cmd={cmd} wd={wd}");
        {
            let mut s = self.sessions.lock().map_err(|e| e.to_string())?;
            if let Some(mut old) = s.remove(&id) {
                let _ = old.child.kill();
            }
        }

        // Create ConPTY — same API as WezTerm/VS Code use
        let pty = native_pty_system();
        let pair = pty
            .openpty(PtySize {
                rows: 30,
                cols: 120,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| {
                eprintln!("[JC9] openpty FAIL: {e}");
                format!("ConPTY 创建失败: {e}")
            })?;
        eprintln!("[JC9] openpty OK");

        // Build shell command inside PTY
        #[cfg(target_os = "windows")]
        let cb = {
            let mut c = CommandBuilder::new("powershell");
            let full_cmd;
            let args: Vec<&str> = if cmd.is_empty() {
                vec!["-NoLogo", "-NoExit"]
            } else {
                full_cmd = format!("[Console]::OutputEncoding=[Text.Encoding]::UTF8; {}", cmd);
                vec!["-NoLogo", "-NoExit", "-Command", &full_cmd]
            };
            c.args(&args);
            if !wd.is_empty() && Path::new(&wd).exists() {
                c.cwd(&wd);
            }
            c
        };
        #[cfg(not(target_os = "windows"))]
        let cb = {
            let mut c = CommandBuilder::new("bash");
            if !cmd.is_empty() {
                let full_cmd = format!("{}; exec bash", cmd);
                c.args(&["-c", &full_cmd]);
            }
            if !wd.is_empty() && Path::new(&wd).exists() {
                c.cwd(&wd);
            }
            c
        };

        let child = pair.slave.spawn_command(cb).map_err(|e| {
            eprintln!("[JC9] spawn FAIL: {e}");
            format!("PTY 内启动失败: {e}")
        })?;
        eprintln!("[JC9] spawned OK");

        drop(pair.slave);

        // Reader
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| format!("reader 失败: {e}"))?;
        let pid = id.clone();
        let app2 = app.clone();
        thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        eprintln!("[JC9] pty EOF id={pid}");
                        let _ = app2.emit("process-exited", serde_json::json!({"processId":pid}));
                        break;
                    }
                    Ok(n) => {
                        let _ = app2.emit(
                            "pty-output",
                            serde_json::json!({"processId":pid,"data":&buf[..n]}),
                        );
                    }
                    Err(e) => {
                        eprintln!("[JC9] pty read err id={pid}: {e}");
                        let _ = app2.emit("process-exited", serde_json::json!({"processId":pid}));
                        break;
                    }
                }
            }
        });

        // Writer (consumes master's internal handle but NOT the ConPTY — master stays alive)
        let writer: Box<dyn Write + Send> = pair
            .master
            .take_writer()
            .map_err(|e| format!("writer 失败: {e}"))?;

        self.sessions.lock().map_err(|e| e.to_string())?.insert(
            id.clone(),
            Session {
                child,
                _master: pair.master,
                writer: Mutex::new(Some(writer)),
            },
        );
        let _ = app.emit("process-started", serde_json::json!({"processId":id}));
        eprintln!("[JC9] PTY done id={id}");
        Ok(())
    }

    pub fn write(&self, id: &str, data: &[u8]) -> Result<(), String> {
        let s = self.sessions.lock().map_err(|e| e.to_string())?;
        if let Some(session) = s.get(id) {
            let mut w = session.writer.lock().map_err(|e| e.to_string())?;
            if let Some(ref mut writer) = *w {
                writer.write_all(data).map_err(|e| format!("write: {e}"))?;
                writer.flush().map_err(|e| format!("flush: {e}"))?;
            }
        }
        Ok(())
    }
    pub fn resize(&self, id: &str, rows: u16, cols: u16) -> Result<(), String> {
        let s = self.sessions.lock().map_err(|e| e.to_string())?;
        if let Some(session) = s.get(id) {
            session
                ._master
                .resize(PtySize {
                    rows,
                    cols,
                    pixel_width: 0,
                    pixel_height: 0,
                })
                .map_err(|e| format!("重设 PTY 尺寸失败: {e}"))?;
        }
        Ok(())
    }
    pub fn stop(&self, id: &str) -> Result<(), String> {
        let mut s = self.sessions.lock().map_err(|e| e.to_string())?;
        if let Some(mut session) = s.remove(id) {
            let _ = session.child.kill();
            let _ = session.child.wait();
        }
        Ok(())
    }
    pub fn stop_all_for_command(&self, p: &str, c: &str) -> Result<(), String> {
        self.stop(&format!("{}::{}", p, c))
    }
    pub fn is_running(&self, id: &str) -> bool {
        self.sessions
            .lock()
            .map(|s| s.contains_key(id))
            .unwrap_or(false)
    }

    /// 执行命令并流式输出（命令完成后进程自动退出）
    #[allow(dead_code)]
    pub fn execute(
        &self,
        id: String,
        wd: String,
        cmd: String,
        app: AppHandle,
    ) -> Result<(), String> {
        eprintln!("[JC9] PTY execute id={id} cmd={cmd} wd={wd}");

        let pty = native_pty_system();
        let pair = pty
            .openpty(PtySize { rows: 30, cols: 120, pixel_width: 0, pixel_height: 0 })
            .map_err(|e| format!("ConPTY 创建失败: {e}"))?;

        #[cfg(target_os = "windows")]
        let cb = {
            let mut c = CommandBuilder::new("powershell");
            let full_cmd = format!("[Console]::OutputEncoding=[Text.Encoding]::UTF8; {}", cmd);
            // 不加 -NoExit，命令完成后进程自动退出
            c.args(&["-NoLogo", "-Command", &full_cmd]);
            if !wd.is_empty() && Path::new(&wd).exists() { c.cwd(&wd); }
            c
        };
        #[cfg(not(target_os = "windows"))]
        let cb = {
            let mut c = CommandBuilder::new("bash");
            c.args(&["-c", &cmd]);
            if !wd.is_empty() && Path::new(&wd).exists() { c.cwd(&wd); }
            c
        };

        let child = pair.slave.spawn_command(cb).map_err(|e| format!("PTY 启动失败: {e}"))?;
        drop(pair.slave);

        let mut reader = pair.master.try_clone_reader().map_err(|e| format!("reader 失败: {e}"))?;
        let pid = id.clone();
        let app2 = app.clone();
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        let _ = app2.emit("process-exited", serde_json::json!({"processId": pid}));
                        break;
                    }
                    Ok(n) => {
                        let _ = app2.emit("pty-output", serde_json::json!({"processId": pid, "data": &buf[..n]}));
                    }
                    Err(_) => {
                        let _ = app2.emit("process-exited", serde_json::json!({"processId": pid}));
                        break;
                    }
                }
            }
        });

        let writer: Box<dyn std::io::Write + Send> = pair.master.take_writer()
            .map_err(|e| format!("writer 失败: {e}"))?;

        self.sessions.lock().map_err(|e| e.to_string())?.insert(id.clone(), Session {
            child, _master: pair.master, writer: Mutex::new(Some(writer)),
        });
        let _ = app.emit("process-started", serde_json::json!({"processId": id}));
        Ok(())
    }
}
