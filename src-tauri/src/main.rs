#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "windows")]
mod windows_setup {
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "shell32")]
    extern "system" {
        fn SetCurrentProcessExplicitAppUserModelID(pcwsz_app_id: *const u16) -> i32;
    }

    #[link(name = "advapi32")]
    extern "system" {
        fn RegSetKeyValueW(
            hkey: isize,
            lp_sub_key: *const u16,
            lp_value_name: *const u16,
            dw_type: u32,
            lp_data: *const u8,
            cb_data: u32,
        ) -> i32;
    }

    const HKEY_CURRENT_USER: isize = -2147483647; // 0x80000001
    const REG_SZ: u32 = 1;
    const APP_ID: &str = "com.jc9.app";
    /// 通知顶部显示的应用名（改成你想要的名称，如「JC9」「我的工具」）
    const DISPLAY_NAME: &str = "JC9";

    fn wide(s: &str) -> Vec<u16> {
        std::ffi::OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    /// 让 Windows Toast 通知显示正确的应用名与图标：
    /// ① 设置进程 AppUserModelID；② 注册 DisplayName；③ 注册 IconUri（exe 内嵌图标）。
    /// 打包安装（NSIS/MSI）后会正式注册，此处在 dev 模式补齐。
    pub fn setup() {
        let app_id = wide(APP_ID);
        unsafe {
            SetCurrentProcessExplicitAppUserModelID(app_id.as_ptr());

            let sub_key = wide("Software\\Classes\\AppUserModelId\\com.jc9.app");

            // ① 应用名（通知顶部显示的名称）
            let display_name_v = wide("DisplayName");
            let display_name = wide(DISPLAY_NAME);
            let rc1 = RegSetKeyValueW(
                HKEY_CURRENT_USER,
                sub_key.as_ptr(),
                display_name_v.as_ptr(),
                REG_SZ,
                display_name.as_ptr() as *const u8,
                (display_name.len() * 2) as u32,
            );

            // ② 图标：优先 exe 同目录 icons/128x128.png（独立 PNG，Windows Toast 最稳），
            //    其次 icons/icon.ico（ICO 内嵌 PNG 可能有兼容问题），最后回退 exe 内嵌图标
            let icon_uri = std::env::current_exe()
                .map(|exe| {
                    let dir = exe.parent().unwrap_or(std::path::Path::new("."));
                    let png = dir.join("icons").join("128x128.png");
                    if png.exists() {
                        return png.display().to_string();
                    }
                    let ico = dir.join("icons").join("icon.ico");
                    if ico.exists() {
                        return ico.display().to_string();
                    }
                    format!("{},0", exe.display())
                })
                .unwrap_or_default();
            let icon_uri_v = wide("IconUri");
            let icon_uri_w = wide(&icon_uri);
            let rc2 = RegSetKeyValueW(
                HKEY_CURRENT_USER,
                sub_key.as_ptr(),
                icon_uri_v.as_ptr(),
                REG_SZ,
                icon_uri_w.as_ptr() as *const u8,
                (icon_uri_w.len() * 2) as u32,
            );
            eprintln!(
                "[jc9] AUMID 注册: DisplayName={} (rc={}), IconUri={} (rc={})",
                DISPLAY_NAME, rc1, icon_uri, rc2
            );
        }
    }
}

fn main() {
    // `jc9 mcp` → 以 stdio 方式运行内置 MCP Server（不启动 GUI），
    // 供外部 MCP 客户端（Claude Desktop / VS Code / Cursor 等）接入笔记与记忆
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("mcp") {
        jc9_lib::run_mcp_stdio();
        return;
    }
    #[cfg(target_os = "windows")]
    windows_setup::setup();
    jc9_lib::run()
}
