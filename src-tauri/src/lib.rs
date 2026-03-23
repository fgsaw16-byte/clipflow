use tauri::{Manager, Emitter, AppHandle, Size, LogicalSize, WebviewWindow, WindowEvent};
use arboard::{Clipboard, ImageData}; 
use std::{thread, time::Duration, sync::{Mutex, Arc, atomic::{AtomicBool, Ordering}}, collections::HashMap};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt; 

use rusqlite::{params, Connection};
use tauri_plugin_global_shortcut::{Builder as ShortcutBuilder, ShortcutState, Code, Modifiers}; 
use serde::{Serialize, Deserialize};
use std::io::Cursor;
use base64::{Engine as _, engine::general_purpose};
use image::ImageOutputFormat;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent, MouseButton};
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use actix_multipart::Multipart;
use futures_util::{StreamExt, TryStreamExt};
use local_ip_address::local_ip;
use tauri_plugin_autostart::MacosLauncher;
use reqwest;
use serde_json;
use image::io::Reader as ImageReader;
use notify_rust::Notification;
use std::hash::{Hash, Hasher};

#[cfg(target_os = "windows")]
use window_vibrancy::apply_acrylic;

#[cfg(target_os = "windows")]
use enigo::{Enigo, Key, KeyboardControllable};

#[cfg(target_os = "windows")]
use windows::Win32::{
    Foundation::{HWND, HINSTANCE, POINT, LPARAM, WPARAM, LRESULT},
    System::DataExchange::{GetClipboardSequenceNumber, AddClipboardFormatListener, RemoveClipboardFormatListener},
    System::Threading::{GetCurrentProcessId, GetCurrentThreadId},
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::{
        GetCursorPos, GetForegroundWindow, GetWindowThreadProcessId, IsIconic, IsWindow,
        SetForegroundWindow, ShowWindow, SW_RESTORE,
        CreateWindowExW, DestroyWindow, DefWindowProcW, RegisterClassExW,
        GetMessageW, TranslateMessage, DispatchMessageW, PostMessageW,
        SetTimer, KillTimer,
        WNDCLASSEXW, MSG, CS_HREDRAW, CS_VREDRAW,
        HWND_MESSAGE, WM_CLIPBOARDUPDATE, WM_APP, WM_TIMER, WM_POWERBROADCAST,
        CW_USEDEFAULT, WINDOW_EX_STYLE, WINDOW_STYLE,
    },
};

/// PBT_APMRESUMEAUTOMATIC: system has resumed from sleep/hibernate
#[cfg(target_os = "windows")]
const PBT_APMRESUMEAUTOMATIC: usize = 0x0012;
/// PBT_APMRESUMESUSPEND: user-initiated resume from sleep
#[cfg(target_os = "windows")]
const PBT_APMRESUMESUSPEND: usize = 0x0007;

use std::panic;

#[cfg(target_os = "windows")]
#[link(name = "user32")]
extern "system" {
    fn AttachThreadInput(id_attach: u32, id_attach_to: u32, f_attach: i32) -> i32;
}

#[tauri::command]
fn set_save_path(app: AppHandle, state: tauri::State<AppState>, path: String) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("路径为空".to_string());
    }
    let pb = PathBuf::from(path.clone());
    fs::create_dir_all(&pb).map_err(|e| e.to_string())?;

    {
        let db = get_db_path(&app);
        let conn = Connection::open(db).map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('file_save_path', ?1)",
            params![path.clone()],
        )
        .map_err(|e| e.to_string())?;
    }

    if let Ok(mut lock) = state.file_save_path.lock() {
        *lock = pb;
    }

    Ok(())
}

#[get("/events")]
async fn sse_events(state: web::Data<AppState>) -> impl Responder {
    let rx = state.event_tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|msg| async move {
        match msg {
            Ok(data) => Some(Ok::<web::Bytes, actix_web::Error>(web::Bytes::from(format!(
                "event: clipboard-update\ndata: {}\n\n",
                data
            )))),
            Err(_) => None,
        }
    });

    HttpResponse::Ok()
        .insert_header(("Content-Type", "text/event-stream"))
        .insert_header(("Cache-Control", "no-cache"))
        .insert_header(("Connection", "keep-alive"))
        .streaming(stream)
}

fn broadcast_event(state: &AppState, value: serde_json::Value) {
    let _ = state.event_tx.send(value.to_string());
}

fn get_configured_save_path(app: &AppHandle, state: &AppState) -> PathBuf {
    if let Ok(p) = state.file_save_path.lock() {
        if p.as_os_str().is_empty() == false {
            return p.clone();
        }
    }

    let db = get_db_path(app);
    if let Ok(conn) = Connection::open(db) {
        if let Ok(path) = conn.query_row(
            "SELECT value FROM settings WHERE key = 'file_save_path'",
            [],
            |row| row.get::<_, String>(0),
        ) {
            if path.is_empty() == false {
                return PathBuf::from(path);
            }
        }
    }

    dirs::download_dir().unwrap_or_else(|| std::env::temp_dir())
}

fn ensure_unique_path(base: &PathBuf, filename: &str) -> PathBuf {
    let mut final_path = base.join(filename);
    let stem = final_path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(String::from)
        .unwrap_or_else(|| "file".to_string());
    let ext = final_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e))
        .unwrap_or_default();
    let mut counter = 1;

    while final_path.exists() {
        let new_name = format!("{}({}){}", stem, counter, ext);
        final_path = base.join(new_name);
        counter += 1;
    }

    final_path
}

fn write_image_bytes_to_clipboard(bytes: &[u8]) -> Result<(), String> {
    use std::borrow::Cow;
    let img = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
    let rgba8 = img.to_rgba8();
    let (w, h) = rgba8.dimensions();
    let img_data = ImageData {
        width: w as usize,
        height: h as usize,
        bytes: Cow::Owned(rgba8.into_raw()),
    };
    let mut cb = Clipboard::new().map_err(|e| e.to_string())?;
    cb.set_image(img_data).map_err(|e| e.to_string())?;
    Ok(())
}

const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Safely acquire a Mutex, recovering from poisoning.
/// If the Mutex was poisoned by a panicked thread, we still get the inner data.
fn safe_lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| {
        eprintln!("[clipflow] Recovered from poisoned Mutex");
        poisoned.into_inner()
    })
}

#[derive(Serialize, Deserialize, Clone)]
struct HistoryItem { id: i64, content: String, created_at: String, category: String }

#[derive(Clone)]
struct AppState { 
    last_content: Arc<Mutex<String>>,
    ignore_signature: Arc<Mutex<Option<String>>>,
    skip_monitor: Arc<Mutex<bool>>,
    last_clipboard_write_ms: Arc<Mutex<i64>>,
    last_image_upload_hash: Arc<Mutex<u64>>,
    last_image_upload_ms: Arc<Mutex<i64>>,
    paste_queue: Arc<Mutex<Vec<i64>>>,
    is_internal_pasting: Arc<Mutex<bool>>,
    file_save_path: Arc<Mutex<PathBuf>>,
    event_tx: broadcast::Sender<String>,
    /// true = monitor thread is alive and listening; false = crashed/not started
    monitor_alive: Arc<AtomicBool>,
    /// Stores the HWND of the current clipboard listener message window (as isize).
    /// Used by restart_clipboard_monitor to send WM_APP and trigger a clean restart.
    #[cfg(target_os = "windows")]
    monitor_hwnd: Arc<Mutex<isize>>,
    #[cfg(target_os = "windows")]
    last_external_handle: Arc<Mutex<isize>>,
}


fn position_window(window: &WebviewWindow) {
    if let Ok(Some(monitor)) = window.current_monitor() {
        let screen_size = monitor.size();
        let scale_factor = monitor.scale_factor();
        let logical_width = 380.0; let logical_height = 430.0;
        let physical_width = (logical_width * scale_factor) as i32;
        let physical_height = (logical_height * scale_factor) as i32;
        let margin = (20.0 * scale_factor) as i32;
        let taskbar_allowance = (40.0 * scale_factor) as i32;
        let x = screen_size.width as i32 - physical_width - margin;
        let y = screen_size.height as i32 - physical_height - taskbar_allowance;
        let _ = window.set_size(Size::Logical(LogicalSize { width: logical_width, height: logical_height }));
        let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
    }
}

fn position_window_at_mouse(window: &WebviewWindow) {
    if let Ok(Some(monitor)) = window.current_monitor() {
        let screen_size = monitor.size();
        let scale_factor = monitor.scale_factor();

        let logical_width = 380.0;
        let logical_height = 430.0;
        let physical_width = (logical_width * scale_factor) as i32;
        let physical_height = (logical_height * scale_factor) as i32;

        #[cfg(target_os = "windows")]
        let (mx, my) = {
            let mut pt = POINT { x: 0, y: 0 };
            unsafe {
                if GetCursorPos(&mut pt).is_ok() {
                    (pt.x, pt.y)
                } else {
                    position_window(window);
                    return;
                }
            }
        };

        #[cfg(not(target_os = "windows"))]
        let (mx, my) = {
            position_window(window);
            return;
        };

        let anchor_x = (120.0 * scale_factor) as i32;
        let anchor_y = (78.0 * scale_factor) as i32;
        let mut x = mx - anchor_x;
        let mut y = my - anchor_y;

        // Clamp to screen bounds
        let max_x = screen_size.width as i32 - physical_width;
        let max_y = screen_size.height as i32 - physical_height;
        if x < 0 { x = 0; }
        if y < 0 { y = 0; }
        if x > max_x { x = max_x; }
        if y > max_y { y = max_y; }

        let _ = window.set_size(Size::Logical(LogicalSize { width: logical_width, height: logical_height }));
        let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
    }
}

fn get_db_path(app: &AppHandle) -> PathBuf {
    let app_data_dir = app.path().app_data_dir().unwrap_or(PathBuf::from("."));
    if !app_data_dir.exists() { let _ = fs::create_dir_all(&app_data_dir); }
    app_data_dir.join("history.db")
}

fn init_db(db_path: &PathBuf) {
    let conn = Connection::open(db_path).expect("DB Open Error");
    conn.execute("CREATE TABLE IF NOT EXISTS history (id INTEGER PRIMARY KEY, content TEXT NOT NULL, created_at DATETIME DEFAULT CURRENT_TIMESTAMP, category TEXT DEFAULT 'text')", []).unwrap_or_default();
    conn.execute("CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT)", []).unwrap_or_default();
    let defaults = [("history_limit", "200"), ("server_port", "19527"), ("privacy_mode", "false"), ("shortcut", "Alt+V"), ("theme", "system"), ("disable_search", "false"), ("file_save_path", ""), ("follow_mouse", "false")];
    for (k, v) in defaults { conn.execute("INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)", params![k, v]).unwrap_or_default(); }
}

fn detect_category(content: &str) -> String {
    if content.starts_with("data:image") { return "image".to_string(); }
    let kws = ["function", "const ", "let ", "var ", "import ", "class ", "def ", "return", "<html>", "<?php", "public "];
    for kw in kws { if content.contains(kw) { return "code".to_string(); } }
    if content.contains(";") && content.contains("{") && content.contains("}") { return "code".to_string(); }
    "text".to_string()
}

fn signature_for(content: &str) -> String {
    content.chars().take(100).collect()
}

fn write_to_clipboard_inner(content: &str) -> Result<(), String> {
    use std::borrow::Cow;
    
    let mut cb = Clipboard::new().map_err(|e| e.to_string())?;
    
    if content.starts_with("data:image") {
        // Extract base64 data from data URL
        let base64_data = content
            .splitn(2, ',')
            .nth(1)
            .ok_or_else(|| "Invalid image data URL".to_string())?;
        
        // Remove possible newlines
        let clean_base64 = base64_data.replace("\r\n", "").replace('\n', "");
        
        // Decode base64 to bytes
        let bytes = general_purpose::STANDARD
            .decode(&clean_base64)
            .map_err(|e| format!("Base64 decode error: {}", e))?;
        
        // Load image and convert to RGBA8
        let img = image::load_from_memory(&bytes).map_err(|e| format!("Image load error: {}", e))?;
        let rgba8 = img.to_rgba8();
        let (w, h) = rgba8.dimensions();
        
        // Create ImageData for arboard
        let img_data = ImageData {
            width: w as usize,
            height: h as usize,
            bytes: Cow::Owned(rgba8.into_raw()),
        };
        
        // Write to clipboard
        cb.set_image(img_data).map_err(|e| format!("Clipboard set_image error: {}", e))?;
        
        return Ok(());
    }
    
    // Text content
    cb.set_text(content.to_string()).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn smart_copy(app: AppHandle, state: tauri::State<'_, AppState>, id: i64) -> Result<(), String> {
    if let Ok(mut skip) = state.skip_monitor.lock() { *skip = true; }
    let db = get_db_path(&app);
    let conn = Connection::open(db).map_err(|e| e.to_string())?;
    let content: String = conn.query_row("SELECT content FROM history WHERE id = ?1", params![id], |row| row.get(0)).map_err(|e| e.to_string())?;

    if content.starts_with("data:image") {
        if let Some(comma_index) = content.find(',') {
            let base64_data = &content[comma_index + 1..];
            let clean_base64 = base64_data.replace("\r\n", "").replace("\n", "");
            if let Ok(bytes) = general_purpose::STANDARD.decode(&clean_base64) {
                let temp_dir = std::env::temp_dir();
                let file_path = temp_dir.join(format!("clipflow_tmp_{}.png", id));
                if fs::write(&file_path, bytes).is_ok() {
                    #[cfg(target_os = "windows")]
                    {
                        let path_str = file_path.to_string_lossy().to_string();
                        let ps_cmd = format!("Set-Clipboard -Path '{}'", path_str);
                        let _ = Command::new("powershell").args(&["-Command", &ps_cmd]).creation_flags(CREATE_NO_WINDOW).spawn();
                        return Ok(());
                    }
                }
            }
        }
        return Err("图片处理失败".to_string());
    } else {
        let mut cb = Clipboard::new().map_err(|e| e.to_string())?;
        cb.set_text(content).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// 🔥🔥🔥 修复重点：更稳健的粘贴逻辑 🔥🔥🔥
#[tauri::command]
fn trigger_paste() {
    #[cfg(target_os = "windows")]
    thread::spawn(|| {
        // 等待窗口隐藏完成，焦点完全切换
        thread::sleep(Duration::from_millis(200)); 
        let mut enigo = Enigo::new();
        
        // 1. 按下 Ctrl
        enigo.key_down(Key::Control);
        // 给一点点时间让系统识别修饰键
        thread::sleep(Duration::from_millis(50)); 
        
        // 2. 点击 V
        enigo.key_click(Key::Layout('v'));
        
        // 3. 保持一小会儿再松开
        thread::sleep(Duration::from_millis(50)); 
        enigo.key_up(Key::Control);
    });
}

#[tauri::command]
async fn paste_item_inner(app: AppHandle, state: AppState, id: i64) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::borrow::Cow;
        use std::ffi::c_void;

        // Step A: 从 DB 读取内容并生成指纹锁
        let db = get_db_path(&app);
        let conn = Connection::open(db).map_err(|e| e.to_string())?;
        let content: String = conn
            .query_row("SELECT content FROM history WHERE id = ?1", params![id], |row| row.get(0))
            .map_err(|e| e.to_string())?;

        let sig = signature_for(&content);
        if let Ok(mut lock) = state.ignore_signature.lock() {
            *lock = Some(sig);
        }

        // Step B: 初始化 Enigo（单实例）
        let mut enigo = Enigo::new();

        {
            if let Ok(mut pasting) = state.is_internal_pasting.lock() {
                *pasting = true;
            }
        }
        thread::sleep(Duration::from_millis(20));

        let write_result = (|| -> Result<(), String> {
            const MAX_RETRIES: usize = 3;
            let mut last_err: Option<String> = None;
            for _ in 0..MAX_RETRIES {
                match Clipboard::new() {
                    Ok(mut clipboard) => {
                        if content.starts_with("data:image") {
                            let base64_data = content
                                .splitn(2, ',')
                                .nth(1)
                                .ok_or_else(|| "图片数据格式错误".to_string())?;
                            let clean_base64 = base64_data.replace("\r\n", "").replace('\n', "");
                            let bytes = general_purpose::STANDARD
                                .decode(&clean_base64)
                                .map_err(|e| e.to_string())?;
                            let size = bytes.len();
                            let wait_time = if size > 1_000_000 { 1000 } else { 200 };

                            let img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
                            let rgba8 = img.to_rgba8();
                            let (w, h) = rgba8.dimensions();
                            let img_data = ImageData {
                                width: w as usize,
                                height: h as usize,
                                bytes: Cow::Owned(rgba8.into_raw()),
                            };
                            if let Err(e) = clipboard.set_image(img_data) {
                                last_err = Some(e.to_string());
                            } else {
                                thread::sleep(Duration::from_millis(wait_time));
                                return Ok(());
                            }
                        } else {
                            if let Err(e) = clipboard.set_text(content.clone()) {
                                last_err = Some(e.to_string());
                            } else {
                                thread::sleep(Duration::from_millis(100));
                                return Ok(());
                            }
                        }
                    }
                    Err(e) => last_err = Some(e.to_string()),
                }
                thread::sleep(Duration::from_millis(50));
            }
            Err(last_err.unwrap_or_else(|| "Clipboard write failed".to_string()))
        })();

        if let Ok(mut pasting) = state.is_internal_pasting.lock() {
            *pasting = false;
        }

        if let Err(e) = write_result {
            println!("CRITICAL ERROR: Clipboard write failed: {}", e);
            return Err(e);
        }

        // Step C: 智能焦点切换
        let raw_hwnd_val = {
            let guard = safe_lock(&state.last_external_handle);
            *guard
        };

        if raw_hwnd_val == 0 {
            return Err("未捕获到目标窗口句柄".to_string());
        }

        let target_hwnd = HWND(raw_hwnd_val as *mut c_void);

        unsafe {
            // 验证窗口是否仍然有效
            if IsWindow(Some(target_hwnd)).as_bool() == false {
                return Err("目标窗口已关闭".to_string());
            }

            let current_thread_id = GetCurrentThreadId();
            let mut target_pid = 0u32;
            let target_thread_id = GetWindowThreadProcessId(target_hwnd, Some(&mut target_pid));

            if target_thread_id == 0 {
                return Err("无法获取目标窗口线程".to_string());
            }

            // AttachThreadInput: 绑定输入队列
            let _ = AttachThreadInput(current_thread_id, target_thread_id, 1);

            // 仅当窗口是最小化时才恢复，避免破坏最大化状态
            if IsIconic(target_hwnd).as_bool() {
                let _ = ShowWindow(target_hwnd, SW_RESTORE);
            }

            // 设置前台窗口
            let _ = SetForegroundWindow(target_hwnd);

            // DetachThreadInput: 解绑输入队列
            let _ = AttachThreadInput(current_thread_id, target_thread_id, 0);
        }

        // Step 3.4: 等待窗口响应并模拟粘贴（Alt 消耗策略）
        thread::sleep(Duration::from_millis(100));
        
        // 🔥 核心修复: Alt 消耗法 🔥
        // 在释放 Alt 前，先"点击"一下 Ctrl，欺骗系统 Alt 已被作为修饰键使用过
        enigo.key_down(Key::Control);
        thread::sleep(Duration::from_millis(20));
        enigo.key_up(Key::Control);
        
        // 现在释放 Alt，系统会认为这是 (Alt+Ctrl) 组合键的结束，不会弹菜单
        enigo.key_up(Key::Alt);
        thread::sleep(Duration::from_millis(50));

        // 执行标准的 Ctrl+V 粘贴
        enigo.key_down(Key::Control);
        thread::sleep(Duration::from_millis(50));
        enigo.key_click(Key::Layout('v'));
        thread::sleep(Duration::from_millis(50));
        enigo.key_up(Key::Control);

        return Ok(());
    }

    #[allow(unreachable_code)]
    Err("paste_item 仅支持 Windows".to_string())
}

#[tauri::command]
async fn paste_item(app: AppHandle, state: tauri::State<'_, AppState>, id: i64) -> Result<(), String> {
    paste_item_inner(app, state.inner().clone(), id).await
}

#[tauri::command]
fn set_queue(state: tauri::State<'_, AppState>, ids: Vec<i64>) -> Result<(), String> {
    if let Ok(mut queue) = state.paste_queue.lock() {
        *queue = ids;
        return Ok(());
    }
    Err("queue lock poisoned".to_string())
}

#[tauri::command]
fn paste_queue_next(app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let next_id = if let Ok(mut queue) = state.paste_queue.lock() {
        if queue.is_empty() {
            None
        } else {
            Some(queue.remove(0))
        }
    } else {
        None
    };

    if let Some(id) = next_id {
        let _ = app.emit("queue-consumed", id);
        let app_handle = app.clone();
        let state_clone = state.inner().clone();
        tauri::async_runtime::spawn(async move {
            let _ = paste_item_inner(app_handle, state_clone, id).await;
        });
    }

    Ok(())
}

#[tauri::command]
fn copy_image_to_clipboard(path: String) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    let img = ImageReader::open(&path).map_err(|e| e.to_string())?.decode().map_err(|e| e.to_string())?;
    let rgba8 = img.to_rgba8();
    let (w, h) = rgba8.dimensions();
    let img_data = ImageData { width: w as usize, height: h as usize, bytes: std::borrow::Cow::Borrowed(rgba8.as_raw()) };
    clipboard.set_image(img_data).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn upload_file(filename: String, data: Vec<u8>, app: AppHandle) -> Result<String, String> {
    // Get save path from settings
    let db = get_db_path(&app);
    let conn = Connection::open(db).map_err(|e| e.to_string())?;
    let save_path_str: String = conn.query_row(
        "SELECT value FROM settings WHERE key = 'file_save_path'",
        [],
        |row| row.get(0)
    ).unwrap_or_default();
    
    let base_path: PathBuf = if save_path_str.is_empty() {
        // Use Downloads folder as default
        dirs::download_dir().unwrap_or_else(|| std::env::temp_dir())
    } else {
        PathBuf::from(save_path_str)
    };
    
    // Ensure directory exists
    if !base_path.exists() {
        fs::create_dir_all(&base_path).map_err(|e| format!("无法创建目录: {}", e))?;
    }
    
    // Handle duplicate filenames
    let mut final_path = base_path.join(&filename);
    let stem = final_path.file_stem().and_then(|s| s.to_str()).map(String::from).unwrap_or_else(|| "file".to_string());
    let ext = final_path.extension().and_then(|e| e.to_str()).map(|e| format!(".{}", e)).unwrap_or_default();
    let mut counter = 1;
    
    while final_path.exists() {
        let new_name = format!("{}({}){}", stem, counter, ext);
        final_path = base_path.join(new_name);
        counter += 1;
    }
    
    // Write file
    fs::write(&final_path, &data).map_err(|e| format!("写入文件失败: {}", e))?;
    
    // Show notification
    let file_name_only = final_path.file_name().and_then(|s| s.to_str()).unwrap_or(&filename);
    Notification::new()
        .summary("ClipFlow")
        .body(&format!("文件 [{}] 已保存", file_name_only))
        .timeout(5000)
        .show()
        .map_err(|e| format!("通知失败: {}", e))?;
    
    Ok(final_path.to_string_lossy().to_string())
}

#[tauri::command]
fn get_file_save_path(app: AppHandle) -> String {
    let db = get_db_path(&app);
    if let Ok(conn) = Connection::open(db) {
        if let Ok(path) = conn.query_row(
            "SELECT value FROM settings WHERE key = 'file_save_path'",
            [],
            |row| row.get::<_, String>(0),
        ) {
            if !path.is_empty() {
                return path;
            }
        }
    }

    dirs::download_dir()
        .unwrap_or_else(|| std::env::temp_dir())
        .to_string_lossy()
        .to_string()
}

#[tauri::command]
fn update_history_content(app: AppHandle, id: i64, content: String) -> Result<(), String> {
    let db = get_db_path(&app);
    let conn = Connection::open(db).map_err(|e| e.to_string())?;
    conn.execute("UPDATE history SET content = ?1, category = ?2 WHERE id = ?3", params![content, detect_category(&content), id]).map_err(|e| e.to_string())?;
    if !content.starts_with("data:image") { let _ = write_to_clipboard_inner(&content); }
    Ok(())
}

fn read_setting_sync(handle: &AppHandle, key: &str) -> String {
    let db_path = get_db_path(handle);
    if let Ok(conn) = Connection::open(db_path) { if let Ok(val) = conn.query_row("SELECT value FROM settings WHERE key = ?1", params![key], |r| r.get(0)) { return val; } } "".to_string()
}
async fn translate_google(content: String) -> Result<String, String> {
    let has_chinese = content.chars().any(|c| c >= '\u{4E00}' && c <= '\u{9FFF}');
    let (source, target) = if has_chinese { ("auto", "en") } else { ("auto", "zh-CN") };
    let client = reqwest::Client::builder().timeout(Duration::from_secs(8)).build().map_err(|e| e.to_string())?;
    let params = [("client", "gtx"), ("sl", source), ("tl", target), ("dt", "t"), ("q", &content)];
    let res = client.post("https://translate.googleapis.com/translate_a/single").form(&params).send().await.map_err(|e| e.to_string())?;
    if !res.status().is_success() { return Err(format!("Google服务不可用: {}", res.status())); }
    let json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    let mut result = String::new();
    if let Some(sentences) = json.get(0).and_then(|v| v.as_array()) { for sentence in sentences { if let Some(segment) = sentence.get(0).and_then(|v| v.as_str()) { result.push_str(segment); } } }
    if result.is_empty() { Err("Google翻译结果为空".into()) } else { Ok(result) }
}
#[tauri::command] async fn translate_text(content: String) -> Result<String, String> { if content.trim().is_empty() { return Err("内容为空".into()); } translate_google(content).await }
#[tauri::command]
fn send_to_phone(state: tauri::State<AppState>, content: String) -> Result<(), String> {
    if let Ok(mut last) = state.last_content.lock() {
        *last = content.clone();
    }

    let msg_type = if content.starts_with("data:image") { "image" } else { "text" };
    broadcast_event(
        state.inner(),
        serde_json::json!({
            "from": "pc",
            "type": msg_type,
            "content": content
        }),
    );

    Ok(())
}
/// Force-restart the clipboard monitor thread.
/// On Windows: posts WM_APP to the listener window, causing the monitor to exit cleanly;
/// the supervisor thread will immediately spawn a fresh one.
/// On non-Windows: no-op (the polling monitor is self-healing via the supervisor).
#[tauri::command]
fn restart_clipboard_monitor(state: tauri::State<'_, AppState>) -> Result<(), String> {
    // Always clear suppression flags so monitor isn't permanently stuck
    *safe_lock(&state.skip_monitor) = false;
    *safe_lock(&state.last_clipboard_write_ms) = 0;
    *safe_lock(&state.is_internal_pasting) = false;

    #[cfg(target_os = "windows")]
    {
        use std::ffi::c_void;
        let hwnd_val = *safe_lock(&state.monitor_hwnd);
        if hwnd_val != 0 {
            let hwnd = HWND(hwnd_val as *mut c_void);
            unsafe {
                let _ = PostMessageW(Some(hwnd), WM_APP, WPARAM(0), LPARAM(0));
            }
            println!("[clipflow][clipboard] restart_clipboard_monitor: WM_APP posted to HWND={}", hwnd_val);
        } else {
            // No active HWND — monitor is already dead.
            // Force monitor_alive to false so supervisor knows to respawn.
            state.monitor_alive.store(false, Ordering::SeqCst);
            println!("[clipflow][clipboard] restart_clipboard_monitor: no active HWND, forcing monitor_alive=false");
        }
    }
    Ok(())
}

/// Hard-reset endpoint for the frontend's pull-to-refresh.
///
/// 1. Clears any stuck suppression flags (skip_monitor, is_internal_pasting).
/// 2. Force-reads the current clipboard and persists if new.
/// 3. Signals the Win32 monitor to restart (remove + re-register listener).
/// 4. Returns the latest history so the frontend can update immediately.
///
/// This command is designed to NEVER block: it uses safe_lock to recover from
/// poisoned mutexes and returns an error string instead of panicking.
#[tauri::command]
fn force_sync(app: AppHandle, state: tauri::State<'_, AppState>) -> Result<Vec<HistoryItem>, String> {
    println!("[clipflow][force_sync] ▶ Hard reset triggered");

    // Step 1: Clear ALL suppression flags
    *safe_lock(&state.skip_monitor) = false;
    *safe_lock(&state.last_clipboard_write_ms) = 0;
    *safe_lock(&state.is_internal_pasting) = false;
    *safe_lock(&state.ignore_signature) = None;

    // Step 2: Force-read clipboard (independent of monitor thread)
    #[cfg(target_os = "windows")]
    {
        match Clipboard::new() {
            Ok(mut cb) => {
                let mut last_seq: u32 = 0;
                let mut last_img: usize = 0;
                let _ = read_and_persist_clipboard(state.inner(), &app, &mut cb, &mut last_seq, &mut last_img);
            }
            Err(e) => {
                println!("[clipflow][force_sync] ⚠️ Clipboard::new() failed: {} — continuing anyway", e);
            }
        }
    }

    // Step 3: Restart the monitor
    #[cfg(target_os = "windows")]
    {
        use std::ffi::c_void;
        let hwnd_val = *safe_lock(&state.monitor_hwnd);
        if hwnd_val != 0 {
            let hwnd = HWND(hwnd_val as *mut c_void);
            unsafe {
                let _ = PostMessageW(Some(hwnd), WM_APP, WPARAM(0), LPARAM(0));
            }
            println!("[clipflow][force_sync] Posted WM_APP to HWND={}", hwnd_val);
        } else {
            // Monitor is dead — force supervisor to notice
            state.monitor_alive.store(false, Ordering::SeqCst);
            println!("[clipflow][force_sync] No active HWND, forcing monitor_alive=false for supervisor");
        }
    }

    // Step 4: Return fresh history
    let db = get_db_path(&app);
    let items: Vec<HistoryItem> = if let Ok(conn) = Connection::open(db) {
        let mut stmt = conn.prepare("SELECT id, content, created_at, category FROM history ORDER BY id DESC")
            .map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |r| Ok(HistoryItem {
            id: r.get(0)?,
            content: r.get(1)?,
            created_at: r.get(2)?,
            category: r.get(3).unwrap_or("text".into()),
        })).map_err(|e| e.to_string())?;
        rows.filter_map(|i| i.ok()).collect()
    } else {
        Vec::new()
    };

    println!("[clipflow][force_sync] ✅ Returning {} items", items.len());
    Ok(items)
}

#[tauri::command] fn write_clipboard(content: String) -> Result<(), String> { write_to_clipboard_inner(&content) }
#[tauri::command] fn get_history(app: AppHandle) -> Vec<HistoryItem> { let db=get_db_path(&app); if let Ok(c)=Connection::open(db){let mut s=c.prepare("SELECT id, content, created_at, category FROM history ORDER BY id DESC").unwrap();s.query_map([],|r|Ok(HistoryItem{id:r.get(0)?,content:r.get(1)?,created_at:r.get(2)?,category:r.get(3).unwrap_or("text".into())})).unwrap().map(|i|i.unwrap()).collect()}else{Vec::new()} }
#[tauri::command] fn set_category(app: AppHandle, id: i64, category: String) -> Result<(), String> { let db = get_db_path(&app); Connection::open(db).map_err(|e|e.to_string())?.execute("UPDATE history SET category = ?1 WHERE id = ?2", params![category, id]).map_err(|e|e.to_string())?; Ok(()) }
#[tauri::command] fn delete_item(app: AppHandle, id: i64) -> Result<(), String> { let db = get_db_path(&app); Connection::open(db).map_err(|e|e.to_string())?.execute("DELETE FROM history WHERE id = ?1", params![id]).map_err(|e|e.to_string())?; Ok(()) }
#[tauri::command] fn clear_history(app: AppHandle) -> Result<(), String> { let db = get_db_path(&app); Connection::open(db).map_err(|e|e.to_string())?.execute("DELETE FROM history", []).map_err(|e|e.to_string())?; Ok(()) }
#[tauri::command] fn get_all_settings(app: AppHandle) -> HashMap<String, String> { let db = get_db_path(&app); let mut m = HashMap::new(); if let Ok(c) = Connection::open(db) { let mut s = c.prepare("SELECT key, value FROM settings").unwrap(); let rows = s.query_map([], |r| Ok((r.get::<_,String>(0)?, r.get::<_,String>(1)?))).unwrap(); for r in rows { if let Ok((k,v)) = r { m.insert(k,v); } } } m }
#[tauri::command] fn save_setting(app: AppHandle, key: String, value: String) -> Result<(), String> { let db = get_db_path(&app); Connection::open(db).map_err(|e|e.to_string())?.execute("INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)", params![key, value]).map_err(|e|e.to_string())?; Ok(()) }
#[tauri::command] fn get_local_ip() -> String { if let Ok(ip) = local_ip() { return ip.to_string(); } "127.0.0.1".to_string() }

#[get("/")] async fn web_home(app: web::Data<AppHandle>, state: web::Data<AppState>) -> impl Responder { 
    let _ = app.emit("mobile-connected", "connected");
    if let Ok(mut last) = state.last_content.lock() { *last = String::new(); }
    let html = r###"<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no, viewport-fit=cover">
  <title>ClipFlow</title>
  <style>
    * { box-sizing: border-box; }
    html, body { margin: 0; padding: 0; }
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
      background: #EDEDED;
      height: 100dvh;
      overflow: hidden;
    }
    .app {
      height: 100dvh;
      display: flex;
      flex-direction: column;
    }
    .header {
      flex: 0 0 auto;
      height: 48px;
      display: flex;
      align-items: center;
      justify-content: center;
      background: #F7F7F7;
      border-bottom: 1px solid #E1E1E1;
      font-size: 16px;
      font-weight: 600;
      color: #111;
    }
    .messages {
      flex: 1 1 auto;
      overflow-y: auto;
      padding: 12px 10px;
    }
    .row {
      display: flex;
      margin: 10px 0;
    }
    .row.left { justify-content: flex-start; }
    .row.right { justify-content: flex-end; }
    .bubble {
      max-width: 78%;
      padding: 10px 12px;
      border-radius: 6px;
      font-size: 15px;
      line-height: 1.4;
      color: #111;
      word-break: break-word;
      white-space: pre-wrap;
    }
    .bubble.left {
      background: #fff;
    }
    .bubble.right {
      background: #95ec69;
    }
    .bubble img {
      display: block;
      max-width: 100%;
      border-radius: 6px;
    }
    .file {
      display: flex;
      flex-direction: column;
      gap: 4px;
    }
    .file-name { font-weight: 600; }
    .file-meta { font-size: 12px; opacity: 0.75; }

    .bottom {
      flex: 0 0 auto;
      background: #F7F7F7;
      border-top: 1px solid #D1D5DB;
      padding: 8px;
      display: flex;
      gap: 8px;
      align-items: flex-end;
    }
    .icon {
      width: 34px;
      height: 34px;
      border: none;
      background: transparent;
      font-size: 20px;
      line-height: 34px;
    }
    .input {
      flex: 1;
      background: #fff;
      border: 1px solid #E5E7EB;
      border-radius: 8px;
      padding: 8px 10px;
      font-size: 15px;
      outline: none;
    }
    .send {
      border: none;
      background: #07C160;
      color: #fff;
      border-radius: 6px;
      padding: 8px 12px;
      font-size: 14px;
      font-weight: 600;
      display: none;
    }
    input[type=file] { display: none; }
    .tip {
      text-align: center;
      color: #8c8c8c;
      font-size: 13px;
      padding: 20px 10px;
    }
  </style>
</head>
<body>
  <div class="app">
    <div class="header">ClipFlow</div>
    <div class="messages" id="messages">
      <div class="tip" id="tip">开始发送消息或文件</div>
    </div>
    <div class="bottom">
      <input type="file" id="imageInput" accept="image/*" />
      <input type="file" id="fileInput" />
      <button class="icon" id="btnImage" title="图片">🖼️</button>
      <button class="icon" id="btnFile" title="文件">📎</button>
      <input class="input" id="textInput" placeholder="输入消息" />
      <button class="send" id="sendBtn">发送</button>
    </div>
  </div>

  <script>
    const messages = document.getElementById('messages');
    const tip = document.getElementById('tip');
    const textInput = document.getElementById('textInput');
    const sendBtn = document.getElementById('sendBtn');
    const btnImage = document.getElementById('btnImage');
    const btnFile = document.getElementById('btnFile');
    const imageInput = document.getElementById('imageInput');
    const fileInput = document.getElementById('fileInput');

    function formatSize(bytes) {
      if (!bytes && bytes !== 0) return '';
      const k = 1024;
      const sizes = ['B','KB','MB','GB'];
      const i = Math.floor(Math.log(bytes) / Math.log(k));
      return (bytes / Math.pow(k, i)).toFixed(i === 0 ? 0 : 2) + ' ' + sizes[i];
    }

    function appendMessage(msg) {
      if (tip) tip.remove();

      const row = document.createElement('div');
      row.className = 'row ' + (msg.from === 'pc' ? 'left' : 'right');

      const bubble = document.createElement('div');
      bubble.className = 'bubble ' + (msg.from === 'pc' ? 'left' : 'right');

      if (msg.type === 'image') {
        const img = document.createElement('img');
        img.src = msg.content;
        bubble.appendChild(img);
      } else if (msg.type === 'file') {
        const box = document.createElement('div');
        box.className = 'file';
        const name = document.createElement('div');
        name.className = 'file-name';
        name.textContent = msg.filename || '文件';
        const meta = document.createElement('div');
        meta.className = 'file-meta';
        meta.textContent = (msg.size ? formatSize(msg.size) : '');
        box.appendChild(name);
        box.appendChild(meta);
        bubble.appendChild(box);
      } else {
        bubble.textContent = msg.content || '';
      }

      row.appendChild(bubble);
      messages.appendChild(row);
      messages.scrollTop = messages.scrollHeight;
    }

    function setSendVisible() {
      const hasText = textInput.value.trim().length > 0;
      sendBtn.style.display = hasText ? 'inline-block' : 'none';
    }

    textInput.addEventListener('input', setSendVisible);
    textInput.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        if (textInput.value.trim()) sendText();
      }
    });

    btnImage.addEventListener('click', () => imageInput.click());
    btnFile.addEventListener('click', () => fileInput.click());
    sendBtn.addEventListener('click', sendText);

    async function sendText() {
      const text = textInput.value.trim();
      if (!text) return;
      textInput.value = '';
      setSendVisible();
      try {
        await fetch('/send', { method: 'POST', body: text });
      } catch (e) {
        alert('发送失败');
      }
    }

    imageInput.addEventListener('change', async (e) => {
      const file = e.target.files && e.target.files[0];
      if (!file) return;
      try {
        const fd = new FormData();
        fd.append('file', file);
        const res = await fetch('/upload_image', { method: 'POST', body: fd });
        if (!res.ok) throw new Error('上传失败');
      } catch (err) {
        alert('图片上传失败');
      } finally {
        e.target.value = '';
      }
    });

    fileInput.addEventListener('change', async (e) => {
      const file = e.target.files && e.target.files[0];
      if (!file) return;
      const fd = new FormData();
      fd.append('file', file);
      try {
        const res = await fetch('/upload_file', { method: 'POST', body: fd });
        if (!res.ok) throw new Error('上传失败');
      } catch (err) {
        alert('文件上传失败');
      } finally {
        e.target.value = '';
      }
    });

    // Single source of truth: SSE
    const es = new EventSource('/events');
    es.addEventListener('clipboard-update', (e) => {
      try {
        const msg = JSON.parse(e.data);
        appendMessage(msg);
      } catch (_) {}
    });
  </script>
</body>
</html>"###;
    HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html) 
}
#[post("/upload_file")]
async fn receive_file(
    mut payload: Multipart,
    app: web::Data<AppHandle>,
    state: web::Data<AppState>,
) -> impl Responder {
    let mut filename: Option<String> = None;
    let mut bytes: Vec<u8> = Vec::new();

    while let Ok(Some(mut field)) = payload.try_next().await {
        if field.name() != Some("file") {
            while let Some(chunk) = field.next().await {
                if chunk.is_err() {
                    break;
                }
            }
            continue;
        }

        if filename.is_none() {
            if let Some(cd) = field.content_disposition() {
                if let Some(f) = cd.get_filename() {
                    filename = Some(f.to_string());
                }
            }
        }

        while let Some(chunk) = field.next().await {
            match chunk {
                Ok(data) => bytes.extend_from_slice(&data),
                Err(e) => return HttpResponse::BadRequest().body(format!("读取文件失败: {}", e)),
            }
        }
    }

    let filename = filename.unwrap_or_else(|| "unknown_file".to_string());
    if bytes.is_empty() {
        return HttpResponse::BadRequest().body("Missing file");
    }

    // UI-first (legacy pattern): notify frontend immediately.
    let _ = app.get_ref().emit(
        "new_message",
        serde_json::json!({
            "type": "file",
            "content": format!("收到文件: {}", filename),
            "sender": "mobile",
            "timestamp": chrono::Local::now().timestamp_millis()
        }),
    );

    let base_path = get_configured_save_path(&app, state.get_ref());
    if !base_path.exists() {
        if let Err(e) = fs::create_dir_all(&base_path) {
            return HttpResponse::InternalServerError().body(format!("无法创建目录: {}", e));
        }
    }

    let final_path = ensure_unique_path(&base_path, &filename);
    if let Err(e) = fs::write(&final_path, &bytes) {
        return HttpResponse::InternalServerError().body(format!("写入失败: {}", e));
    }

    let file_name_only = final_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(&filename);

    let _ = Notification::new()
        .summary("ClipFlow")
        .body(&format!("收到文件: {}", file_name_only))
        .timeout(5000)
        .show();

    broadcast_event(
        state.get_ref(),
        serde_json::json!({
            "from": "mobile",
            "type": "file",
            "filename": file_name_only,
            "size": bytes.len(),
            "saved_path": final_path.to_string_lossy().to_string()
        }),
    );

    HttpResponse::Ok().body(final_path.to_string_lossy().to_string())
}

#[post("/upload_image")]
async fn receive_image(
    mut payload: Multipart,
    app: web::Data<AppHandle>,
    state: web::Data<AppState>,
) -> impl Responder {
    let mut bytes: Vec<u8> = Vec::new();

    while let Ok(Some(mut field)) = payload.try_next().await {
        if field.name() != Some("file") {
            while let Some(chunk) = field.next().await {
                if chunk.is_err() {
                    break;
                }
            }
            continue;
        }

        while let Some(chunk) = field.next().await {
            match chunk {
                Ok(data) => bytes.extend_from_slice(&data),
                Err(e) => return HttpResponse::BadRequest().body(format!("读取图片失败: {}", e)),
            }
        }
    }

    if bytes.is_empty() {
        return HttpResponse::BadRequest().body("Missing image");
    }

    // Normalize to PNG so the desktop UI (<img src="data:image/png;base64,...">) can always render.
    let png_bytes: Vec<u8> = match image::load_from_memory(&bytes) {
        Ok(img) => {
            let rgba8 = img.to_rgba8();
            let (w, h) = rgba8.dimensions();
            let mut out: Vec<u8> = Vec::new();
            if image::write_buffer_with_format(
                &mut std::io::Cursor::new(&mut out),
                &rgba8,
                w,
                h,
                image::ColorType::Rgba8,
                ImageOutputFormat::Png,
            )
            .is_ok()
            {
                out
            } else {
                return HttpResponse::BadRequest().body("图片转 PNG 失败");
            }
        }
        Err(e) => {
            return HttpResponse::BadRequest().body(format!("图片解码失败: {}", e));
        }
    };

    let mime = "image/png".to_string();
    let data_url = format!(
        "data:{};base64,{}",
        mime,
        general_purpose::STANDARD.encode(&png_bytes)
    );

    println!(
        "DEBUG: /upload_image data_url prefix='{}' len={} png_bytes={} hash_pending",
        data_url.chars().take(30).collect::<String>(),
        data_url.len(),
        png_bytes.len()
    );

    // Dedupe: if the same image arrives twice within 1s, ignore the second one.
    // This prevents double clipboard writes when the mobile browser re-triggers the upload.
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    let img_hash = hasher.finish();
    let now_ms = chrono::Local::now().timestamp_millis();
    {
        // Lock both fields together to avoid race when two uploads arrive nearly simultaneously.
        let mut last_hash = match state.last_image_upload_hash.lock() {
            Ok(v) => v,
            Err(_) => return HttpResponse::InternalServerError().body("lock poisoned"),
        };
        let mut last_ms = match state.last_image_upload_ms.lock() {
            Ok(v) => v,
            Err(_) => return HttpResponse::InternalServerError().body("lock poisoned"),
        };

        if *last_hash == img_hash && now_ms.saturating_sub(*last_ms) < 1000 {
            println!("⚠️ Dropped duplicate image upload (hash={})", img_hash);
            return HttpResponse::Ok().body("Duplicate dropped");
        }

        *last_hash = img_hash;
        *last_ms = now_ms;
    }

    // Time-window suppression: mark clipboard write start time so monitor skips echo.
    let now_ms = chrono::Local::now().timestamp_millis();
    if let Ok(mut t) = state.last_clipboard_write_ms.lock() {
        *t = now_ms;
    }

    // Pause clipboard monitor during write to prevent 1418 race condition
    if let Ok(mut skip) = state.skip_monitor.lock() {
        *skip = true;
    }

    // Record in history immediately
    {
        let db = get_db_path(&app);
        if let Ok(conn) = Connection::open(db) {
            let _ = conn.execute("DELETE FROM history WHERE content = ?1", params![data_url]);
            let _ = conn.execute(
                "INSERT INTO history (content, category) VALUES (?1, 'image')",
                params![data_url],
            );
            let _ = conn.execute(
                "DELETE FROM history WHERE id NOT IN (SELECT id FROM history ORDER BY id DESC LIMIT 200)",
                [],
            );

            let last_row: Result<(i64, String, String), _> = conn.query_row(
                "SELECT id, category, substr(content, 1, 30) FROM history ORDER BY id DESC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            );
            if let Ok((id, cat, prefix)) = last_row {
                println!("DEBUG: /upload_image DB last id={} cat={} prefix='{}'", id, cat, prefix);
            } else {
                println!("DEBUG: /upload_image DB last row query failed");
            }
        }
    }

    // UI-first: emit after DB insert so the frontend loadHistory() can see it immediately.
    let _ = app.get_ref().emit(
        "new_message",
        serde_json::json!({
            "type": "image",
            "content": data_url,
            "mime": mime,
            "sender": "mobile",
            "timestamp": chrono::Local::now().timestamp_millis()
        }),
    );

    // Transfer assistant chat uses `mobile-msg` to append bubbles.
    // Send the data URL so the chat can render it as an <img>.
    let _ = app.get_ref().emit("mobile-msg", &data_url);

    // Fallback: force a history refresh path used by the desktop UI.
    // This does NOT write DB; it only triggers frontend reload.
    let _ = app.get_ref().emit("clipboard-monitor", &data_url);

    // Fire and Forget: spawn thread for clipboard write
    let bytes_for_thread = Arc::new(png_bytes);
    let state_for_thread = state.get_ref().clone();
    thread::spawn(move || {
        use std::borrow::Cow;

        // Helper: unconditionally release skip_monitor and reset write-time suppression.
        // Called on every exit path so skip_monitor is never leaked.
        // Uses safe_lock to recover even from poisoned mutexes.
        let release_monitor = |state: &AppState| {
            *safe_lock(&state.skip_monitor) = false;
            *safe_lock(&state.last_clipboard_write_ms) = 0;
        };

        // Wrap the entire clipboard write in catch_unwind so a panic
        // can never leak skip_monitor=true and silently kill the monitor.
        let state_ref = &state_for_thread;
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            // Mark again inside the writer thread (more accurate timing).
            let now_ms = chrono::Local::now().timestamp_millis();
            *safe_lock(&state_ref.last_clipboard_write_ms) = now_ms;

            // Small delay to let monitor thread observe skip_monitor=true
            thread::sleep(Duration::from_millis(50));

            let img = image::load_from_memory(&bytes_for_thread)
                .map_err(|e| format!("Image decode failed: {:?}", e))?;
            let rgba8 = img.to_rgba8();
            let (w, h) = rgba8.dimensions();
            let rgba_raw = rgba8.into_raw();

            let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
            let img_obj = ImageData {
                width: w as usize,
                height: h as usize,
                bytes: Cow::Owned(rgba_raw),
            };
            clipboard.set_image(img_obj).map_err(|e| e.to_string())?;
            Ok::<(), String>(())
        }));

        match result {
            Ok(Ok(_)) => {
                println!("✅ Image successfully written to clipboard!");
                thread::sleep(Duration::from_millis(600));
            }
            Ok(Err(e)) => {
                println!("❌ Failed to write clipboard: {}", e);
            }
            Err(_) => {
                eprintln!("❌ [clipflow] PANIC in clipboard image writer thread — releasing locks");
            }
        }

        // Resume monitor on every path (including panic recovery).
        release_monitor(&state_for_thread);
    });

    broadcast_event(
        state.get_ref(),
        serde_json::json!({
            "from": "mobile",
            "type": "image",
            "content": data_url
        }),
    );

    HttpResponse::Ok().body("Image processing started")
}

#[post("/send")]
async fn receive_data(body: String, app: web::Data<AppHandle>, state: web::Data<AppState>) -> impl Responder {
    let handle = app.get_ref().clone();
    if let Ok(mut last) = state.last_content.lock() {
        *last = body.clone();
    }

    // Write to clipboard (clipboard-monitor thread will handle database storage)
    let _ = write_to_clipboard_inner(&body);

    // Notify desktop UI and phone UI via SSE
    let _ = handle.emit("mobile-msg", &body);
    broadcast_event(
        state.get_ref(),
        serde_json::json!({
            "from": "mobile",
            "type": "text",
            "content": body
        }),
    );

    HttpResponse::Ok().body("Received")
}

/// Read the current clipboard content using arboard and persist/notify if it is new.
/// Returns true if a new item was recorded.
/// `last_seq` is updated in-place on Windows to track sequence number across calls.
#[cfg(target_os = "windows")]
fn read_and_persist_clipboard(
    state: &AppState,
    handle: &AppHandle,
    cb: &mut Clipboard,
    last_seq: &mut u32,
    last_img: &mut usize,
) -> bool {
    // ---------- suppression guards ----------
    let now_ms = chrono::Local::now().timestamp_millis();

    {
        let t = safe_lock(&state.last_clipboard_write_ms);
        if *t > 0 && now_ms.saturating_sub(*t) < 800 {
            return false;
        }
    }
    {
        let skip = safe_lock(&state.skip_monitor);
        if *skip { return false; }
    }
    if *safe_lock(&state.is_internal_pasting) {
        return false;
    }

    // ---------- read ----------
    let mut has_new = false;
    let mut new_c = String::new();
    let mut cat = "text".to_string();

    // Try image first
    if let Ok(i) = cb.get_image() {
        let h = i.bytes.len() + i.width;
        if h != *last_img && h > 0 {
            *last_img = h;
            let mut b = Vec::new();
            if image::write_buffer_with_format(
                &mut Cursor::new(&mut b),
                &i.bytes,
                i.width as u32,
                i.height as u32,
                image::ColorType::Rgba8,
                ImageOutputFormat::Png,
            ).is_ok() {
                new_c = format!("data:image/png;base64,{}", general_purpose::STANDARD.encode(b));
                cat = "image".to_string();
                has_new = true;
            }
        }
    }

    // Try text if no image change
    if !has_new {
        if let Ok(t) = cb.get_text() {
            if !t.is_empty() {
                // Use sequence number to detect genuine changes, avoiding echo loops.
                // IMPORTANT: reset last_seq to 0 on monitor restart so first read is
                // always fresh. The sequence number can wrap/reset after system events
                // (hibernate, lock/unlock, remote desktop). We handle that by treating
                // any non-zero seq that differs from last_seq as a genuine change; if
                // seq resets to a value <= last_seq we also treat it as new.
                let seq = unsafe { GetClipboardSequenceNumber() };
                let is_new_seq = seq != 0 && (seq != *last_seq);
                if is_new_seq {
                    *last_seq = seq;
                    new_c = t;
                    cat = detect_category(&new_c);
                    has_new = true;
                    *last_img = 0;
                }
            }
        }
    }

    if !has_new { return false; }

    // ---------- ignore-signature check (prevents echo from our own writes) ----------
    let new_sig = signature_for(&new_c);
    let should_ignore = {
        let mut lock = safe_lock(&state.ignore_signature);
        if lock.as_deref() == Some(&new_sig) {
            *lock = None;
            true
        } else {
            false
        }
    };
    if should_ignore { return false; }

    // ---------- persist and notify ----------
    let privacy = read_setting_sync(handle, "privacy_mode");
    if privacy != "true" {
        let db = get_db_path(handle);
        if let Ok(conn) = Connection::open(db) {
            let _ = conn.execute("DELETE FROM history WHERE content = ?1", params![new_c]);
            let _ = conn.execute(
                "INSERT INTO history (content, category) VALUES (?1, ?2)",
                params![new_c, cat],
            );
            let limit: i64 = read_setting_sync(handle, "history_limit").parse().unwrap_or(200);
            if limit > 0 {
                let _ = conn.execute(
                    "DELETE FROM history WHERE id NOT IN (SELECT id FROM history ORDER BY id DESC LIMIT ?1)",
                    params![limit],
                );
            }
        }
        let _ = handle.emit("clipboard-monitor", &new_c);
    }
    true
}

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
fn run_clipboard_monitor(state: AppState, handle: AppHandle) {
    use windows::core::PCWSTR;

    // Mark alive
    state.monitor_alive.store(true, Ordering::SeqCst);

    // ── 1. Register a minimal window class ───────────────────────────────────
    let class_name: Vec<u16> = "ClipFlowMonitor\0".encode_utf16().collect();

    let hmodule = unsafe { GetModuleHandleW(None).unwrap_or_default() };
    let hinstance = HINSTANCE(hmodule.0);

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM,
    ) -> LRESULT {
        // We must NOT filter here — pass everything through so the loop can inspect.
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

    // ── 2. Create a message-only window (HWND_MESSAGE parent) ────────────────
    let hwnd_result = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            PCWSTR(class_name.as_ptr()),
            PCWSTR(std::ptr::null()),       // no title
            WINDOW_STYLE::default(),
            CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT,
            Some(HWND_MESSAGE),             // message-only; never visible
            None,
            Some(hinstance),
            None,
        )
    };

    let hwnd = match hwnd_result {
        Ok(h) => h,
        Err(e) => {
            eprintln!("\x1b[31m[CLIPFLOW][CRITICAL] Failed to create message window ({})\x1b[0m", e);
            let _ = handle.emit("listener-crashed", serde_json::json!({
                "reason": "window_create_failed",
                "ts_ms": chrono::Local::now().timestamp_millis()
            }));
            state.monitor_alive.store(false, Ordering::SeqCst);
            return;
        }
    };

    // ── 3. Register for clipboard change notifications ────────────────────────
    if let Err(e) = unsafe { AddClipboardFormatListener(hwnd) } {
        eprintln!("\x1b[31m[CLIPFLOW][CRITICAL] AddClipboardFormatListener failed: {}\x1b[0m", e);
        let _ = handle.emit("listener-crashed", serde_json::json!({
            "reason": "add_listener_failed",
            "ts_ms": chrono::Local::now().timestamp_millis()
        }));
        unsafe { let _ = DestroyWindow(hwnd); }
        state.monitor_alive.store(false, Ordering::SeqCst);
        return;
    }

    // Store HWND so restart_clipboard_monitor can signal us
    *safe_lock(&state.monitor_hwnd) = hwnd.0 as isize;

    println!("[clipflow][clipboard] Win32 monitor started (HWND={:?})", hwnd);
    let _ = handle.emit("clipboard-monitor-status", serde_json::json!({
        "state": "running_win32",
        "ts_ms": chrono::Local::now().timestamp_millis()
    }));

    // ── 4. Set a 30-second heartbeat timer ───────────────────────────────────
    // This ensures that even if GetMessageW becomes stuck after hibernate,
    // we will get a WM_TIMER within 30 seconds to probe the clipboard.
    const HEARTBEAT_TIMER_ID: usize = 1;
    const HEARTBEAT_INTERVAL_MS: u32 = 30_000;
    unsafe { SetTimer(Some(hwnd), HEARTBEAT_TIMER_ID, HEARTBEAT_INTERVAL_MS, None) };

    // ── 5. Init arboard for reading ───────────────────────────────────────────
    let mut cb = match Clipboard::new() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("\x1b[31m[CLIPFLOW][CRITICAL] arboard init failed: {} — monitor exiting\x1b[0m", e);
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

    // Initialize last_seq with the current clipboard sequence number so the
    // heartbeat timer doesn't immediately report a "missed" change on startup.
    let mut last_seq: u32 = unsafe { GetClipboardSequenceNumber() };
    let mut last_img: usize = 0;

    /// Helper: remove + re-add clipboard format listener and re-create arboard.
    /// Returns Ok(new_clipboard) or Err (in which case the monitor should exit for restart).
    fn reregister_listener(hwnd: HWND) -> Result<(), String> {
        unsafe {
            let _ = RemoveClipboardFormatListener(hwnd);
            AddClipboardFormatListener(hwnd).map_err(|e| format!("Re-register failed: {}", e))?;
        }
        Ok(())
    }

    // ── 6. Message loop ───────────────────────────────────────────────────────
    // We do NOT filter by hwnd (pass None) so we can receive WM_POWERBROADCAST
    // which is broadcast to all top-level windows (including message-only ones).
    loop {
        let mut msg = MSG::default();
        let ret = unsafe { GetMessageW(&mut msg, None, 0, 0) };

        if ret.0 == 0 {
            // WM_QUIT — clean exit
            break;
        }
        if ret.0 < 0 {
            eprintln!("\x1b[31m[CLIPFLOW][CRITICAL] GetMessage returned error — monitor exiting for restart\x1b[0m");
            break;
        }

        match msg.message {
            // ── Our voluntary shutdown/restart signal ──
            x if x == WM_APP => {
                println!("[clipflow][clipboard] received WM_APP shutdown signal, exiting for restart");
                break;
            }

            // ── Clipboard content changed ──
            x if x == WM_CLIPBOARDUPDATE => {
                thread::sleep(Duration::from_millis(30));
                read_and_persist_clipboard(&state, &handle, &mut cb, &mut last_seq, &mut last_img);
            }

            // ── System resumed from sleep/hibernate ──
            x if x == WM_POWERBROADCAST => {
                let event_type = msg.wParam.0;
                if event_type == PBT_APMRESUMEAUTOMATIC || event_type == PBT_APMRESUMESUSPEND {
                    println!("[clipflow][clipboard] ⚡ System resumed from sleep/hibernate — rebuilding listener");
                    let _ = handle.emit("clipboard-monitor-status", serde_json::json!({
                        "state": "resuming_from_sleep",
                        "ts_ms": chrono::Local::now().timestamp_millis()
                    }));

                    // Brief delay — Windows needs a moment to stabilize after wake
                    thread::sleep(Duration::from_millis(500));

                    // Re-register the clipboard format listener
                    if let Err(e) = reregister_listener(hwnd) {
                        eprintln!("[clipflow][clipboard] ❌ Failed to re-register after resume: {} — exiting for full restart", e);
                        break;
                    }

                    // Re-create arboard (clipboard handles may be stale)
                    match Clipboard::new() {
                        Ok(new_cb) => { cb = new_cb; }
                        Err(e) => {
                            eprintln!("[clipflow][clipboard] ❌ arboard re-init failed after resume: {} — exiting for full restart", e);
                            break;
                        }
                    }

                    // Reset sequence tracking so first post-wake read is always treated as new
                    last_seq = 0;
                    last_img = 0;

                    // Clear any stuck suppression flags
                    *safe_lock(&state.skip_monitor) = false;
                    *safe_lock(&state.last_clipboard_write_ms) = 0;
                    *safe_lock(&state.is_internal_pasting) = false;

                    // Immediate clipboard read
                    read_and_persist_clipboard(&state, &handle, &mut cb, &mut last_seq, &mut last_img);

                    println!("[clipflow][clipboard] ✅ Successfully recovered after sleep/hibernate");
                }
            }

            // ── Heartbeat timer: anti-stuck probe ──
            x if x == WM_TIMER => {
                // Every 30 seconds, re-check the clipboard sequence number.
                // If it changed since our last observation, we missed a WM_CLIPBOARDUPDATE.
                let current_seq = unsafe { GetClipboardSequenceNumber() };
                if current_seq != 0 && current_seq != last_seq {
                    println!("[clipflow][clipboard] ⚠️ Heartbeat detected missed clipboard change (seq {}→{}) — re-reading", last_seq, current_seq);
                    read_and_persist_clipboard(&state, &handle, &mut cb, &mut last_seq, &mut last_img);
                    // Always sync last_seq to current_seq even if read_and_persist
                    // didn't update it (e.g. image content, suppression guards).
                    // This prevents the heartbeat from firing repeatedly for the
                    // same already-observed clipboard state.
                    last_seq = current_seq;
                }
                // Also clear stuck suppression flags periodically
                let now_ms = chrono::Local::now().timestamp_millis();
                {
                    let t = *safe_lock(&state.last_clipboard_write_ms);
                    if t > 0 && now_ms.saturating_sub(t) > 5_000 {
                        // If skip was set more than 5 seconds ago, it's leaked — clear it
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

    // ── 7. Cleanup ────────────────────────────────────────────────────────────
    unsafe {
        let _ = KillTimer(Some(hwnd), HEARTBEAT_TIMER_ID);
        let _ = RemoveClipboardFormatListener(hwnd);
        let _ = DestroyWindow(hwnd);
    }
    *safe_lock(&state.monitor_hwnd) = 0;
    state.monitor_alive.store(false, Ordering::SeqCst);
    println!("[clipflow][clipboard] Win32 monitor thread exited cleanly");
}

/// Non-Windows fallback: arboard polling (unchanged behaviour).
#[cfg(not(target_os = "windows"))]
fn run_clipboard_monitor(state: AppState, handle: AppHandle) {
    state.monitor_alive.store(true, Ordering::SeqCst);

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
            let _ = handle.emit("clipboard-monitor-status",
                serde_json::json!({"state": "running_poll", "ts_ms": now_ms}));
        }

        if let Ok(t) = state.last_clipboard_write_ms.lock() {
            if *t > 0 && now_ms.saturating_sub(*t) < 800 {
                thread::sleep(Duration::from_millis(50));
                continue;
            }
        }
        if let Ok(skip) = state.skip_monitor.lock() {
            if *skip { thread::sleep(Duration::from_millis(50)); continue; }
        }
        if state.is_internal_pasting.lock().map(|g| *g).unwrap_or(false) {
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
                    &mut Cursor::new(&mut b), &i.bytes,
                    i.width as u32, i.height as u32,
                    image::ColorType::Rgba8, ImageOutputFormat::Png,
                ).is_ok() {
                    new_c = format!("data:image/png;base64,{}", general_purpose::STANDARD.encode(b));
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
                if lock.as_deref() == Some(&new_sig) { *lock = None; true } else { false }
            } else { false };

            if !should_ignore {
                let privacy = read_setting_sync(&handle, "privacy_mode");
                if privacy != "true" {
                    let db = get_db_path(&handle);
                    if let Ok(conn) = Connection::open(db) {
                        let _ = conn.execute("DELETE FROM history WHERE content = ?1", params![new_c]);
                        let _ = conn.execute("INSERT INTO history (content, category) VALUES (?1, ?2)", params![new_c, cat]);
                        let _ = conn.execute("DELETE FROM history WHERE id NOT IN (SELECT id FROM history ORDER BY id DESC LIMIT 200)", []);
                    }
                    let _ = handle.emit("clipboard-monitor", &new_c);
                }
            }
        }

        thread::sleep(Duration::from_millis(500));
    }
}

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
            
            let quit_i = MenuItem::with_id(app, "quit", "退出 ClipFlow", true, None::<&str>).unwrap();
            let show_i = MenuItem::with_id(app, "show", "显示主界面", true, None::<&str>).unwrap();
            let menu = Menu::with_items(app, &[&show_i, &quit_i]).unwrap();
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("ClipFlow")
                .menu(&menu)
                .on_menu_event(|app, event| {
                    match event.id().as_ref() {
                        "quit" => app.exit(0),
                        "show" => {
                            if let Some(w) = app.get_webview_window("main") {
                                position_window(&w);
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, .. } = event {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("main") {
                            if w.is_visible().unwrap_or(false) {
                                let _ = w.hide();
                            } else {
                                position_window(&w);
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    }
                })
                .build(app)
                .unwrap();

            let server_handle = handle.clone(); let state_server = app_state.clone(); 
            tauri::async_runtime::spawn(async move { let _ = HttpServer::new(move || { App::new().app_data(web::Data::new(state_server.clone())).app_data(web::Data::new(server_handle.clone())).app_data(web::PayloadConfig::new(50 * 1024 * 1024)).service(web_home).service(sse_events).service(receive_data).service(receive_image).service(receive_file) }).bind(("0.0.0.0", 19527)).expect("port error").run().await; });

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

            let state_clip = app_state.clone(); let state_clip_handle = handle.clone();
            // Outer supervisor thread: restarts the clipboard monitor whenever it exits.
            // - Normal exit (WM_APP signal from restart_clipboard_monitor): clean restart, no crash event.
            // - Panic: emit `listener-crashed` so the frontend can show a recovery UI.
            thread::spawn(move || {
                let mut panic_count: u32 = 0;
                loop {
                    let state_inner = state_clip.clone();
                    let handle_inner = state_clip_handle.clone();
                    let join_handle = thread::spawn(move || {
                        run_clipboard_monitor(state_inner, handle_inner);
                    });
                    match join_handle.join() {
                        Ok(_) => {
                            // Normal exit — either a clean WM_APP restart or init failure.
                            // Only treat it as a crash if monitor_alive was never set to true
                            // (i.e. init failed) — otherwise it was a voluntary restart.
                            let was_alive = state_clip.monitor_alive.load(Ordering::SeqCst);
                            if !was_alive {
                                // Init/window creation failed — treat as crash
                                panic_count += 1;
                                eprintln!("\x1b[31m[CLIPFLOW][CRITICAL] Monitor init failed (crash_count={}) — will retry\x1b[0m", panic_count);
                                let _ = state_clip_handle.emit("listener-crashed", serde_json::json!({
                                    "reason": "init_failed",
                                    "crash_count": panic_count,
                                    "ts_ms": chrono::Local::now().timestamp_millis()
                                }));
                            } else {
                                println!("[clipflow][clipboard] monitor restarted voluntarily (count={})", panic_count);
                                // Reset panic counter on clean voluntary restarts
                                panic_count = 0;
                            }
                        }
                        Err(_) => {
                            // Thread panicked — genuine crash
                            panic_count += 1;
                            eprintln!("\x1b[31m[CLIPFLOW][CRITICAL] !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!\x1b[0m");
                            eprintln!("\x1b[31m[CLIPFLOW][CRITICAL]  CLIPBOARD MONITOR THREAD PANICKED         \x1b[0m");
                            eprintln!("\x1b[31m[CLIPFLOW][CRITICAL]  crash_count={}  — restarting              \x1b[0m", panic_count);
                            eprintln!("\x1b[31m[CLIPFLOW][CRITICAL] !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!\x1b[0m");
                            let _ = state_clip_handle.emit("listener-crashed", serde_json::json!({
                                "reason": "panic",
                                "crash_count": panic_count,
                                "ts_ms": chrono::Local::now().timestamp_millis()
                            }));
                        }
                    }
                    // Safety: ensure suppression flags never remain stuck after any exit.
                    // Use safe_lock to recover even if a panic poisoned the Mutex.
                    *safe_lock(&state_clip.skip_monitor) = false;
                    *safe_lock(&state_clip.last_clipboard_write_ms) = 0;
                    *safe_lock(&state_clip.is_internal_pasting) = false;
                    #[cfg(target_os = "windows")]
                    { *safe_lock(&state_clip.monitor_hwnd) = 0; }
                    // Back-off only on repeated panics; voluntary restarts are immediate
                    let back_off_ms = if panic_count > 0 {
                        (500u64).saturating_mul(panic_count.min(10) as u64)
                    } else {
                        100
                    };
                    thread::sleep(Duration::from_millis(back_off_ms));
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_history, delete_item, clear_history, write_clipboard, set_category, get_local_ip, get_all_settings, save_setting, update_history_content, translate_text, send_to_phone, copy_image_to_clipboard, smart_copy, trigger_paste, paste_item, set_queue, paste_queue_next, upload_file, get_file_save_path, set_save_path, restart_clipboard_monitor, force_sync])
        .run(tauri::generate_context!())
        .expect("error");
}