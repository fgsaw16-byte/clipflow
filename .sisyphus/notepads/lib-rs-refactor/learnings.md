# Notepads: lib-rs-refactor

## Conventions & Patterns

### Key Architecture Rules
- `state.rs` and `db.rs` are the ROOT dependencies - no other project modules depend on them upward
- ALL modules use `pub(crate)` visibility for inter-module access
- `commands/mod.rs` uses `pub use *` re-exports so lib.rs can use `use commands::*` for generate_handler!
- `server/mod.rs` exposes `pub use handlers::broadcast_event` since commands/sync.rs needs it

### Critical Details
- `AppState` is `#[derive(Clone)]` already - no changes needed
- `HistoryItem` is `#[derive(Serialize, Deserialize, Clone)]` - keep those derives
- `safe_lock` function: goes in state.rs since it only uses std::sync::Mutex
- `CREATE_NO_WINDOW: u32 = 0x08000000` goes in state.rs (used by commands/paste.rs)
- `AttachThreadInput` extern "system" declaration: goes in commands/paste.rs (only used there)
- ALL `#[cfg(target_os = "windows")]` guards must be preserved exactly as-is
- `use crate::state::{AppState, HistoryItem, safe_lock};` pattern for modules
- `use crate::db::{get_db_path, detect_category, signature_for, read_setting_sync};` for DB functions

### server/mobile.html
- The HTML content is from lines 882-1136 of lib.rs (inside r###"..."### raw string)
- In mobile_ui.rs, use: `const MOBILE_HTML: &str = include_str!("mobile.html");`
- The include_str! path is relative to the source file location

### supervisor code
- Supervisor lives in run() at lines 2083-2145
- Extract as: `pub fn spawn_clipboard_supervisor(state: AppState, handle: AppHandle)`
- In lib.rs setup() closure, call: `clipboard::supervisor::spawn_clipboard_supervisor(app_state.clone(), handle.clone())`

### Commands in lib.rs invoke_handler
All 23 commands: get_history, delete_item, clear_history, write_clipboard, set_category, get_local_ip, get_all_settings, save_setting, update_history_content, translate_text, send_to_phone, copy_image_to_clipboard, smart_copy, trigger_paste, paste_item, set_queue, paste_queue_next, upload_file, get_file_save_path, set_save_path, restart_clipboard_monitor, force_sync
(Note: paste_item_inner is private, not in generate_handler)

### Imports each module needs
- state.rs: std::sync::{Mutex, Arc, atomic::{AtomicBool}}, std::collections::HashSet, std::path::PathBuf, tokio::sync::broadcast, serde::{Serialize, Deserialize}, #[cfg(target_os = "windows")] windows deps
- db.rs: rusqlite::{params, Connection}, tauri::AppHandle, std::{fs, path::PathBuf}, std::collections::HashMap
- window.rs: tauri::{Manager, AppHandle, WebviewWindow, Size, LogicalSize, menu::{Menu, MenuItem}, tray::{TrayIconBuilder, TrayIconEvent, MouseButton}}, #[cfg(target_os="windows")] {windows::Win32::Foundation::POINT, windows::Win32::UI::WindowsAndMessaging::GetCursorPos, window_vibrancy::apply_acrylic}
- operations.rs: arboard::{Clipboard, ImageData}, base64, image, std::io::Cursor, crate::state, crate::db
- monitor.rs: all windows imports, crate::state, crate::db, crate::clipboard::operations
- supervisor.rs: std::thread, std::time::Duration, crate::state, crate::clipboard::monitor

## Issues & Gotchas

### paste_item_inner signature
- Takes `state: AppState` (not `tauri::State<AppState>`) because it's called both directly and via tauri
- `paste_item` wraps it: `paste_item_inner(app, state.inner().clone(), id).await`
- This pattern must be preserved exactly

### read_setting_sync in db.rs
- This function calls `get_db_path` - both should be in db.rs

### chrono dependency
- Used in server/handlers.rs (receive_file, receive_image timestamps)
- Already in Cargo.toml as dependency

### notify_rust dependency  
- Used in commands/sync.rs (upload_file) and server/handlers.rs (receive_file)
- Already in Cargo.toml

### window.rs - build_tray needs app: &tauri::App
- The tray code uses `app.default_window_icon()` and builds Menu/MenuItem
- build_tray signature: `pub fn build_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>>`
- Or just inline in run() setup if returning from setup closure

### lib.rs setup() still contains:
- AppState initialization (stays in lib.rs)
- DB init call
- Window position + acrylic blur
- build_tray call
- HTTP server spawn
- Foreground radar thread (#[cfg(target_os="windows")])  
- spawn_clipboard_supervisor call

### 2026-04-10 Steps 1-6 extraction notes
- `window.rs` must import `tauri::Manager` so tray callbacks can call `get_webview_window` on `AppHandle`
- `clipboard/monitor.rs` should cfg-gate non-Windows-only imports; otherwise Windows builds accumulate structural refactor warnings
- `clipboard/mod.rs` re-exports for staged extraction can need `#[allow(unused_imports)]` while lib.rs still references a subset directly
- After Steps 1-6, `lib.rs` still owns broad legacy imports; later command/server extraction is what removes the remaining Windows and clipboard-monitor import bulk

### 2026-04-10 Steps 7-8 extraction notes
- `#[tauri::command]` functions moved into `commands/history.rs` and `commands/paste.rs` need `pub fn`/`pub async fn` so `commands/mod.rs` re-exports can feed `use commands::*;` in `lib.rs`
- `commands/mod.rs` can keep `pub use history::*; pub use paste::*;` while `lib.rs` uses bare command names in `generate_handler![]`; add `#[allow(unused_imports)] use commands::*;` during staged extraction to avoid re-export noise
- Moving `AttachThreadInput` into `commands/paste.rs` works cleanly as long as the Windows-only extern block stays beside the paste commands and all existing `#[cfg(target_os = "windows")]` guards remain unchanged

### 2026-04-10 Steps 9-10 extraction notes
- `commands/settings.rs` can take the settings/path commands verbatim from `lib.rs`; `set_save_path` still mutates `state.file_save_path`, so it must import `AppState` even though the other settings commands only touch the DB
- `commands/sync.rs` should temporarily call `crate::broadcast_event(...)` until Step 12 moves that helper into the server module
- After moving settings/sync commands out, `lib.rs` still needs `safe_lock` for the remaining `receive_image` panic-recovery path, so do not drop that import yet

### 2026-04-10 Steps 11-12 extraction notes
- server/mobile_ui.rs can stay as the minimal include_str! wrapper while server/mobile.html preserves the raw-string HTML byte-for-byte as a standalone file extracted from lib.rs.
- server/mod.rs should re-export roadcast_event with pub(crate) visibility, matching the helper's crate-only visibility while still allowing commands/sync.rs to call crate::server::broadcast_event(...).
- lib.rs still needs the Windows resume PBT_* constants after server extraction because clipboard/monitor.rs references them through crate::; dropping them breaks cargo check even though the HTTP handlers moved out.

