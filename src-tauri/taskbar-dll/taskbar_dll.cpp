// jc9_taskbar.dll - ITaskbarList3 COM wrapper
// Uses plain GDI (no GDI+) for overlay icon generation

#define JC9_TASKBAR_EXPORTS
#include "taskbar_dll.h"
#include <shobjidl.h>
#include <objbase.h>
#include <vector>
#include <string>

#pragma comment(lib, "gdi32.lib")

// Fallback defines for older SDKs
#ifndef THB_ICON
#define THB_ICON 0x1
#endif
#ifndef THB_TIP
#define THB_TIP 0x2
#endif
#ifndef THB_FLAGS
#define THB_FLAGS 0x4
#endif

// State
static ITaskbarList3* g_taskbar = nullptr;
static HWND g_hwnd = nullptr;

struct ButtonInfo {
    int id;
    std::wstring tip;
    bool enabled;
    bool hidden;
};
static std::vector<ButtonInfo> g_buttons;

// Draw a simple circular count badge using plain GDI
static HICON create_count_icon(int count, int size) {
    HDC screen = GetDC(nullptr);
    HDC mem = CreateCompatibleDC(screen);

    BITMAPINFO bi = {0};
    bi.bmiHeader.biSize = sizeof(BITMAPINFOHEADER);
    bi.bmiHeader.biWidth = size;
    bi.bmiHeader.biHeight = size;
    bi.bmiHeader.biPlanes = 1;
    bi.bmiHeader.biBitCount = 32;
    bi.bmiHeader.biCompression = BI_RGB;

    void* bits = nullptr;
    HBITMAP hBmp = CreateDIBSection(mem, &bi, DIB_RGB_COLORS, &bits, nullptr, 0);
    HBITMAP oldBmp = (HBITMAP)SelectObject(mem, hBmp);

    // Fill transparent (alpha = 0)
    if (bits) memset(bits, 0, size * size * 4);

    // Red circle
    HBRUSH red = CreateSolidBrush(RGB(229, 57, 53));
    HBRUSH oldBr = (HBRUSH)SelectObject(mem, red);
    Ellipse(mem, 2, 2, size - 2, size - 2);
    SelectObject(mem, oldBr);
    DeleteObject(red);

    // White text
    wchar_t text[8] = {0};
    if (count < 0) wcscpy_s(text, L"...");
    else if (count == 0) text[0] = 0;
    else if (count > 99) wcscpy_s(text, L"99+");
    else _itow_s(count, text, 10);

    if (text[0]) {
        SetBkMode(mem, TRANSPARENT);
        SetTextColor(mem, RGB(255, 255, 255));

        // Basic font
        HFONT font = CreateFontW(
            count > 99 ? 11 : 14, 0, 0, 0, FW_BOLD,
            FALSE, FALSE, FALSE, DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS,
            DEFAULT_QUALITY, DEFAULT_PITCH, L"Segoe UI");
        HFONT oldFont = (HFONT)SelectObject(mem, font);

        SIZE sz;
        GetTextExtentPoint32W(mem, text, (int)wcslen(text), &sz);
        int x = (size - sz.cx) / 2;
        int y = (size - sz.cy) / 2;
        TextOutW(mem, x, y, text, (int)wcslen(text));

        SelectObject(mem, oldFont);
        DeleteObject(font);
    }

    SelectObject(mem, oldBmp);

    // Create icon
    ICONINFO ii = {0};
    ii.fIcon = TRUE;
    ii.hbmColor = hBmp;
    ii.hbmMask = CreateBitmap(size, size, 1, 1, nullptr);
    HICON hIcon = CreateIconIndirect(&ii);

    DeleteObject(ii.hbmMask);
    DeleteObject(hBmp);
    DeleteDC(mem);
    ReleaseDC(nullptr, screen);
    return hIcon;
}

// Refresh all buttons
static HRESULT refresh_buttons() {
    if (!g_taskbar || !g_hwnd) return E_FAIL;
    g_taskbar->ThumbBarUpdateButtons(g_hwnd, 0, nullptr);
    if (g_buttons.empty()) return S_OK;

    std::vector<THUMBBUTTON> btns;
    for (auto& b : g_buttons) {
        THUMBBUTTON tb = {0};
        tb.iId = b.id;
        tb.dwMask = (THUMBBUTTONMASK)(THB_ICON | THB_TIP | THB_FLAGS);
        tb.dwFlags = (THUMBBUTTONFLAGS)(0);
        if (b.hidden) tb.dwFlags = (THUMBBUTTONFLAGS)((int)tb.dwFlags | THBF_HIDDEN);
        if (!b.enabled) tb.dwFlags = (THUMBBUTTONFLAGS)((int)tb.dwFlags | THBF_DISABLED);
        wcsncpy_s(tb.szTip, b.tip.c_str(), 260);
        btns.push_back(tb);
    }
    return g_taskbar->ThumbBarAddButtons(g_hwnd, (UINT)btns.size(), btns.data());
}

// === Exports ===

JC9_TASKBAR_API int jc9_taskbar_init(HWND hWnd) {
    if (g_taskbar) return 0;
    SetCurrentProcessExplicitAppUserModelID(L"com.jc9.app");

    HRESULT hr = CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
    if (FAILED(hr)) return 1;

    hr = CoCreateInstance(CLSID_TaskbarList, nullptr, CLSCTX_INPROC_SERVER,
        IID_ITaskbarList3, (void**)&g_taskbar);
    if (FAILED(hr) || !g_taskbar) return 2;

    hr = g_taskbar->HrInit();
    if (FAILED(hr)) return 3;

    hr = g_taskbar->AddTab(hWnd);
    if (FAILED(hr)) return 4;

    g_hwnd = hWnd;
    return 0;
}

JC9_TASKBAR_API int jc9_taskbar_add_button(int id, const wchar_t* tip) {
    g_buttons.push_back({id, tip ? tip : L"", true, false});
    return SUCCEEDED(refresh_buttons()) ? 0 : 1;
}

JC9_TASKBAR_API int jc9_taskbar_update_button(int id, const wchar_t* tip, int enabled, int hidden) {
    for (auto& b : g_buttons) {
        if (b.id == id) {
            if (tip) b.tip = tip;
            b.enabled = (enabled != 0);
            b.hidden = (hidden != 0);
            return SUCCEEDED(refresh_buttons()) ? 0 : 1;
        }
    }
    return 2;
}

JC9_TASKBAR_API int jc9_taskbar_clear_buttons() {
    g_buttons.clear();
    return SUCCEEDED(refresh_buttons()) ? 0 : 1;
}

JC9_TASKBAR_API int jc9_taskbar_set_overlay(int count, const wchar_t* description) {
    if (!g_taskbar || !g_hwnd) return 1;
    if (count == 0) {
        g_taskbar->SetOverlayIcon(g_hwnd, nullptr, nullptr);
        return 0;
    }
    HICON hIcon = create_count_icon(count, 24);
    if (!hIcon) return 2;
    HRESULT hr = g_taskbar->SetOverlayIcon(g_hwnd, hIcon, description);
    DestroyIcon(hIcon);
    return SUCCEEDED(hr) ? 0 : 3;
}

JC9_TASKBAR_API int jc9_taskbar_set_progress(ULONGLONG completed, ULONGLONG total) {
    if (!g_taskbar || !g_hwnd) return 1;
    if (total == 0 && completed == 0) {
        g_taskbar->SetProgressState(g_hwnd, TBPF_NOPROGRESS);
        return 0;
    }
    if (total == 0) {
        g_taskbar->SetProgressState(g_hwnd, TBPF_INDETERMINATE);
        return 0;
    }
    HRESULT hr = g_taskbar->SetProgressValue(g_hwnd, completed, total);
    if (SUCCEEDED(hr)) g_taskbar->SetProgressState(g_hwnd, TBPF_NORMAL);
    return SUCCEEDED(hr) ? 0 : 2;
}

JC9_TASKBAR_API void jc9_taskbar_cleanup() {
    if (g_taskbar) {
        if (g_hwnd) g_taskbar->DeleteTab(g_hwnd);
        g_taskbar->Release();
        g_taskbar = nullptr;
        g_hwnd = nullptr;
    }
    g_buttons.clear();
    CoUninitialize();
}
