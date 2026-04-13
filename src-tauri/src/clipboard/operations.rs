use arboard::{Clipboard, ImageData};
use base64::{engine::general_purpose, Engine as _};
use image::ImageOutputFormat;
use rusqlite::{params, Connection};
use std::io::Cursor;
use tauri::{AppHandle, Emitter};

use crate::db::{detect_category, get_db_path, read_setting_sync, signature_for};
use crate::state::{safe_lock, AppState};

#[allow(dead_code)]
pub fn write_image_bytes_to_clipboard(bytes: &[u8]) -> Result<(), String> {
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

pub fn write_to_clipboard_inner(content: &str) -> Result<(), String> {
    use std::borrow::Cow;

    let mut cb = Clipboard::new().map_err(|e| e.to_string())?;

    if content.starts_with("data:image") {
        let base64_data = content
            .splitn(2, ',')
            .nth(1)
            .ok_or_else(|| "Invalid image data URL".to_string())?;

        let clean_base64 = base64_data.replace("\r\n", "").replace('\n', "");

        let bytes = general_purpose::STANDARD
            .decode(&clean_base64)
            .map_err(|e| format!("Base64 decode error: {}", e))?;

        let img =
            image::load_from_memory(&bytes).map_err(|e| format!("Image load error: {}", e))?;
        let rgba8 = img.to_rgba8();
        let (w, h) = rgba8.dimensions();

        let img_data = ImageData {
            width: w as usize,
            height: h as usize,
            bytes: Cow::Owned(rgba8.into_raw()),
        };

        cb.set_image(img_data)
            .map_err(|e| format!("Clipboard set_image error: {}", e))?;

        return Ok(());
    }

    cb.set_text(content.to_string())
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Read the current clipboard content using arboard and persist/notify if it is new.
/// Returns true if a new item was recorded.
/// `last_seq` is updated in-place on Windows to track sequence number across calls.
#[cfg(target_os = "windows")]
pub fn read_and_persist_clipboard(
    state: &AppState,
    handle: &AppHandle,
    cb: &mut Clipboard,
    last_seq: &mut u32,
    last_img: &mut usize,
) -> bool {
    let now_ms = chrono::Local::now().timestamp_millis();

    {
        let t = safe_lock(&state.last_clipboard_write_ms);
        if *t > 0 && now_ms.saturating_sub(*t) < 800 {
            return false;
        }
    }
    {
        let skip = safe_lock(&state.skip_monitor);
        if *skip {
            return false;
        }
    }
    if *safe_lock(&state.is_internal_pasting) {
        return false;
    }

    let mut has_new = false;
    let mut new_c = String::new();
    let mut cat = "text".to_string();

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
            )
            .is_ok()
            {
                new_c = format!(
                    "data:image/png;base64,{}",
                    general_purpose::STANDARD.encode(b)
                );
                cat = "image".to_string();
                has_new = true;
            }
        }
    }

    if !has_new {
        if let Ok(t) = cb.get_text() {
            if !t.is_empty() {
                let seq =
                    unsafe { windows::Win32::System::DataExchange::GetClipboardSequenceNumber() };
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

    if !has_new {
        return false;
    }

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
    if should_ignore {
        return false;
    }

    let privacy = read_setting_sync(handle, "privacy_mode");
    if privacy != "true" {
        let db = get_db_path(handle);
        if let Ok(conn) = Connection::open(db) {
            let _ = conn.execute("DELETE FROM history WHERE content = ?1", params![new_c]);
            let _ = conn.execute(
                "INSERT INTO history (content, category) VALUES (?1, ?2)",
                params![new_c, cat],
            );
            let limit: i64 = read_setting_sync(handle, "history_limit")
                .parse()
                .unwrap_or(200);
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
