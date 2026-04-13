use arboard::Clipboard;
#[cfg(not(target_os = "windows"))]
use base64::{engine::general_purpose, Engine as _};
#[cfg(not(target_os = "windows"))]
use image::ImageOutputFormat;
#[cfg(not(target_os = "windows"))]
use rusqlite::{params, Connection};
#[cfg(not(target_os = "windows"))]
use std::io::Cursor;
use std::{thread, time::Duration};
use tauri::{AppHandle, Emitter};

use crate::clipboard::operations::read_and_persist_clipboard;
#[cfg(not(target_os = "windows"))]
use crate::db::{detect_category, get_db_path, read_setting_sync, signature_for};
use crate::state::{safe_lock, AppState};

#[cfg(target_os = "windows")]
use std::sync::atomic::Ordering;

#[cfg(target_os = "windows")]
use windows::Win32::{
    Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
    System::{
        DataExchange::{
            AddClipboardFormatListener, GetClipboardSequenceNumber, RemoveClipboardFormatListener,
        },
        LibraryLoader::GetModuleHandleW,
    },
    UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, KillTimer,
        RegisterClassExW, SetTimer, TranslateMessage, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT,
        HWND_MESSAGE, MSG, WINDOW_EX_STYLE, WINDOW_STYLE, WM_APP, WM_CLIPBOARDUPDATE,
        WM_POWERBROADCAST, WM_TIMER, WNDCLASSEXW,
    },
};

/// Win32 message-loop based clipboard monitor.
///
/// Creates an invisible message-only window, registers it with
/// `AddClipboardFormatListener`, then drives a `GetMessage` loop.
/// The OS calls our window procedure with `WM_CLIPBOARDUPDATE` every time
/// the clipboard changes — no polling, no missed events during sleep/lock.
///
/// **Resilience features (post-sleep/hibernate recovery):**
/// - A 30-second `SetTimer` heartbeat fires `WM_TIMER`.  On each tick we
///   re-check `AddClipboardFormatListener` and re-read the clipboard so
///   that `GetMessageW` being silently stuck doesn't cause a permanent hang.
/// - `WM_POWERBROADCAST` with `PBT_APMRESUMEAUTOMATIC`/`PBT_APMRESUMESUSPEND`
///   triggers a full cycle: remove → re-add listener, re-init arboard, and
///   do an immediate clipboard read.
///
/// The function returns (instead of looping forever) only when:
///   - The hidden window cannot be created (init failure).
///   - `WM_APP` (our shutdown signal) is posted to the window's queue.
/// In both cases the supervisor thread will restart us.
#[cfg(target_os = "windows")]
pub fn run_clipboard_monitor(state: AppState, handle: AppHandle) {
    use windows::core::PCWSTR;

    state.monitor_alive.store(true, Ordering::SeqCst);

    let class_name: Vec<u16> = "ClipFlowMonitor\0".encode_utf16().collect();

    let hmodule = unsafe { GetModuleHandleW(None).unwrap_or_default() };
    let hinstance = HINSTANCE(hmodule.0);

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }

    let wc = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(wnd_proc),
        hInstance: hinstance,
        lpszClassName: PCWSTR(class_name.as_ptr()),
        ..Default::default()
    };
    unsafe { RegisterClassExW(&wc) };

    let hwnd_result = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            PCWSTR(class_name.as_ptr()),
            PCWSTR(std::ptr::null()),
            WINDOW_STYLE::default(),
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            Some(HWND_MESSAGE),
            None,
            Some(hinstance),
            None,
        )
    };

    let hwnd = match hwnd_result {
        Ok(h) => h,
        Err(e) => {
            eprintln!(
                "\x1b[31m[CLIPFLOW][CRITICAL] Failed to create message window ({})\x1b[0m",
                e
            );
            let _ = handle.emit(
                "listener-crashed",
                serde_json::json!({
                    "reason": "window_create_failed",
                    "ts_ms": chrono::Local::now().timestamp_millis()
                }),
            );
            state.monitor_alive.store(false, Ordering::SeqCst);
            return;
        }
    };

    if let Err(e) = unsafe { AddClipboardFormatListener(hwnd) } {
        eprintln!(
            "\x1b[31m[CLIPFLOW][CRITICAL] AddClipboardFormatListener failed: {}\x1b[0m",
            e
        );
        let _ = handle.emit(
            "listener-crashed",
            serde_json::json!({
                "reason": "add_listener_failed",
                "ts_ms": chrono::Local::now().timestamp_millis()
            }),
        );
        unsafe {
            let _ = DestroyWindow(hwnd);
        }
        state.monitor_alive.store(false, Ordering::SeqCst);
        return;
    }

    *safe_lock(&state.monitor_hwnd) = hwnd.0 as isize;

    println!(
        "[clipflow][clipboard] Win32 monitor started (HWND={:?})",
        hwnd
    );
    let _ = handle.emit(
        "clipboard-monitor-status",
        serde_json::json!({
            "state": "running_win32",
            "ts_ms": chrono::Local::now().timestamp_millis()
        }),
    );

    const HEARTBEAT_TIMER_ID: usize = 1;
    const HEARTBEAT_INTERVAL_MS: u32 = 30_000;
    unsafe { SetTimer(Some(hwnd), HEARTBEAT_TIMER_ID, HEARTBEAT_INTERVAL_MS, None) };

    let mut cb = match Clipboard::new() {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "\x1b[31m[CLIPFLOW][CRITICAL] arboard init failed: {} — monitor exiting\x1b[0m",
                e
            );
            unsafe {
                let _ = KillTimer(Some(hwnd), HEARTBEAT_TIMER_ID);
                let _ = RemoveClipboardFormatListener(hwnd);
                let _ = DestroyWindow(hwnd);
            }
            *safe_lock(&state.monitor_hwnd) = 0;
            state.monitor_alive.store(false, Ordering::SeqCst);
            return;
        }
    };

    let mut last_seq: u32 = unsafe { GetClipboardSequenceNumber() };
    let mut last_img: usize = 0;

    fn reregister_listener(hwnd: HWND) -> Result<(), String> {
        unsafe {
            let _ = RemoveClipboardFormatListener(hwnd);
            AddClipboardFormatListener(hwnd).map_err(|e| format!("Re-register failed: {}", e))?;
        }
        Ok(())
    }

    let mut voluntary_exit = false;
    loop {
        let mut msg = MSG::default();
        let ret = unsafe { GetMessageW(&mut msg, None, 0, 0) };

        if ret.0 == 0 {
            break;
        }
        if ret.0 < 0 {
            eprintln!("\x1b[31m[CLIPFLOW][CRITICAL] GetMessage returned error — monitor exiting for restart\x1b[0m");
            break;
        }

        match msg.message {
            x if x == WM_APP => {
                println!(
                    "[clipflow][clipboard] received WM_APP shutdown signal, exiting for restart"
                );
                voluntary_exit = true;
                break;
            }
            x if x == WM_CLIPBOARDUPDATE => {
                thread::sleep(Duration::from_millis(30));
                read_and_persist_clipboard(&state, &handle, &mut cb, &mut last_seq, &mut last_img);
            }
            x if x == WM_POWERBROADCAST => {
                let event_type = msg.wParam.0;
                if event_type == crate::PBT_APMRESUMEAUTOMATIC
                    || event_type == crate::PBT_APMRESUMESUSPEND
                {
                    println!("[clipflow][clipboard] ⚡ System resumed from sleep/hibernate — rebuilding listener");
                    let _ = handle.emit(
                        "clipboard-monitor-status",
                        serde_json::json!({
                            "state": "resuming_from_sleep",
                            "ts_ms": chrono::Local::now().timestamp_millis()
                        }),
                    );

                    thread::sleep(Duration::from_millis(500));

                    if let Err(e) = reregister_listener(hwnd) {
                        eprintln!("[clipflow][clipboard] ❌ Failed to re-register after resume: {} — exiting for full restart", e);
                        break;
                    }

                    match Clipboard::new() {
                        Ok(new_cb) => {
                            cb = new_cb;
                        }
                        Err(e) => {
                            eprintln!("[clipflow][clipboard] ❌ arboard re-init failed after resume: {} — exiting for full restart", e);
                            break;
                        }
                    }

                    last_seq = 0;
                    last_img = 0;

                    *safe_lock(&state.skip_monitor) = false;
                    *safe_lock(&state.last_clipboard_write_ms) = 0;
                    *safe_lock(&state.is_internal_pasting) = false;

                    read_and_persist_clipboard(
                        &state,
                        &handle,
                        &mut cb,
                        &mut last_seq,
                        &mut last_img,
                    );

                    println!(
                        "[clipflow][clipboard] ✅ Successfully recovered after sleep/hibernate"
                    );
                }
            }
            x if x == WM_TIMER => {
                let current_seq = unsafe { GetClipboardSequenceNumber() };
                if current_seq != 0 && current_seq != last_seq {
                    println!(
                        "[clipflow][clipboard] ⚠️ Heartbeat detected missed clipboard change (seq {}→{}) — re-reading",
                        last_seq,
                        current_seq
                    );
                    read_and_persist_clipboard(
                        &state,
                        &handle,
                        &mut cb,
                        &mut last_seq,
                        &mut last_img,
                    );
                    last_seq = current_seq;
                }
                let now_ms = chrono::Local::now().timestamp_millis();
                {
                    let t = *safe_lock(&state.last_clipboard_write_ms);
                    if t > 0 && now_ms.saturating_sub(t) > 5_000 {
                        *safe_lock(&state.skip_monitor) = false;
                        *safe_lock(&state.last_clipboard_write_ms) = 0;
                    }
                }
            }
            _ => {}
        }

        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    unsafe {
        let _ = KillTimer(Some(hwnd), HEARTBEAT_TIMER_ID);
        let _ = RemoveClipboardFormatListener(hwnd);
        let _ = DestroyWindow(hwnd);
    }
    *safe_lock(&state.monitor_hwnd) = 0;
    if !voluntary_exit {
        state.monitor_alive.store(false, Ordering::SeqCst);
    }
    println!(
        "[clipflow][clipboard] Win32 monitor thread exited cleanly (voluntary={})",
        voluntary_exit
    );
}

/// Non-Windows fallback: arboard polling (unchanged behaviour).
#[cfg(not(target_os = "windows"))]
pub fn run_clipboard_monitor(state: AppState, handle: AppHandle) {
    state
        .monitor_alive
        .store(true, std::sync::atomic::Ordering::SeqCst);

    let mut cb = loop {
        match Clipboard::new() {
            Ok(c) => break c,
            Err(e) => {
                println!("[clipflow][clipboard] init failed, retrying: {}", e);
                thread::sleep(Duration::from_millis(800));
            }
        }
    };

    let mut last_txt = String::new();
    let mut last_img: usize = 0;
    let mut last_status_ms: i64 = 0;

    loop {
        let now_ms = chrono::Local::now().timestamp_millis();
        if last_status_ms == 0 || now_ms.saturating_sub(last_status_ms) > 30_000 {
            last_status_ms = now_ms;
            let _ = handle.emit(
                "clipboard-monitor-status",
                serde_json::json!({"state": "running_poll", "ts_ms": now_ms}),
            );
        }

        if let Ok(t) = state.last_clipboard_write_ms.lock() {
            if *t > 0 && now_ms.saturating_sub(*t) < 800 {
                thread::sleep(Duration::from_millis(50));
                continue;
            }
        }
        if let Ok(skip) = state.skip_monitor.lock() {
            if *skip {
                thread::sleep(Duration::from_millis(50));
                continue;
            }
        }
        if state
            .is_internal_pasting
            .lock()
            .map(|g| *g)
            .unwrap_or(false)
        {
            thread::sleep(Duration::from_millis(50));
            continue;
        }

        let mut has_new = false;
        let mut new_c = String::new();
        let mut cat = "text".to_string();

        if let Ok(i) = cb.get_image() {
            let h = i.bytes.len() + i.width;
            if h != last_img && h > 0 {
                last_img = h;
                let mut b = Vec::new();
                if image::write_buffer_with_format(
                    &mut Cursor::new(&mut b),
                    &i.bytes,
                    i.width as u32,
                    i.height as u32,
                    image::ColorType::Rgba8,
                    ImageOutputFormat::Png,
                )
                .is_ok()
                {
                    new_c = format!(
                        "data:image/png;base64,{}",
                        general_purpose::STANDARD.encode(b)
                    );
                    cat = "image".to_string();
                    has_new = true;
                    last_txt = String::new();
                }
            }
        }

        if !has_new {
            if let Ok(t) = cb.get_text() {
                if !t.is_empty() && t != last_txt {
                    last_txt = t.clone();
                    new_c = t;
                    cat = detect_category(&new_c);
                    has_new = true;
                    last_img = 0;
                }
            }
        }

        if has_new {
            let new_sig = signature_for(&new_c);
            let should_ignore = if let Ok(mut lock) = state.ignore_signature.lock() {
                if lock.as_deref() == Some(&new_sig) {
                    *lock = None;
                    true
                } else {
                    false
                }
            } else {
                false
            };

            if !should_ignore {
                let privacy = read_setting_sync(&handle, "privacy_mode");
                if privacy != "true" {
                    let db = get_db_path(&handle);
                    if let Ok(conn) = Connection::open(db) {
                        let _ =
                            conn.execute("DELETE FROM history WHERE content = ?1", params![new_c]);
                        let _ = conn.execute(
                            "INSERT INTO history (content, category) VALUES (?1, ?2)",
                            params![new_c, cat],
                        );
                        let _ = conn.execute(
                            "DELETE FROM history WHERE id NOT IN (SELECT id FROM history ORDER BY id DESC LIMIT 200)",
                            [],
                        );
                    }
                    let _ = handle.emit("clipboard-monitor", &new_c);
                }
            }
        }

        thread::sleep(Duration::from_millis(500));
    }
}
