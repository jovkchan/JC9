// jc9_taskbar.dll - DeskBand Shell Extension
// Proper COM implementation with IDeskBand2 + IObjectWithSite + IPersistStream
// Based on Windows SDK DeskBand sample

#pragma once
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <shlobj.h>
#include <shobjidl.h>

// {A1B2C3D4-E5F6-7890-ABCD-EF1234567890}
DEFINE_GUID(CLSID_JC9DeskBand,
0xa1b2c3d4,0xe5f6,0x7890,0xab,0xcd,0xef,0x12,0x34,0x56,0x78,0x90);

#ifdef JC9_TASKBAR_EXPORTS
#define JC9_API extern "C" __declspec(dllexport)
#else
#define JC9_API extern "C" __declspec(dllimport)
#endif

JC9_API HRESULT jc9_init(HWND tauriHwnd);
JC9_API HRESULT jc9_set_text(const wchar_t* text);
JC9_API void    jc9_cleanup();
