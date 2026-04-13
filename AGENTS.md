# AGENTS.md — ClipFlow

ClipFlow is a Windows-first desktop clipboard manager built with **Tauri v2** (Rust backend + React/TypeScript frontend). This file provides guidance for agentic coding agents working in this repository.

---

## Project Layout

```
clipflow/
├── src/                   # Frontend — TypeScript + React 19 (modularized)
│   ├── App.tsx            # Frontend composition root (~223 lines)
│   ├── App.css            # All styles (~1377 lines, single stylesheet)
│   ├── main.tsx           # React entry point
│   ├── types/
│   │   └── index.ts       # Shared frontend types: HistoryItem, ChatMessage, ViewType
│   ├── constants/
│   │   └── index.ts       # TAB_ORDER, CATEGORIES
│   ├── lib/
│   │   └── window.ts      # Tauri appWindow singleton
│   ├── utils/
│   │   ├── format.ts      # getTruncatedText(), formatTime()
│   │   └── Icon.tsx       # Local icon wrapper component
│   ├── api/
│   │   ├── history.ts     # History CRUD invoke wrappers
│   │   ├── paste.ts       # Clipboard / queue invoke wrappers
│   │   ├── settings.ts    # Settings + file path invoke wrappers
│   │   └── sync.ts        # Force sync / phone sync / translate invoke wrappers
│   ├── hooks/
│   │   ├── useToast.ts            # Toast state + showToast()
│   │   ├── useSettings.ts         # Settings loading, theme apply, auto-start, folder picker
│   │   ├── useTabAnimation.ts     # activeTab/renderTab/listStyle tab state machine
│   │   ├── useTranslations.ts     # Translation state + selection translation actions
│   │   ├── useViewer.ts           # Viewer zoom/pan/edit/save behaviors
│   │   ├── useClipboardController.ts # History loading, copy/paste/delete/queue/forceSync
│   │   ├── usePullRefresh.ts      # Pull-to-refresh refs + DOM interaction handlers
│   │   └── useClipboardEvents.ts  # listen() subscriptions, focus sync, global mouseup
│   ├── components/
│   │   ├── Toast.tsx       # Toast overlay
│   │   ├── DeleteModal.tsx # Clear-all confirmation modal
│   │   ├── FilterBar.tsx   # Category tabs + scroll-to-top control
│   │   ├── Toolbar.tsx     # Top toolbar / search / navigation shell
│   │   ├── HistoryCard.tsx # Single history item card
│   │   ├── HistoryList.tsx # History list + pull-refresh container
│   │   └── views/
│   │       ├── HomeView.tsx     # Main history screen
│   │       ├── ViewerView.tsx   # Detail / image / text viewer
│   │       ├── ChatView.tsx     # Phone sync chat screen
│   │       └── SettingsView.tsx # Settings screen (includes theme mode select)
│   ├── assets/
│   │   └── react.svg       # Vite default asset
│   └── vite-env.d.ts       # Vite type declarations
├── src-tauri/             # Backend — Rust (multi-module)
│   ├── src/
│   │   ├── main.rs        # Entry point — calls lib::run() (3 lines)
│   │   ├── lib.rs         # mod declarations + run() bootstrap (~143 lines)
│   │   ├── state.rs       # AppState, HistoryItem, safe_lock, CREATE_NO_WINDOW
│   │   ├── db.rs          # SQLite helpers: get_db_path, init_db, detect_category, signature_for, read_setting_sync
│   │   ├── window.rs      # Window positioning + system tray: position_window, position_window_at_mouse, build_tray
│   │   ├── clipboard/
│   │   │   ├── mod.rs     # Re-exports
│   │   │   ├── operations.rs  # read/write helpers: write_to_clipboard_inner, write_image_bytes_to_clipboard, read_and_persist_clipboard
│   │   │   ├── monitor.rs     # Win32 message loop + non-Windows polling: run_clipboard_monitor
│   │   │   └── supervisor.rs  # Auto-restart loop: spawn_clipboard_supervisor
│   │   ├── commands/
│   │   │   ├── mod.rs     # pub use * re-exports all 23 Tauri commands
│   │   │   ├── history.rs # get_history, delete_item, clear_history, set_category, update_history_content
│   │   │   ├── paste.rs   # write_clipboard, smart_copy, trigger_paste, paste_item, set_queue, paste_queue_next, copy_image_to_clipboard
│   │   │   ├── settings.rs# get_all_settings, save_setting, get_file_save_path, set_save_path
│   │   │   └── sync.rs    # force_sync, restart_clipboard_monitor, send_to_phone, translate_text, get_local_ip, upload_file
│   │   └── server/
│   │       ├── mod.rs     # configure_server() + re-exports broadcast_event
│   │       ├── handlers.rs# sse_events, broadcast_event, receive_file, receive_image, receive_data, get_configured_save_path, ensure_unique_path
│   │       ├── mobile_ui.rs # web_home handler (uses include_str!("mobile.html"))
│   │       └── mobile.html  # Phone sync web UI (~255 lines)
│   ├── Cargo.toml         # Rust dependencies
│   ├── tauri.conf.json    # App configuration (window, bundle, CSP)
│   └── capabilities/      # Tauri permission grants
├── index.html             # Vite HTML entry
├── vite.config.ts         # Vite config (port 1420, watches src/ only)
├── tsconfig.json          # Strict TypeScript config
└── package.json           # npm scripts
```

---

## Build, Dev, and Check Commands

### Full App (Tauri — preferred)

```bash
npm run tauri dev     # Start full app in development mode
npm run tauri build   # Production build (frontend + Rust bundled)
```

### Frontend Only

```bash
npm run dev           # Vite dev server on http://localhost:1420
npm run build         # Type-check (tsc) then bundle (vite build)
npm run preview       # Preview the production build
```

### Rust / Backend Only

```bash
# Run from src-tauri/
cargo build                     # Debug build
cargo build --release           # Release build
cargo check                     # Fast syntax + type check (no binary)
cargo clippy                    # Lints (not configured, but available)
```

### Type-Checking Only

```bash
# Frontend (from project root)
npx tsc --noEmit

# Rust (from src-tauri/)
cargo check
```

---

## Test Commands

**There are currently no tests in this project** — no test files, no test framework (no vitest/jest on the frontend; no `#[cfg(test)]` blocks in Rust).

If tests are added in the future:

```bash
# Frontend (after adding vitest)
npx vitest run                          # Run all tests
npx vitest run src/foo.test.ts          # Run a single test file
npx vitest run -t "test name"           # Run tests matching a name pattern

# Rust (from src-tauri/)
cargo test                              # Run all tests
cargo test test_function_name           # Run a single test by name
cargo test -- --nocapture               # Show println! output during tests
```

---

## Lint Commands

No linter is configured. The closest equivalents:

```bash
# Frontend type errors (from project root)
npx tsc --noEmit

# Rust lints (from src-tauri/)
cargo clippy
```

---

## Architecture

- **Frontend**: Modular React app with a thin `App.tsx` composition root (~223 lines). Business logic is split across `hooks/`, Tauri invoke wrappers live in `api/`, shared types/constants live in `types/` + `constants/`, and UI is split between reusable `components/` and top-level screen components in `components/views/`. `App.css` remains the single global stylesheet.
- **Backend**: Multi-module Rust system across 15 files under `src-tauri/src/`. `lib.rs` (~143 lines) contains only module declarations and `run()`. Business logic is split into `state.rs` (types), `db.rs` (SQLite), `window.rs` (tray/window), `clipboard/` (monitoring + read/write), `commands/` (23 Tauri commands), and `server/` (Actix-web HTTP server for phone sync).
- **State**: `AppState` struct wrapped in `Arc<Mutex<T>>`, injected via Tauri's `.manage()` and Actix-web's `web::Data`.
- **Persistence**: SQLite (`rusqlite`) — one `history.db` file with `history` and `settings` tables.
- **Background threads**: clipboard monitor (polls every 500 ms), Win32 foreground-window radar (polls every 250 ms), Actix-web HTTP server (port 19527 for phone-to-PC sync via SSE).
- **Frontend↔Backend bridge**: `invoke()` for frontend→Rust calls; `emit()` for Rust→frontend events.

### Frontend Module Reference

Quick-lookup table for agents working on the React/TypeScript frontend. All files are under `src/`.

| File / Dir | Responsibility | Key symbols |
|---|---|---|
| `App.tsx` | Composition root that wires hooks, cross-view state, and top-level routing | `App` |
| `types/index.ts` | Shared frontend types | `HistoryItem`, `ChatMessage`, `ViewType` |
| `constants/index.ts` | Shared UI constants | `TAB_ORDER`, `CATEGORIES` |
| `lib/window.ts` | Shared Tauri window singleton | `appWindow` |
| `utils/format.ts` | Display formatting helpers | `getTruncatedText()`, `formatTime()` |
| `utils/Icon.tsx` | Local icon mapping wrapper | `Icon` |
| `api/history.ts` | History-related invoke wrappers | `getHistory`, `deleteItem`, `clearHistory`, `setCategory`, `updateHistoryContent` |
| `api/paste.ts` | Clipboard / paste / queue invoke wrappers | `smartCopy`, `pasteItem`, `setQueue`, `pasteQueueNext` |
| `api/settings.ts` | Settings invoke wrappers | `getAllSettings`, `saveSetting`, `getFileSavePath`, `setSavePath` |
| `api/sync.ts` | Sync / translate invoke wrappers | `forceSync`, `sendToPhone`, `translateText`, `getLocalIp` |
| `hooks/useToast.ts` | Toast message state | `useToast` |
| `hooks/useSettings.ts` | Settings bootstrapping, theme application, auto-start, folder picking | `useSettings`, `applyTheme` |
| `hooks/useTabAnimation.ts` | Tab animation state machine | `useTabAnimation` |
| `hooks/useTranslations.ts` | Translation state and translation actions | `useTranslations` |
| `hooks/useViewer.ts` | Viewer interaction state (zoom/pan/edit/save) | `useViewer` |
| `hooks/useClipboardController.ts` | Main history/clipboard orchestration hook | `useClipboardController` |
| `hooks/usePullRefresh.ts` | Pull-to-refresh refs and mouse handlers | `usePullRefresh` |
| `hooks/useClipboardEvents.ts` | Tauri event listeners and global side effects | `useClipboardEvents` |
| `components/Toolbar.tsx` | Top toolbar shared across views | `Toolbar` |
| `components/HistoryList.tsx` | History list rendering and scroll registration | `HistoryList` |
| `components/HistoryCard.tsx` | Individual history card interactions | `HistoryCard` |
| `components/views/HomeView.tsx` | Main history page composition | `HomeView` |
| `components/views/ViewerView.tsx` | Viewer page composition | `ViewerView` |
| `components/views/ChatView.tsx` | Phone sync / chat page composition | `ChatView` |
| `components/views/SettingsView.tsx` | Settings page composition | `SettingsView` |

- To change toolbar/search/navigation behavior: edit `components/Toolbar.tsx` and its props in `App.tsx`
- To change list rendering or card interactions: start with `components/HistoryList.tsx` and `components/HistoryCard.tsx`
- To change clipboard/history actions: start with `hooks/useClipboardController.ts`, then check `api/`
- To change settings/theme behavior: start with `hooks/useSettings.ts`, then `components/views/SettingsView.tsx`
- To change viewer zoom/edit/translate behavior: read `hooks/useViewer.ts` and `hooks/useTranslations.ts` together

---

## Backend Module Reference

Quick-lookup table for agents working on the Rust backend. All files are under `src-tauri/src/`.

| File | Responsibility | Key symbols |
|------|---------------|-------------|
| `lib.rs` | Bootstrap: mod declarations, `run()`, plugin registration, foreground radar thread | `run()` |
| `state.rs` | Core types and shared state primitives | `AppState`, `HistoryItem`, `safe_lock()`, `CREATE_NO_WINDOW` |
| `db.rs` | All SQLite operations and content utilities | `get_db_path()`, `init_db()`, `detect_category()`, `signature_for()`, `read_setting_sync()` |
| `window.rs` | Window positioning and system tray construction | `position_window()`, `position_window_at_mouse()`, `build_tray()` |
| `clipboard/operations.rs` | Clipboard read/write and persistence to DB | `write_to_clipboard_inner()`, `write_image_bytes_to_clipboard()`, `read_and_persist_clipboard()` |
| `clipboard/monitor.rs` | Win32 message loop (Windows) + polling fallback (non-Windows) | `run_clipboard_monitor()` |
| `clipboard/supervisor.rs` | Crash-detection loop that restarts the monitor | `spawn_clipboard_supervisor()` |
| `commands/history.rs` | Tauri commands for history CRUD | `get_history`, `delete_item`, `clear_history`, `set_category`, `update_history_content` |
| `commands/paste.rs` | Tauri commands for paste operations; Win32 `AttachThreadInput` FFI lives here | `write_clipboard`, `smart_copy`, `trigger_paste`, `paste_item`, `set_queue`, `paste_queue_next`, `copy_image_to_clipboard` |
| `commands/settings.rs` | Tauri commands for app settings and file paths | `get_all_settings`, `save_setting`, `get_file_save_path`, `set_save_path` |
| `commands/sync.rs` | Tauri commands for sync, clipboard recovery, translation | `force_sync`, `restart_clipboard_monitor`, `send_to_phone`, `translate_text`, `get_local_ip`, `upload_file` |
| `server/handlers.rs` | Actix-web handlers for phone sync; SSE event stream and file/image/data receive | `sse_events`, `broadcast_event`, `receive_file`, `receive_image`, `receive_data` |
| `server/mobile_ui.rs` | Actix-web handler serving the phone web UI | `web_home` (uses `include_str!("mobile.html")`) |
| `server/mobile.html` | Self-contained HTML for the phone sync web interface | (static HTML, ~255 lines) |

### Module Dependency Order (bottom-up)

```
state  ←  db  ←  clipboard/*  ←  commands/*
                              ←  server/*
window  (standalone, only depends on tauri + windows crate)
lib.rs  (depends on everything, orchestrates run())
```

- To find a Tauri command: search by command name in `commands/`
- To modify clipboard capture logic: edit `clipboard/operations.rs` or `clipboard/monitor.rs`
- To change the phone UI: edit `server/mobile.html`
- To add a new Tauri command: add to the appropriate `commands/*.rs` file, then add to `generate_handler![]` in `lib.rs`

---

## TypeScript / Frontend Code Style

### Imports Order

Follow this order — do not reorder arbitrarily:

1. React hooks (`import { useState, useEffect } from "react"`)
2. Tauri APIs (`@tauri-apps/api/core`, `@tauri-apps/api/event`, `@tauri-apps/plugin-*`)
3. Third-party libraries (`qrcode.react`, `framer-motion`, `lucide-react`)
4. Local CSS (`./App.css`)

Group large icon sets into one multi-line import block.

### Naming Conventions

| Element | Convention | Example |
|---|---|---|
| React component files | `PascalCase` | `App.tsx` |
| Entry/utility files | `camelCase` | `main.tsx` |
| React components | `PascalCase` | `App`, `Icon` |
| Functions / handlers | `camelCase` | `loadHistory`, `handleCopy` |
| State variables | `camelCase` | `searchText`, `isQueueMode` |
| Refs | `camelCase` + `Ref` suffix | `listRef`, `isPastingRef` |
| Module-level constants | `SCREAMING_SNAKE_CASE` | `TAB_ORDER`, `CATEGORIES` |
| Interfaces | `PascalCase` | `HistoryItem`, `ChatMessage` |
| Type aliases | `PascalCase` | `ViewType` |
| CSS classes | `kebab-case` | `card-header`, `filter-chip` |
| CSS custom properties | `--kebab-case` | `--bg-card`, `--accent-color` |

### TypeScript Usage

- `strict: true` is enforced — no unused locals/parameters, no implicit `any` in new code.
- Use `interface` for data shapes, `type` for union/alias types.
- Use generic `invoke<T>()` — always specify the return type: `invoke<HistoryItem[]>("get_history")`.
- Avoid `any` in new code; the existing `settings` state uses `any` for legacy reasons.
- `noEmit: true` — TypeScript only type-checks; Vite handles transpilation.
- `moduleResolution: "bundler"` — use ESM-style imports (no `.js` extension required).
- JSX transform is `react-jsx` — no need to `import React` in JSX files.

### Formatting

No auto-formatter is configured. Follow these conventions when writing new code:

- Semicolons: always present.
- Quotes: double quotes for string literals (`"value"`).
- Trailing commas: use them in multi-line objects and arrays.
- Line length: prefer lines under 120 characters; avoid ultra-long one-liners.
- Indentation: 2 spaces (consistent with the existing codebase).

### Error Handling (Frontend)

- Wrap `await invoke(...)` calls in `try/catch`.
- Log errors with `console.error(e)` at minimum.
- Tauri commands return `Result<T, String>` on the Rust side — errors surface as rejected promises.
- Most errors are silently logged; only show UI feedback for user-visible failures.

### React Patterns

- Use `useRef` for mutable interaction guards (e.g., `isPastingRef`, `isInteractingRef`) to avoid stale closures in async callbacks.
- Use `useLayoutEffect` when scroll position must be restored synchronously.
- Event handler props: prefix with `handle` (`handleCopy`, `handleDelete`, `handleTabChange`).

---

## Rust / Backend Code Style

### Naming Conventions

| Element | Convention | Example |
|---|---|---|
| Functions | `snake_case` | `get_db_path`, `init_db` |
| Structs | `PascalCase` | `AppState`, `HistoryItem` |
| Constants | `SCREAMING_SNAKE_CASE` | `CREATE_NO_WINDOW` |
| Fields | `snake_case` | `db_path`, `last_clipboard` |
| Tauri command fns | `snake_case` decorated with `#[tauri::command]` | `get_history` |

### Rust Patterns

- Wrap all shared mutable state in `Arc<Mutex<T>>` — never use `static mut`.
- Use `#[cfg(target_os = "windows")]` for all Win32-specific code blocks.
- Propagate errors with the `?` operator where possible; use `.map_err(|e| e.to_string())` for Tauri commands (which must return `Result<T, String>`).
- Spawn background work with `thread::spawn`; use `tokio::sync::broadcast` for cross-thread events.
- Use `println!` for debug output (no structured logging crate is in use).

### Error Handling (Rust)

All `#[tauri::command]` functions must return `Result<T, String>`:

```rust
#[tauri::command]
fn example(state: tauri::State<AppState>) -> Result<Vec<HistoryItem>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    // ...
    Ok(items)
}
```

---

## Key Dependencies

| Crate / Package | Purpose |
|---|---|
| `tauri` v2 | App shell, tray, window, global hotkeys |
| `rusqlite` (bundled) | SQLite for history + settings |
| `actix-web` 4 | Built-in HTTP server for phone sync (port 19527) |
| `arboard` | Cross-platform clipboard read/write |
| `windows` 0.61 | Win32 APIs (foreground window, cursor pos) |
| `window-vibrancy` | Windows Acrylic blur effect |
| `enigo` | Keyboard simulation (Ctrl+V paste) |
| `reqwest` | HTTP client (Google Translate) |
| `react` 19 | UI framework |
| `framer-motion` | Animations |
| `lucide-react` | Icon set |
| `qrcode.react` | QR code for phone pairing |

---

## Important Notes for Agents

- **Windows-only**: Win32 APIs, Acrylic blur, and global hotkeys are guarded by `#[cfg(target_os = "windows")]`. Do not remove these guards.
- **No linter/formatter is configured**: Run `npx tsc --noEmit` and `cargo check` to validate changes before finishing.
- **CSP is disabled** (`"csp": null` in `tauri.conf.json`) — do not enable it without testing the phone sync UI.
- **Vite dev server requires port 1420** (`strictPort: true`) — ensure the port is free before running `npm run dev`.
- **The mobile UI HTML** lives in `src-tauri/src/server/mobile.html` — edit that file directly. It is compiled into the binary via `include_str!()` in `server/mobile_ui.rs`.
- **No tests exist** — manually verify behavior after changes, especially around clipboard monitoring and the Actix-web server.

---

## Change Log

详细修改日志已迁移至 [CHANGELOG.md](./CHANGELOG.md)。
