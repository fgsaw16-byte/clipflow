use crate::clipboard::write_to_clipboard_inner;
use crate::db::{detect_category, get_db_path, signature_for};
use crate::state::{safe_lock, AppState, HistoryItem};
use rusqlite::{params, Connection};
use tauri::AppHandle;

#[tauri::command]
pub fn get_history(app: AppHandle) -> Vec<HistoryItem> {
    let db = get_db_path(&app);
    if let Ok(c) = Connection::open(db) {
        let mut s = c
            .prepare("SELECT id, content, created_at, category FROM history ORDER BY id DESC")
            .unwrap();
        s.query_map([], |r| {
            Ok(HistoryItem {
                id: r.get(0)?,
                content: r.get(1)?,
                created_at: r.get(2)?,
                category: r.get(3).unwrap_or("text".into()),
            })
        })
        .unwrap()
        .map(|i| i.unwrap())
        .collect()
    } else {
        Vec::new()
    }
}
#[tauri::command]
pub fn set_category(app: AppHandle, id: i64, category: String) -> Result<(), String> {
    let db = get_db_path(&app);
    Connection::open(db)
        .map_err(|e| e.to_string())?
        .execute(
            "UPDATE history SET category = ?1 WHERE id = ?2",
            params![category, id],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}
#[tauri::command]
pub fn delete_item(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    id: i64,
) -> Result<(), String> {
    let db = get_db_path(&app);
    let conn = Connection::open(&db).map_err(|e| e.to_string())?;
    // Record the content signature before deleting, so force_sync won't resurrect it.
    if let Ok(content) = conn.query_row(
        "SELECT content FROM history WHERE id = ?1",
        params![id],
        |r| r.get::<_, String>(0),
    ) {
        safe_lock(&state.recently_deleted_sigs).insert(signature_for(&content));
    }
    conn.execute("DELETE FROM history WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}
#[tauri::command]
pub fn clear_history(app: AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let db = get_db_path(&app);
    Connection::open(db)
        .map_err(|e| e.to_string())?
        .execute("DELETE FROM history", [])
        .map_err(|e| e.to_string())?;
    safe_lock(&state.recently_deleted_sigs).clear();
    Ok(())
}
#[tauri::command]
pub fn update_history_content(app: AppHandle, id: i64, content: String) -> Result<(), String> {
    let db = get_db_path(&app);
    let conn = Connection::open(db).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE history SET content = ?1, category = ?2 WHERE id = ?3",
        params![content, detect_category(&content), id],
    )
    .map_err(|e| e.to_string())?;
    if !content.starts_with("data:image") {
        let _ = write_to_clipboard_inner(&content);
    }
    Ok(())
}
