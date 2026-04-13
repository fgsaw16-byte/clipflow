use std::{thread, time::Duration};
use tauri::{AppHandle, Emitter};

use crate::clipboard::monitor::run_clipboard_monitor;
use crate::state::{safe_lock, AppState};

pub fn spawn_clipboard_supervisor(state: AppState, handle: AppHandle) {
    thread::spawn(move || {
        let mut panic_count: u32 = 0;
        loop {
            let state_inner = state.clone();
            let handle_inner = handle.clone();
            let join_handle = thread::spawn(move || {
                run_clipboard_monitor(state_inner, handle_inner);
            });
            match join_handle.join() {
                Ok(_) => {
                    let was_alive = state
                        .monitor_alive
                        .load(std::sync::atomic::Ordering::SeqCst);
                    if !was_alive {
                        panic_count += 1;
                        eprintln!(
                            "\x1b[31m[CLIPFLOW][CRITICAL] Monitor init failed (crash_count={}) — will retry\x1b[0m",
                            panic_count
                        );
                        let _ = handle.emit(
                            "listener-crashed",
                            serde_json::json!({
                                "reason": "init_failed",
                                "crash_count": panic_count,
                                "ts_ms": chrono::Local::now().timestamp_millis()
                            }),
                        );
                    } else {
                        println!(
                            "[clipflow][clipboard] monitor restarted voluntarily (count={})",
                            panic_count
                        );
                        panic_count = 0;
                    }
                }
                Err(_) => {
                    panic_count += 1;
                    eprintln!("\x1b[31m[CLIPFLOW][CRITICAL] !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!\x1b[0m");
                    eprintln!("\x1b[31m[CLIPFLOW][CRITICAL]  CLIPBOARD MONITOR THREAD PANICKED         \x1b[0m");
                    eprintln!("\x1b[31m[CLIPFLOW][CRITICAL]  crash_count={}  — restarting              \x1b[0m", panic_count);
                    eprintln!("\x1b[31m[CLIPFLOW][CRITICAL] !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!\x1b[0m");
                    let _ = handle.emit(
                        "listener-crashed",
                        serde_json::json!({
                            "reason": "panic",
                            "crash_count": panic_count,
                            "ts_ms": chrono::Local::now().timestamp_millis()
                        }),
                    );
                }
            }
            *safe_lock(&state.skip_monitor) = false;
            *safe_lock(&state.last_clipboard_write_ms) = 0;
            *safe_lock(&state.is_internal_pasting) = false;
            #[cfg(target_os = "windows")]
            {
                *safe_lock(&state.monitor_hwnd) = 0;
            }
            let back_off_ms = if panic_count > 0 {
                (500u64).saturating_mul(panic_count.min(10) as u64)
            } else {
                100
            };
            thread::sleep(Duration::from_millis(back_off_ms));
        }
    });
}
