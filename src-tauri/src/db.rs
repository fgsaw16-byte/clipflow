use rusqlite::{params, Connection};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub fn get_db_path(app: &AppHandle) -> PathBuf {
    let app_data_dir = app.path().app_data_dir().unwrap_or(PathBuf::from("."));
    if !app_data_dir.exists() {
        let _ = fs::create_dir_all(&app_data_dir);
    }
    app_data_dir.join("history.db")
}

pub fn init_db(db_path: &PathBuf) {
    let conn = Connection::open(db_path).expect("DB Open Error");
    conn.execute("CREATE TABLE IF NOT EXISTS history (id INTEGER PRIMARY KEY, content TEXT NOT NULL, created_at DATETIME DEFAULT CURRENT_TIMESTAMP, category TEXT DEFAULT 'text')", []).unwrap_or_default();
    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT)",
        [],
    )
    .unwrap_or_default();
    let defaults = [
        ("history_limit", "200"),
        ("server_port", "19527"),
        ("privacy_mode", "false"),
        ("shortcut", "Alt+V"),
        ("theme", "system"),
        ("disable_search", "false"),
        ("file_save_path", ""),
        ("follow_mouse", "false"),
    ];
    for (k, v) in defaults {
        conn.execute(
            "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
            params![k, v],
        )
        .unwrap_or_default();
    }
}

pub fn detect_category(content: &str) -> String {
    if content.starts_with("data:image") {
        return "image".to_string();
    }
    let kws = [
        "function", "const ", "let ", "var ", "import ", "class ", "def ", "return", "<html>",
        "<?php", "public ",
    ];
    for kw in kws {
        if content.contains(kw) {
            return "code".to_string();
        }
    }
    if content.contains(";") && content.contains("{") && content.contains("}") {
        return "code".to_string();
    }
    "text".to_string()
}

pub fn signature_for(content: &str) -> String {
    content.chars().take(100).collect()
}

pub fn read_setting_sync(handle: &AppHandle, key: &str) -> String {
    let db_path = get_db_path(handle);
    if let Ok(conn) = Connection::open(db_path) {
        if let Ok(val) = conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |r| r.get(0),
        ) {
            return val;
        }
    }
    "".to_string()
}
