// jc9_taskbar.dll - DeskBand Shell Extension
// Implements IDeskBand2 for taskbar embedding

#define JC9_TASKBAR_EXPORTS
#define INITGUID
#include "taskbar_dll.h"
#include <string>
#include <shlwapi.h>
#pragma comment(lib, "shlwapi.lib")

#define BAND_CLASS L"JC9DeskBand_Window"

// Forward refs
class JC9Band;
HRESULT RegisterBand();
void     UnregisterBand();

EXTERN_C IMAGE_DOS_HEADER __ImageBase;

static HWND    g_tauriHwnd = nullptr;
static JC9Band* g_band = nullptr;

// =============================================
// DeskBand COM Object
// =============================================
class JC9Band : public IDeskBand2,
                public IObjectWithSite,
                public IPersistStream,
                public IInputObject
{
    LONG  m_ref;
    HWND  m_hwnd;
    HWND  m_hwndParent;
    IInputObjectSite* m_site;
    BOOL  m_showing;
    DWORD m_bandId;
    DWORD m_viewMode;
    std::wstring m_text;

public:
    JC9Band() : m_ref(1), m_hwnd(nullptr), m_hwndParent(nullptr),
                m_site(nullptr), m_showing(FALSE), m_bandId(0), m_viewMode(0),
                m_text(L"\u26A1 JC9") {}

    ~JC9Band() { if(m_hwnd) DestroyWindow(m_hwnd); if(m_site) m_site->Release(); }

    void SetText(const wchar_t* t) {
        m_text = t ? t : L"";
        if(m_hwnd) InvalidateRect(m_hwnd, nullptr, TRUE);
    }
    HWND GetBandHwnd() const { return m_hwnd; }

    // IUnknown
    STDMETHOD(QueryInterface)(REFIID riid, void** ppv) {
        static const QITAB qit[] = {
            QITABENT(JC9Band, IDeskBand2),
            QITABENT(JC9Band, IDeskBand),
            QITABENT(JC9Band, IDockingWindow),
            QITABENT(JC9Band, IOleWindow),
            QITABENT(JC9Band, IObjectWithSite),
            QITABENT(JC9Band, IPersistStream),
            QITABENT(JC9Band, IInputObject),
            {0}
        };
        HRESULT hr = QISearch(this, qit, riid, ppv);
        if(SUCCEEDED(hr)) AddRef();
        return hr;
    }
    STDMETHOD_(ULONG, AddRef)() { return InterlockedIncrement(&m_ref); }
    STDMETHOD_(ULONG, Release)() {
        ULONG c = InterlockedDecrement(&m_ref);
        if(c == 0) delete this;
        return c;
    }

    // IOleWindow
    STDMETHOD(GetWindow)(HWND* ph) { *ph = m_hwnd; return S_OK; }
    STDMETHOD(ContextSensitiveHelp)(BOOL) { return E_NOTIMPL; }

    // IDockingWindow
    STDMETHOD(ShowDW)(BOOL f) {
        m_showing = f;
        if(m_hwnd) ShowWindow(m_hwnd, f ? SW_SHOW : SW_HIDE);
        return S_OK;
    }
    STDMETHOD(CloseDW)(DWORD) {
        if(m_hwnd) { ShowWindow(m_hwnd, SW_HIDE); }
        return S_OK;
    }
    STDMETHOD(ResizeBorderDW)(LPCRECT, IUnknown*, BOOL) { return E_NOTIMPL; }

    // IDeskBand
    STDMETHOD(GetBandInfo)(DWORD id, DWORD view, DESKBANDINFO* dbi) {
        m_bandId = id; m_viewMode = view;
        if(dbi->dwMask & DBIM_MINSIZE)  { dbi->ptMinSize.x = 48; dbi->ptMinSize.y = 22; }
        if(dbi->dwMask & DBIM_MAXSIZE)  { dbi->ptMaxSize.x = 200; dbi->ptMaxSize.y = 22; }
        if(dbi->dwMask & DBIM_INTEGRAL) { dbi->ptIntegral.x = 1; dbi->ptIntegral.y = 1; }
        if(dbi->dwMask & DBIM_ACTUAL)   { dbi->ptActual.x = 80; dbi->ptActual.y = 22; }
        if(dbi->dwMask & DBIM_TITLE)    { wcscpy_s(dbi->wszTitle, L"JC9"); }
        if(dbi->dwMask & DBIM_MODEFLAGS){ dbi->dwModeFlags = DBIMF_NORMAL | DBIMF_BKCOLOR; }
        if(dbi->dwMask & DBIM_BKCOLOR)  { dbi->crBkgnd = RGB(30,30,46); }
        return S_OK;
    }

    // IDeskBand2
    STDMETHOD(CanRenderComposited)(BOOL* p) { *p = TRUE; return S_OK; }
    STDMETHOD(SetCompositionState)(BOOL f) { return S_OK; }
    STDMETHOD(GetCompositionState)(BOOL* p) { *p = TRUE; return S_OK; }

    // IObjectWithSite
    STDMETHOD(SetSite)(IUnknown* p) {
        if(m_site) { m_site->Release(); m_site = nullptr; }
        if(!p) { if(m_hwnd) { DestroyWindow(m_hwnd); m_hwnd = nullptr; } return S_OK; }

        IOleWindow* oleWnd = nullptr;
        if(SUCCEEDED(p->QueryInterface(IID_IOleWindow, (void**)&oleWnd))) {
            oleWnd->GetWindow(&m_hwndParent);
            oleWnd->Release();
        }
        p->QueryInterface(IID_IInputObjectSite, (void**)&m_site);

        if(m_hwndParent && !m_hwnd) {
            HINSTANCE hi = GetModuleHandleW(nullptr);
            WNDCLASSEXW wc = {sizeof(wc)};
            wc.lpfnWndProc = BandWndProc;
            wc.hInstance = hi;
            wc.lpszClassName = BAND_CLASS;
            wc.hCursor = LoadCursorW(nullptr, (LPCWSTR)IDC_HAND);
            wc.hbrBackground = (HBRUSH)GetStockObject(NULL_BRUSH);
            RegisterClassExW(&wc);

            m_hwnd = CreateWindowExW(WS_EX_TOOLWINDOW, BAND_CLASS, L"",
                WS_CHILD | WS_VISIBLE | WS_CLIPCHILDREN,
                0, 0, 80, 22, m_hwndParent, nullptr, hi, this);

            g_band = this;
        }
        return S_OK;
    }
    STDMETHOD(GetSite)(REFIID riid, void** pp) {
        if(m_site) return m_site->QueryInterface(riid, pp);
        return E_FAIL;
    }

    // IPersist
    STDMETHOD(GetClassID)(CLSID* p) { *p = CLSID_JC9DeskBand; return S_OK; }

    // IPersistStream
    STDMETHOD(IsDirty)() { return S_FALSE; }
    STDMETHOD(Load)(IStream*) { return S_OK; }
    STDMETHOD(Save)(IStream*, BOOL) { return S_OK; }
    STDMETHOD(GetSizeMax)(ULARGE_INTEGER* p) { p->QuadPart = 0; return S_OK; }

    // IInputObject
    STDMETHOD(UIActivateIO)(BOOL f, MSG*) { if(f && m_hwnd) SetFocus(m_hwnd); return S_OK; }
    STDMETHOD(HasFocusIO)() { return (GetFocus() == m_hwnd) ? S_OK : S_FALSE; }
    STDMETHOD(TranslateAcceleratorIO)(MSG*) { return S_FALSE; }

    // Window procedure
    static LRESULT CALLBACK BandWndProc(HWND hw, UINT msg, WPARAM wp, LPARAM lp) {
        JC9Band* b = (JC9Band*)GetWindowLongPtrW(hw, GWLP_USERDATA);
        switch(msg) {
        case WM_CREATE:
            b = (JC9Band*)((CREATESTRUCT*)lp)->lpCreateParams;
            SetWindowLongPtrW(hw, GWLP_USERDATA, (LONG_PTR)b);
            return 0;
        case WM_PAINT: {
            PAINTSTRUCT ps; HDC hdc = BeginPaint(hw, &ps); RECT rc;
            GetClientRect(hw, &rc);
            HBRUSH bg = CreateSolidBrush(RGB(30,30,46));
            FillRect(hdc, &rc, bg); DeleteObject(bg);
            if(b) {
                SetBkMode(hdc, TRANSPARENT);
                SetTextColor(hdc, RGB(137,180,250));
                HFONT f = CreateFontW(15,0,0,0,FW_BOLD,0,0,0,DEFAULT_CHARSET,0,0,0,0,L"Segoe UI");
                SelectObject(hdc, f);
                DrawTextW(hdc, b->m_text.c_str(), -1, &rc, DT_CENTER|DT_VCENTER|DT_SINGLELINE);
                DeleteObject(f);
            }
            EndPaint(hw, &ps); return 0;
        }
        case WM_LBUTTONDOWN:
            if(g_tauriHwnd) PostMessageW(g_tauriHwnd, WM_USER+100, 0, 0);
            return 0;
        case WM_ERASEBKGND: return 1;
        }
        return DefWindowProcW(hw, msg, wp, lp);
    }
};

// =============================================
// Class Factory
// =============================================
class BandFactory : public IClassFactory {
    LONG m_ref;
public:
    BandFactory() : m_ref(1) {}
    STDMETHOD(QueryInterface)(REFIID riid, void** ppv) {
        static const QITAB qit[] = { QITABENT(BandFactory, IClassFactory), {0} };
        return QISearch(this, qit, riid, ppv);
    }
    STDMETHOD_(ULONG, AddRef)() { return InterlockedIncrement(&m_ref); }
    STDMETHOD_(ULONG, Release)() { ULONG c=InterlockedDecrement(&m_ref); if(!c) delete this; return c; }
    STDMETHOD(CreateInstance)(IUnknown* p, REFIID riid, void** pp) {
        if(p) return CLASS_E_NOAGGREGATION;
        JC9Band* b = new JC9Band();
        if(!b) return E_OUTOFMEMORY;
        HRESULT hr = b->QueryInterface(riid, pp);
        b->Release();
        return hr;
    }
    STDMETHOD(LockServer)(BOOL) { return S_OK; }
};

static LONG g_lockCount = 0;

// DLL exports
STDAPI DllGetClassObject(REFCLSID rclsid, REFIID riid, void** ppv) {
    if(rclsid == CLSID_JC9DeskBand) {
        BandFactory* f = new BandFactory();
        HRESULT hr = f->QueryInterface(riid, ppv);
        f->Release();
        return hr;
    }
    return CLASS_E_CLASSNOTAVAILABLE;
}

STDAPI DllCanUnloadNow() { return (g_lockCount == 0) ? S_OK : S_FALSE; }

STDAPI DllRegisterServer() { RegisterBand(); return S_OK; }
STDAPI DllUnregisterServer() { UnregisterBand(); return S_OK; }

// =============================================
// COM Registration
// =============================================
HRESULT RegisterBand() {
    WCHAR path[MAX_PATH];
    GetModuleFileNameW((HMODULE)&__ImageBase, path, MAX_PATH);

    HKEY hk;
    // CLSID
    RegCreateKeyExW(HKEY_CLASSES_ROOT,
        L"CLSID\\{A1B2C3D4-E5F6-7890-ABCD-EF1234567890}", 0, 0, 0,
        KEY_WRITE, 0, &hk, 0);
    RegSetValueExW(hk, 0, 0, REG_SZ, (BYTE*)L"JC9 DeskBand", 26);
    HKEY hk2;
    RegCreateKeyExW(hk, L"InProcServer32", 0,0,0, KEY_WRITE,0,&hk2,0);
    RegSetValueExW(hk2, 0,0, REG_SZ, (BYTE*)path, (DWORD)((wcslen(path)+1)*2));
    RegSetValueExW(hk2, L"ThreadingModel",0,REG_SZ,(BYTE*)L"Apartment",20);
    RegCloseKey(hk2);
    RegCloseKey(hk);

    // Component Category: DeskBand
    RegCreateKeyExW(HKEY_CLASSES_ROOT,
        L"Component Categories\\{00021492-0000-0000-C000-000000000046}"
        L"\\Enum\\{A1B2C3D4-E5F6-7890-ABCD-EF1234567890}", 0,0,0, KEY_WRITE,0,&hk,0);
    RegCloseKey(hk);

    SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, 0, 0);
    return S_OK;
}

void UnregisterBand() {
    RegDeleteTreeW(HKEY_CLASSES_ROOT, L"CLSID\\{A1B2C3D4-E5F6-7890-ABCD-EF1234567890}");
    RegDeleteTreeW(HKEY_CLASSES_ROOT,
        L"Component Categories\\{00021492-0000-0000-C000-000000000046}"
        L"\\Enum\\{A1B2C3D4-E5F6-7890-ABCD-EF1234567890}");
    SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, 0, 0);
}

// =============================================
// Public API (for Tauri)
// =============================================
JC9_API HRESULT jc9_init(HWND hwnd) {
    g_tauriHwnd = hwnd;
    return S_OK;
}
JC9_API HRESULT jc9_set_text(const wchar_t* text) {
    if(g_band) { g_band->SetText(text); return S_OK; }
    return E_FAIL;
}
JC9_API void jc9_cleanup() {
    g_band = nullptr;
    g_tauriHwnd = nullptr;
}
