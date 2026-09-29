// midi_proxy.c — passive MIDI sniffer proxy for GP-100.exe capture (Rota 1).
//
// Build: python analysis/build_proxy.py --build   (generates impl + build file, runs zig cc)
//
// How it works:
//   - The exe loads winmm.dll from its own directory first (DLL search order),
//     so a copy of this DLL next to GP-100.exe intercepts all winmm imports.
//   - Every export is forwarded 1:1 to the REAL C:\Windows\SysWOW64\winmm.dll
//     (resolved lazily) — traffic is never modified. Delete the DLL to restore.
//   - Logged to %TEMP%\midi_trace.jsonl (JSONL, one message per line):
//       out_short  midiOutShortMsg        (4-byte UINT, little-endian)
//       out_long   midiOutLongMsg payload (sysex — commands PC -> GP-100)
//       in_short   MIM_DATA via hooked midiInOpen callback   (button/knob acks)
//       in_long    MIM_LONGDATA payload                      (sysex dumps device -> PC)
//       meta       proxy lifecycle events (load, in_open)
//
// SAFETY: read-only sniffer. If log opening fails, forwarding still works
// (capture is best-effort and can never crash the app).

#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <mmsystem.h>
#include <stdint.h>

static HMODULE g_real = NULL;
static CRITICAL_SECTION g_cs;
static HANDLE g_log = NULL;
static DWORD_PTR g_in_user_cb = 0; // app's original midiInOpen callback

typedef MMRESULT (WINAPI *fn_outShort)(HMIDIOUT, UINT);
typedef MMRESULT (WINAPI *fn_outLong) (HMIDIOUT, LPMIDIHDR);
typedef MMRESULT (WINAPI *fn_inOpen)  (LPHMIDIIN, UINT, DWORD_PTR, DWORD_PTR, DWORD);
typedef MMRESULT (WINAPI *fn_inAdd)   (HMIDIIN, LPMIDIHDR);
typedef void (WINAPI *midiin_proc_t)(HMIDIIN, UINT, DWORD_PTR, DWORD_PTR, DWORD_PTR);

// ---- logging ----

static void log_open(void) {
    char tmp[MAX_PATH], path[MAX_PATH + 32];
    if (GetEnvironmentVariableA("TEMP", tmp, MAX_PATH) == 0 || !tmp[0])
        lstrcpyA(tmp, ".");
    wsprintfA(path, "%s\\midi_trace.jsonl", tmp);
    g_log = CreateFileA(path, FILE_APPEND_DATA,
                        FILE_SHARE_READ | FILE_SHARE_WRITE, NULL,
                        OPEN_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULL);
}

static void log_meta(const char *note) {
    if (!g_log || g_log == INVALID_HANDLE_VALUE) return;
    char buf[192];
    int n = wsprintfA(buf, "{\"ts\":%lu,\"pid\":%lu,\"dir\":\"meta\",\"note\":\"%s\"}\r\n",
                      GetTickCount(), GetCurrentProcessId(), note);
    EnterCriticalSection(&g_cs);
    DWORD w;
    WriteFile(g_log, buf, n, &w, NULL);
    FlushFileBuffers(g_log);
    LeaveCriticalSection(&g_cs);
}

static char *to_hex(const uint8_t *p, DWORD len) {
    if (len > 0x40000) len = 0x40000; // 256 KiB cap; real sysex is far smaller
    char *s = (char *)HeapAlloc(GetProcessHeap(), 0, (SIZE_T)len * 2 + 1);
    if (!s) return NULL;
    static const char d[] = "0123456789abcdef";
    for (DWORD i = 0; i < len; i++) {
        s[i * 2]     = d[p[i] >> 4];
        s[i * 2 + 1] = d[p[i] & 15];
    }
    s[len * 2] = 0;
    return s;
}

static void log_raw(const char *dir, const char *hex, DWORD hexchars) {
    if (!g_log || g_log == INVALID_HANDLE_VALUE) return;
    char head[160];
    int n = wsprintfA(head, "{\"ts\":%lu,\"pid\":%lu,\"dir\":\"%s\",\"len\":%lu,\"hex\":\"",
                      GetTickCount(), GetCurrentProcessId(), dir, hexchars / 2);
    EnterCriticalSection(&g_cs);
    DWORD w;
    WriteFile(g_log, head, n, &w, NULL);
    if (hexchars) WriteFile(g_log, hex, hexchars, &w, NULL);
    WriteFile(g_log, "\"}\r\n", 4, &w, NULL); // close hex string + object
    FlushFileBuffers(g_log);
    LeaveCriticalSection(&g_cs);
}

static void log_bytes(const char *dir, const uint8_t *p, DWORD len) {
    if (!len) return;
    char *h = to_hex(p, len);
    if (h) {
        log_raw(dir, h, len * 2 > 0x80000 ? 0x80000 : len * 2);
        HeapFree(GetProcessHeap(), 0, h);
    } else {
        log_raw(dir, "", 0); // event recorded, payload dropped (allocation failed)
    }
}

// ---- hooked output functions ----

__attribute__((used))
MMRESULT WINAPI hook_midiOutShortMsg(HMIDIOUT h, UINT msg) {
    uint8_t b[4] = { (uint8_t)(msg & 0xFF), (uint8_t)((msg >> 8) & 0xFF),
                     (uint8_t)((msg >> 16) & 0xFF), (uint8_t)((msg >> 24) & 0xFF) };
    log_bytes("out_short", b, 4);
    static fn_outShort f;
    if (!f) f = (fn_outShort)(void *)GetProcAddress(g_real, "midiOutShortMsg");
    return f(h, msg);
}

__attribute__((used))
MMRESULT WINAPI hook_midiOutLongMsg(HMIDIOUT h, LPMIDIHDR pmh) {
    if (pmh && pmh->lpData)
        log_bytes("out_long", (const uint8_t *)pmh->lpData, pmh->dwBufferLength);
    static fn_outLong f;
    if (!f) f = (fn_outLong)(void *)GetProcAddress(g_real, "midiOutLongMsg");
    return f(h, pmh);
}

// ---- hooked input: capture what the GP-100 SENDS us ----

// JUCE opens midiIn with CALLBACK_FUNCTION (0x4AC660). We swap the app's
// callback for this trampoline, log, then call the app's callback unchanged.
static void WINAPI in_trampoline(HMIDIIN h, UINT wMsg, DWORD_PTR dwInstance,
                                 DWORD_PTR p1, DWORD_PTR p2) {
    if (wMsg == MIM_DATA || wMsg == MIM_MOREDATA) {
        uint8_t b[3] = { (uint8_t)(p1 & 0xFF), (uint8_t)((p1 >> 8) & 0xFF),
                         (uint8_t)((p1 >> 16) & 0xFF) };
        log_bytes("in_short", b, 3);
    } else if (wMsg == MIM_LONGDATA) {
        LPMIDIHDR pmh = (LPMIDIHDR)p1;
        if (pmh && pmh->lpData) {
            DWORD len = pmh->dwBytesRecorded ? pmh->dwBytesRecorded
                                             : pmh->dwBufferLength;
            log_bytes("in_long", (const uint8_t *)pmh->lpData, len);
        }
    }
    if (g_in_user_cb) {
        midiin_proc_t cb = (midiin_proc_t)(void *)g_in_user_cb;
        cb(h, wMsg, dwInstance, p1, p2);
    }
}

__attribute__((used))
MMRESULT WINAPI hook_midiInOpen(LPHMIDIIN ph, UINT devid, DWORD_PTR cb,
                                DWORD_PTR inst, DWORD flags) {
    DWORD_PTR user = cb;
    g_in_user_cb = 0;
    if (cb && (flags & CALLBACK_TYPEMASK) == CALLBACK_FUNCTION) {
        g_in_user_cb = cb;
        user = (DWORD_PTR)(void *)&in_trampoline;
    }
    static fn_inOpen f;
    if (!f) f = (fn_inOpen)(void *)GetProcAddress(g_real, "midiInOpen");
    MMRESULT r = f(ph, devid, user, inst, flags);
    if (r == MMSYSERR_NOERROR) log_meta("midiInOpen captured (in-direction logging active)");
    return r;
}

__attribute__((used))
MMRESULT WINAPI hook_midiInAddBuffer(HMIDIIN h, LPMIDIHDR pmh) {
    static fn_inAdd f;
    if (!f) f = (fn_inAdd)(void *)GetProcAddress(g_real, "midiInAddBuffer");
    return f(h, pmh);
}

// ---- lazy resolver for the generic forwarders (included below) ----

static int WINAPI no_op_stub(void) { return 0; }

static FARPROC resolve(const char *name) {
    FARPROC p = GetProcAddress(g_real, name);
    if (!p) {
        // never hard-crash the host app; cross-check in build_proxy.py guarantees
        // every target exists in the real winmm, so this is belt-and-suspenders
        log_meta("missing export -> no-op fallback");
        return (FARPROC)no_op_stub;
    }
    return p;
}

// __FORWARDERS_INCLUDE__

BOOL WINAPI DllMain(HINSTANCE h, DWORD reason, LPVOID reserved) {
    (void)reserved; (void)h;
    if (reason == DLL_PROCESS_ATTACH) {
        DisableThreadLibraryCalls(h);
        InitializeCriticalSection(&g_cs);
        char path[MAX_PATH];
        GetSystemDirectoryA(path, MAX_PATH);
        lstrcatA(path, "\\winmm.dll");
        g_real = LoadLibraryA(path);
        if (!g_real) return FALSE;
        log_open();
        log_meta("proxy loaded");
    } else if (reason == DLL_PROCESS_DETACH) {
        if (g_log && g_log != INVALID_HANDLE_VALUE) CloseHandle(g_log);
        if (g_real) FreeLibrary(g_real);
        DeleteCriticalSection(&g_cs);
    }
    return TRUE;
}
