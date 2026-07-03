//! Windows 任务栏集成 — Rust FFI 绑定 (完整版)
//!
//! 运行时加载 `jc9_taskbar.dll`，封装所有 ITaskbarList3 功能：
//! - 缩略图按钮（添加/更新/清除）
//! - 角标（OverlayIcon — 圆形计数徽章）
//! - 进度条（确定/不确定/清除）
//!
//! DLL 职责：ITaskbarList3 COM + GDI+ 图标
//! Tauri 职责：前端 UI、事件处理、秒级刷新逻辑

#![cfg(target_os = "windows")]

use std::sync::Mutex;
use std::path::PathBuf;

// ── C 函数指针类型 ──

type FnInit          = unsafe extern "C" fn(isize) -> i32;
type FnAddButton     = unsafe extern "C" fn(i32, *const u16) -> i32;
type FnUpdateButton  = unsafe extern "C" fn(i32, *const u16, i32, i32) -> i32;
type FnClearButtons  = unsafe extern "C" fn() -> i32;
type FnSetOverlay    = unsafe extern "C" fn(i32, *const u16) -> i32;
type FnSetProgress   = unsafe extern "C" fn(u64, u64) -> i32;
type FnCleanup       = unsafe extern "C" fn();

// ── 内部状态 ──

struct TaskbarDll {
    init:          FnInit,
    add_button:    FnAddButton,
    update_button: FnUpdateButton,
    clear_buttons: FnClearButtons,
    set_overlay:   FnSetOverlay,
    set_progress:  FnSetProgress,
    cleanup:       FnCleanup,
}

static TASKBAR: Mutex<Option<TaskbarDll>> = Mutex::new(None);
static mut DLL_LIB: Option<libloading::Library> = None;

fn find_dll() -> Option<PathBuf> {
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();

    let mut search = vec![
        exe_dir.join("jc9_taskbar.dll"),
        exe_dir.join("target").join("debug").join("jc9_taskbar.dll"),
        exe_dir.join("target").join("release").join("jc9_taskbar.dll"),
    ];

    let mut p = exe_dir.clone();
    for _ in 0..5 { if !p.pop() { break; } }
    p.push("src-tauri");
    p.push("target");
    p.push("debug");
    p.push("jc9_taskbar.dll");
    search.push(p);

    for path in &search {
        if path.exists() { return Some(path.clone()); }
    }
    None
}

// ── 辅助：Rust str → UTF-16 Vec ──

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// ── 公共 API ──

/// 初始化：加载 DLL 并调用 jc9_taskbar_init(hwnd)
pub fn init(hwnd_raw: isize) -> Result<(), String> {
    let dll_path = find_dll()
        .ok_or("找不到 jc9_taskbar.dll\n请先运行: cd src-tauri/taskbar-dll && .\\build.ps1")?;

    unsafe {
        let lib = libloading::Library::new(&dll_path)
            .map_err(|e| format!("加载 DLL 失败: {} ({:?})", e, dll_path))?;

        // 提取原始函数指针（不借用 Library 生命周期）
        macro_rules! load_fn {
            ($name:expr, $type:ty) => {{
                let sym: libloading::Symbol<$type> = lib.get($name)
                    .map_err(|e| format!("符号 {} ({:?}): {}", stringify!($name), $name, e))?;
                *sym
            }};
        }

        let fns = TaskbarDll {
            init:          load_fn!(b"jc9_taskbar_init",           FnInit),
            add_button:    load_fn!(b"jc9_taskbar_add_button",     FnAddButton),
            update_button: load_fn!(b"jc9_taskbar_update_button",  FnUpdateButton),
            clear_buttons: load_fn!(b"jc9_taskbar_clear_buttons",  FnClearButtons),
            set_overlay:   load_fn!(b"jc9_taskbar_set_overlay",    FnSetOverlay),
            set_progress:  load_fn!(b"jc9_taskbar_set_progress",   FnSetProgress),
            cleanup:       load_fn!(b"jc9_taskbar_cleanup",        FnCleanup),
        };

        DLL_LIB = Some(lib);

        let ret = (fns.init)(hwnd_raw);
        if ret != 0 {
            return Err(format!("jc9_taskbar_init 返回错误码 {}", ret));
        }

        *TASKBAR.lock().map_err(|e| e.to_string())? = Some(fns);
        println!("✅ 任务栏 DLL 已加载 (hwnd={:#x})", hwnd_raw);
        Ok(())
    }
}

/// 添加缩略图按钮
pub fn add_button(id: i32, tip: &str) -> Result<(), String> {
    call(|tb| unsafe {
        let w = to_wide(tip);
        check((tb.add_button)(id, w.as_ptr()), "add_button")
    })
}

/// 动态更新按钮文字和状态
/// enabled: false=变灰禁用, hidden: true=隐藏
pub fn update_button(id: i32, tip: &str, enabled: bool, hidden: bool) -> Result<(), String> {
    call(|tb| unsafe {
        let w = to_wide(tip);
        check(
            (tb.update_button)(id, w.as_ptr(), enabled as i32, hidden as i32),
            "update_button",
        )
    })
}

/// 清除所有缩略图按钮
pub fn clear_buttons() -> Result<(), String> {
    call(|tb| unsafe { check((tb.clear_buttons)(), "clear_buttons") })
}

/// 设置角标徽章
/// count: 显示数字 (0=清除, >99→"99+", <0→"⋯")
/// description: 悬浮提示 (可选)
pub fn set_overlay(count: i32, description: Option<&str>) -> Result<(), String> {
    call(|tb| unsafe {
        let desc = description.map(|s| to_wide(s));
        let ptr = desc.as_ref().map(|v| v.as_ptr()).unwrap_or(std::ptr::null());
        check((tb.set_overlay)(count, ptr), "set_overlay")
    })
}

/// 清除角标 (等同于 set_overlay(0, None))
pub fn clear_overlay() -> Result<(), String> {
    set_overlay(0, None)
}

/// 设置任务栏进度条
/// total=0 且 completed>0: 不确定进度（动画横条）
/// total=completed=0: 清除
pub fn set_progress(completed: u64, total: u64) -> Result<(), String> {
    call(|tb| unsafe { check((tb.set_progress)(completed, total), "set_progress") })
}

/// 清除进度条
pub fn clear_progress() -> Result<(), String> {
    set_progress(0, 0)
}

/// 清理 DLL 和 COM
pub fn cleanup() {
    if let Ok(mut guard) = TASKBAR.lock() {
        if let Some(ref tb) = *guard { unsafe { (tb.cleanup)(); } }
        *guard = None;
    }
}

// ── 内部辅助 ──

fn call<F: FnOnce(&TaskbarDll) -> Result<(), String>>(f: F) -> Result<(), String> {
    let guard = TASKBAR.lock().map_err(|e| e.to_string())?;
    let tb = guard.as_ref().ok_or("Taskbar DLL 未加载")?;
    f(tb)
}

fn check(ret: i32, name: &str) -> Result<(), String> {
    if ret != 0 { Err(format!("{} 返回错误码 {}", name, ret)) } else { Ok(()) }
}
