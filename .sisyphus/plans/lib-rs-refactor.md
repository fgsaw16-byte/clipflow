# Plan: lib.rs Refactor — ClipFlow Backend Multi-Module System

## Overview

Pure structural refactoring of `src-tauri/src/lib.rs` (2151 lines) into a multi-module system.
**NO business logic changes.** Only move code to the right files.

## Target Structure

```
src-tauri/src/
├── main.rs                  (unchanged)
├── lib.rs                   (mod declarations + run() only, ~200 lines)
├── state.rs                 (AppState, HistoryItem, safe_lock, CREATE_NO_WINDOW)
├── db.rs                    (get_db_path, init_db, detect_category, signature_for, read_setting_sync)
├── window.rs                (position_window, position_window_at_mouse, build_tray)
├── clipboard/
│   ├── mod.rs
│   ├── operations.rs        (write_image_bytes_to_clipboard, write_to_clipboard_inner, read_and_persist_clipboard)
│   ├── monitor.rs           (run_clipboard_monitor - Windows + non-Windows)
│   └── supervisor.rs        (spawn_clipboard_supervisor)
├── commands/
│   ├── mod.rs
│   ├── history.rs           (get_history, delete_item, clear_history, set_category, update_history_content)
│   ├── paste.rs             (write_clipboard, smart_copy, trigger_paste, paste_item_inner, paste_item, set_queue, paste_queue_next, copy_image_to_clipboard)
│   ├── settings.rs          (get_all_settings, save_setting, get_file_save_path, set_save_path)
│   └── sync.rs              (force_sync, restart_clipboard_monitor, send_to_phone, translate_text, translate_google, get_local_ip, upload_file)
└── server/
    ├── mod.rs
    ├── handlers.rs          (sse_events, broadcast_event, get_configured_save_path, ensure_unique_path, receive_file, receive_image, receive_data)
    ├── mobile_ui.rs         (web_home handler using include_str!)
    └── mobile.html          (extracted HTML from web_home)
```

## TODOs

- [x] Step 1: Create `state.rs` — extract AppState, HistoryItem, safe_lock, CREATE_NO_WINDOW
- [x] Step 2: Create `db.rs` — extract get_db_path, init_db, detect_category, signature_for, read_setting_sync
- [x] Step 3: Create `window.rs` — extract position_window, position_window_at_mouse, build_tray
- [x] Step 4: Create `clipboard/mod.rs` + `clipboard/operations.rs` — extract write_image_bytes_to_clipboard, write_to_clipboard_inner, read_and_persist_clipboard
- [x] Step 5: Create `clipboard/monitor.rs` — extract run_clipboard_monitor (Windows + non-Windows)
- [x] Step 6: Create `clipboard/supervisor.rs` — extract supervisor thread logic
- [x] Step 7: Create `commands/mod.rs` + `commands/history.rs` — extract get_history, delete_item, clear_history, set_category, update_history_content
- [x] Step 8: Create `commands/paste.rs` — extract write_clipboard, smart_copy, trigger_paste, paste_item_inner, paste_item, set_queue, paste_queue_next, copy_image_to_clipboard
- [x] Step 9: Create `commands/settings.rs` — extract get_all_settings, save_setting, get_file_save_path, set_save_path
- [x] Step 10: Create `commands/sync.rs` — extract force_sync, restart_clipboard_monitor, send_to_phone, translate_text, get_local_ip, upload_file
- [x] Step 11: Create `server/mobile.html` + `server/mobile_ui.rs` + `server/mod.rs` — extract web_home handler and HTML
- [x] Step 12: Create `server/handlers.rs` — extract sse_events, broadcast_event, receive_data, receive_image, receive_file, get_configured_save_path, ensure_unique_path
- [x] Step 13: Slim down `lib.rs` — keep only mod declarations + run() + foreground radar thread
- [x] Step 14: Final verification — cargo check + cargo build + npx tsc --noEmit

## Final Verification Wave

- [x] FV1: `cargo check` passes with zero errors
- [x] FV2: `cargo build` completes successfully (debug build)
- [x] FV3: `npx tsc --noEmit` passes (frontend unchanged, no regressions)
- [x] FV4: Final file structure matches target (all 15 files present, lib.rs ~200 lines)
