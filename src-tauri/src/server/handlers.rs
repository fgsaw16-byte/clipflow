use actix_web::{get, post, web, HttpResponse, Responder};
use actix_multipart::Multipart;
use futures_util::{StreamExt, TryStreamExt};
use tauri::{AppHandle, Emitter};
use arboard::{Clipboard, ImageData};
use std::{thread, time::Duration, sync::Arc};
use std::fs;
use std::path::PathBuf;
use std::hash::{Hash, Hasher};
use std::panic;
use tokio_stream::wrappers::BroadcastStream;
use base64::{Engine as _, engine::general_purpose};
use image::ImageOutputFormat;
use notify_rust::Notification;
use rusqlite::{params, Connection};
use serde_json;
use crate::state::{AppState, safe_lock};
use crate::db::get_db_path;
use crate::clipboard::write_to_clipboard_inner;

#[get("/events")]
pub async fn sse_events(state: web::Data<AppState>) -> impl Responder {
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

pub(crate) fn broadcast_event(state: &AppState, value: serde_json::Value) {
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

#[post("/upload_file")]
pub async fn receive_file(
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
pub async fn receive_image(
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
pub async fn receive_data(body: String, app: web::Data<AppHandle>, state: web::Data<AppState>) -> impl Responder {
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
