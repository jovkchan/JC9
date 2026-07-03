// jc9_taskbar.dll - Windows Taskbar Integration
// ITaskbarList3 COM wrapper for Tauri FFI

#pragma once
#include <windows.h>

#ifdef JC9_TASKBAR_EXPORTS
    #define JC9_TASKBAR_API extern "C" __declspec(dllexport)
#else
    #define JC9_TASKBAR_API extern "C" __declspec(dllimport)
#endif

JC9_TASKBAR_API int jc9_taskbar_init(HWND hWnd);
JC9_TASKBAR_API int jc9_taskbar_add_button(int id, const wchar_t* tip);
JC9_TASKBAR_API int jc9_taskbar_update_button(int id, const wchar_t* tip, int enabled, int hidden);
JC9_TASKBAR_API int jc9_taskbar_clear_buttons();
JC9_TASKBAR_API int jc9_taskbar_set_overlay(int count, const wchar_t* description);
JC9_TASKBAR_API int jc9_taskbar_set_progress(ULONGLONG completed, ULONGLONG total);
JC9_TASKBAR_API void jc9_taskbar_cleanup();
