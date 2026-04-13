use tauri::AppHandle;
use arboard::Clipboard;
use std::{time::Duration, sync::atomic::Ordering};
use rusqlite::{params, Connection};
use reqwest;
use serde_json;
use local_ip_address::local_ip;
use notify_rust::Notification;
use std::{fs, path::PathBuf};
use crate::state::{AppState, HistoryItem, safe_lock};
use crate::db::{get_db_path, detect_category, signature_for};
#[cfg(target_os = "windows")]
use std::ffi::c_void;
#[cfg(target_os = "windows")]
use windows::Win32::{
    Foundation::{HWND, LPARAM, WPARAM},
    UI::WindowsAndMessaging::{PostMessageW, WM_APP},
};

#[tauri::command]
pub async fn upload_file(filename: String, data: Vec<u8>, app: AppHandle) -> Result<String, String> {
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

#[tauri::command]
pub async fn translate_text(content: String) -> Result<String, String> { if content.trim().is_empty() { return Err("内容为空".into()); } translate_google(content).await }

#[tauri::command]
pub fn send_to_phone(state: tauri::State<AppState>, content: String) -> Result<(), String> {
    if let Ok(mut last) = state.last_content.lock() {
        *last = content.clone();
    }

    let msg_type = if content.starts_with("data:image") { "image" } else { "text" };
    crate::server::broadcast_event(
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
pub fn restart_clipboard_monitor(state: tauri::State<'_, AppState>) -> Result<(), String> {
    // Always clear suppression flags so monitor isn't permanently stuck
    *safe_lock(&state.skip_monitor) = false;
    *safe_lock(&state.last_clipboard_write_ms) = 0;
    *safe_lock(&state.is_internal_pasting) = false;

    #[cfg(target_os = "windows")]
    {
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
pub fn force_sync(app: AppHandle, state: tauri::State<'_, AppState>) -> Result<Vec<HistoryItem>, String> {
    println!("[clipflow][force_sync] ▶ Hard reset triggered");

    // Step 1: Clear ALL suppression flags
    *safe_lock(&state.skip_monitor) = false;
    *safe_lock(&state.last_clipboard_write_ms) = 0;
    *safe_lock(&state.is_internal_pasting) = false;
    *safe_lock(&state.ignore_signature) = None;

    // Step 2: Independently read clipboard content.
    // Uses recently_deleted_sigs to prevent ghost resurrection of user-deleted items.
    #[cfg(target_os = "windows")]
    {
        match Clipboard::new() {
            Ok(mut cb) => {
                // Try to read current clipboard text
                if let Ok(text) = cb.get_text() {
                    if !text.is_empty() {
                        let sig = signature_for(&text);
                        let is_deleted = safe_lock(&state.recently_deleted_sigs).contains(&sig);
                        if !is_deleted {
                            let db = get_db_path(&app);
                            if let Ok(conn) = Connection::open(&db) {
                                // Only insert if this exact content is not already the latest entry
                                let already_exists: bool = conn.query_row(
                                    "SELECT COUNT(*) > 0 FROM history WHERE content = ?1",
                                    params![text], |r| r.get(0)
                                ).unwrap_or(true);
                                if !already_exists {
                                    let cat = detect_category(&text);
                                    let _ = conn.execute(
                                        "INSERT INTO history (content, category) VALUES (?1, ?2)",
                                        params![text, cat],
                                    );
                                    println!("[clipflow][force_sync] Inserted missed clipboard content");
                                }
                            }
                        } else {
                            println!("[clipflow][force_sync] Skipped clipboard content (recently deleted by user)");
                        }
                    }
                }
            }
            Err(e) => {
                println!("[clipflow][force_sync] ⚠️ Clipboard::new() failed: {} — continuing anyway", e);
            }
        }
    }

    // Step 3: Nudge the monitor to restart if needed
    #[cfg(target_os = "windows")]
    {
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

    // Step 3: Return fresh history
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

#[tauri::command]
pub fn get_local_ip() -> String { if let Ok(ip) = local_ip() { return ip.to_string(); } "127.0.0.1".to_string() }
