use crate::db::get_db_path;
use crate::state::AppState;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::{fs, path::PathBuf};
use tauri::AppHandle;

#[tauri::command]
pub fn get_all_settings(app: AppHandle) -> HashMap<String, String> {
    let db = get_db_path(&app);
    let mut m = HashMap::new();
    if let Ok(c) = Connection::open(db) {
        let mut s = c.prepare("SELECT key, value FROM settings").unwrap();
        let rows = s
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .unwrap();
        for r in rows {
            if let Ok((k, v)) = r {
                m.insert(k, v);
            }
        }
    }
    m
}

#[tauri::command]
pub fn save_setting(app: AppHandle, key: String, value: String) -> Result<(), String> {
    let db = get_db_path(&app);
    Connection::open(db)
        .map_err(|e| e.to_string())?
        .execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_file_save_path(app: AppHandle) -> String {
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
pub fn set_save_path(
    app: AppHandle,
    state: tauri::State<AppState>,
    path: String,
) -> Result<(), String> {
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
