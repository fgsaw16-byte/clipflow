#![allow(unused_imports)]

use tauri::{AppHandle, Emitter, Manager};
use arboard::{Clipboard, ImageData};
use std::{thread, time::Duration, sync::Arc};
use std::fs;
use std::process::Command;
use base64::{Engine as _, engine::general_purpose};
use image::io::Reader as ImageReader;
use rusqlite::{params, Connection};
use crate::state::{AppState, HistoryItem, safe_lock, CREATE_NO_WINDOW};
use crate::db::{get_db_path, signature_for};
use crate::clipboard::write_to_clipboard_inner;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
use windows::Win32::{
    Foundation::{HWND, LPARAM, WPARAM},
    System::Threading::{GetCurrentThreadId},
    UI::WindowsAndMessaging::{
        IsIconic, IsWindow, SetForegroundWindow, ShowWindow, SW_RESTORE,
        GetWindowThreadProcessId,
    },
};
#[cfg(target_os = "windows")]
use enigo::{Enigo, Key, KeyboardControllable};

#[cfg(target_os = "windows")]
#[link(name = "user32")]
extern "system" {
    fn AttachThreadInput(id_attach: u32, id_attach_to: u32, f_attach: i32) -> i32;
}

#[tauri::command] pub fn write_clipboard(content: String) -> Result<(), String> { write_to_clipboard_inner(&content) }

#[tauri::command]
pub async fn smart_copy(app: AppHandle, state: tauri::State<'_, AppState>, id: i64) -> Result<(), String> {
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
pub fn trigger_paste() {
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

        let target_hwnd = HWND(raw_hwnd_val as *mut std::ffi::c_void);

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
pub async fn paste_item(app: AppHandle, state: tauri::State<'_, AppState>, id: i64) -> Result<(), String> {
    paste_item_inner(app, state.inner().clone(), id).await
}

#[tauri::command]
pub fn set_queue(state: tauri::State<'_, AppState>, ids: Vec<i64>) -> Result<(), String> {
    if let Ok(mut queue) = state.paste_queue.lock() {
        *queue = ids;
        return Ok(());
    }
    Err("queue lock poisoned".to_string())
}

#[tauri::command]
pub fn paste_queue_next(app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
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
pub fn copy_image_to_clipboard(path: String) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    let img = ImageReader::open(&path).map_err(|e| e.to_string())?.decode().map_err(|e| e.to_string())?;
    let rgba8 = img.to_rgba8();
    let (w, h) = rgba8.dimensions();
    let img_data = ImageData { width: w as usize, height: h as usize, bytes: std::borrow::Cow::Borrowed(rgba8.as_raw()) };
    clipboard.set_image(img_data).map_err(|e| e.to_string())?;
    Ok(())
}
