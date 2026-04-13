mod state;
mod db;
mod clipboard;
mod window;
mod commands;
mod server;

use tauri::{Manager, Emitter, WindowEvent};
use std::{thread, time::Duration, sync::{Mutex, Arc, atomic::{AtomicBool}}, collections::{HashSet}};
use std::path::PathBuf;
use tokio::sync::broadcast;

use tauri_plugin_global_shortcut::{Builder as ShortcutBuilder, ShortcutState, Code, Modifiers}; 
use actix_web::{web, App, HttpServer};
use tauri_plugin_autostart::MacosLauncher;

#[cfg(target_os = "windows")]
use window_vibrancy::apply_acrylic;

#[cfg(target_os = "windows")]
use windows::Win32::{
    System::Threading::{GetCurrentProcessId},
    UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
};

/// PBT_APMRESUMEAUTOMATIC: system has resumed from sleep/hibernate
#[cfg(target_os = "windows")]
const PBT_APMRESUMEAUTOMATIC: usize = 0x0012;
/// PBT_APMRESUMESUSPEND: user-initiated resume from sleep
#[cfg(target_os = "windows")]
const PBT_APMRESUMESUSPEND: usize = 0x0007;

use db::{get_db_path, init_db, read_setting_sync};
use clipboard::spawn_clipboard_supervisor;
use commands::*;
use state::AppState;
use window::{position_window, position_window_at_mouse};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let (event_tx, _) = broadcast::channel::<String>(128);
    let app_state = AppState {
        last_content: Arc::new(Mutex::new(String::new())),
        ignore_signature: Arc::new(Mutex::new(None)),
        skip_monitor: Arc::new(Mutex::new(false)),
        last_clipboard_write_ms: Arc::new(Mutex::new(0)),
        last_image_upload_hash: Arc::new(Mutex::new(0)),
        last_image_upload_ms: Arc::new(Mutex::new(0)),
        paste_queue: Arc::new(Mutex::new(Vec::new())),
        is_internal_pasting: Arc::new(Mutex::new(false)),
        file_save_path: Arc::new(Mutex::new(dirs::download_dir().unwrap_or_else(|| std::env::temp_dir()))),
        event_tx,
        monitor_alive: Arc::new(AtomicBool::new(false)),
        #[cfg(target_os = "windows")]
        monitor_hwnd: Arc::new(Mutex::new(0)),
        #[cfg(target_os = "windows")]
        last_external_handle: Arc::new(Mutex::new(0)),
        recently_deleted_sigs: Arc::new(Mutex::new(HashSet::new())),
    };
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec![]))) 
        .manage(app_state.clone())
        .plugin(ShortcutBuilder::new()
            .with_shortcut("Alt+V").expect("shortcut error")
            .with_shortcut("Alt+Q").expect("shortcut error")
            .with_handler(|app, shortcut, event| {
                if event.state == ShortcutState::Pressed && shortcut.matches(Modifiers::ALT, Code::KeyV) {
                    if let Some(w) = app.get_webview_window("main") {
                        if w.is_visible().unwrap_or(false) {
                            let _ = w.hide();
                        } else {
                            let follow_mouse = read_setting_sync(app, "follow_mouse");
                            if follow_mouse == "true" {
                                position_window_at_mouse(&w);
                            } else {
                                position_window(&w);
                            }
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                }

                if event.state == ShortcutState::Pressed && shortcut.matches(Modifiers::ALT, Code::KeyQ) {
                    let _ = app.emit("trigger-queue-paste", ());
                }
            })
            .build())
        .on_window_event(|window, event| { if let WindowEvent::CloseRequested { api, .. } = event { window.hide().unwrap(); api.prevent_close(); } })
        .setup(move |app| {
            let handle = app.handle().clone(); 
            let db = get_db_path(&handle);
            init_db(&db);
            {
                let saved = read_setting_sync(&handle, "file_save_path");
                if saved.is_empty() == false {
                    if let Ok(mut p) = app_state.file_save_path.lock() {
                        *p = PathBuf::from(saved);
                    }
                }
            }
            if let Some(w) = app.get_webview_window("main") { position_window(&w); #[cfg(target_os = "windows")] let _ = apply_acrylic(&w, Some((0, 0, 0, 0))); }
            
            window::build_tray(app)?;

            let server_handle = handle.clone(); let state_server = app_state.clone(); 
            tauri::async_runtime::spawn(async move { let _ = HttpServer::new(move || { App::new().app_data(web::Data::new(state_server.clone())).app_data(web::Data::new(server_handle.clone())).app_data(web::PayloadConfig::new(50 * 1024 * 1024)).configure(server::configure_server) }).bind(("0.0.0.0", 19527)).expect("port error").run().await; });

            // Step 2: 启动后台雷达轮询线程（持续捕获外部窗口句柄）
            #[cfg(target_os = "windows")]
            {
                let state_radar = app_state.clone();
                thread::spawn(move || {
                    let current_pid = unsafe { GetCurrentProcessId() };
                    loop {
                        unsafe {
                            let hwnd = GetForegroundWindow();
                            if !hwnd.0.is_null() {
                                let mut window_pid = 0u32;
                                let _ = GetWindowThreadProcessId(hwnd, Some(&mut window_pid));

                                // 如果前台窗口不是 ClipFlow 自己，更新句柄
                                if window_pid != 0 && window_pid != current_pid {
                                    if let Ok(mut last) = state_radar.last_external_handle.lock() {
                                        *last = hwnd.0 as isize;
                                    }
                                }
                            }
                        }
                        thread::sleep(Duration::from_millis(250));
                    }
                });
            }

            spawn_clipboard_supervisor(app_state.clone(), handle.clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_history, delete_item, clear_history, write_clipboard, set_category, get_local_ip, get_all_settings, save_setting, update_history_content, translate_text, send_to_phone, copy_image_to_clipboard, smart_copy, trigger_paste, paste_item, set_queue, paste_queue_next, upload_file, get_file_save_path, set_save_path, restart_clipboard_monitor, force_sync])
        .run(tauri::generate_context!())
        .expect("error");
}
