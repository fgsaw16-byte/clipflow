# AGENTS.md — ClipFlow

ClipFlow is a Windows-first desktop clipboard manager built with **Tauri v2** (Rust backend + React/TypeScript frontend). This file provides guidance for agentic coding agents working in this repository.

---

## Project Layout

```
clipflow/
├── src/                   # Frontend — TypeScript + React 19
│   ├── App.tsx            # All UI logic (single component, ~824 lines)
│   ├── App.css            # All styles (~1361 lines, single stylesheet)
│   └── main.tsx           # React entry point
├── src-tauri/             # Backend — Rust
│   ├── src/lib.rs         # All business logic (~1613 lines)
│   ├── src/main.rs        # Entry point — calls lib::run()
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

- **Frontend**: Single `App.tsx` component (~824 lines) with all views (home, chat, settings, viewer) and state. No sub-components.
- **Backend**: Single `lib.rs` (~1613 lines) with all Tauri commands, background threads, and the built-in Actix-web HTTP server.
- **State**: `AppState` struct wrapped in `Arc<Mutex<T>>`, injected via Tauri's `.manage()` and Actix-web's `web::Data`.
- **Persistence**: SQLite (`rusqlite`) — one `history.db` file with `history` and `settings` tables.
- **Background threads**: clipboard monitor (polls every 500 ms), Win32 foreground-window radar (polls every 250 ms), Actix-web HTTP server (port 19527 for phone-to-PC sync via SSE).
- **Frontend↔Backend bridge**: `invoke()` for frontend→Rust calls; `emit()` for Rust→frontend events.

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
- **The mobile UI is an embedded HTML string** inside `lib.rs` — editing it requires finding the raw string literal and maintaining its structure.
- **No tests exist** — manually verify behavior after changes, especially around clipboard monitoring and the Actix-web server.

---

## Change Log

### 2026-03-23: Deadlock / 死锁深度修复

**改动原因**

将剪贴板监听从定时轮询重构为 Win32 `AddClipboardFormatListener` 后，电脑闲置约 20 分钟（经历息屏/休眠）后出现致命症状：
1. 后台剪贴板监听彻底失效，不再捕获任何新内容。
2. 前端 `invoke()` 调用（包括下拉刷新 `get_history`）完全卡死，拿不到返回值。

**根因诊断（三杀组合）**

| # | 杀手 | 机制 |
|---|---|---|
| 1 | `GetMessageW` 僵死 | 休眠唤醒后，`HWND_MESSAGE` 消息窗口的消息链被系统掐断。`GetMessageW` 永远阻塞，但线程没有 panic，`monitor_alive` 仍为 `true`，supervisor 不会重启。 |
| 2 | `skip_monitor` 泄漏 | `receive_image` 设置 `skip_monitor = true` 后 spawn 子线程写剪贴板，如果该子线程 panic（休眠后剪贴板句柄失效），`release_monitor()` 不执行，`skip_monitor` 永远卡在 `true`。 |
| 3 | Mutex 毒化静默失败 | `restart_clipboard_monitor` 使用 `.lock().map().unwrap_or(0)`，锁被毒化时拿到 0，什么都不做。前端调用该命令无法真正重启监听。 |

**改动内容**

#### Rust 后端 (`src-tauri/src/lib.rs`)

1. **新增 `safe_lock()` 辅助函数**
   - 使用 `.unwrap_or_else(|poisoned| poisoned.into_inner())` 从毒化锁中恢复数据。
   - 所有关键路径的 `.lock()` 调用（`restart_clipboard_monitor`、`read_and_persist_clipboard`、supervisor 清理、`receive_image` 子线程）已替换为 `safe_lock()`。

2. **`receive_image` 子线程加 `catch_unwind` 保护**
   - 整个剪贴板写入逻辑用 `panic::catch_unwind` 包裹。
   - 无论正常退出、返回错误、还是 panic，都保证执行 `release_monitor()` 清除 `skip_monitor` 和 `last_clipboard_write_ms`。

3. **`restart_clipboard_monitor` 命令增强**
   - 调用时主动清除 `skip_monitor`、`last_clipboard_write_ms`、`is_internal_pasting` 三个抑制标志。
   - 若 HWND 为 0（监听器已死），主动将 `monitor_alive` 设为 `false`，让 supervisor 感知并重启。

4. **`run_clipboard_monitor` 消息循环重写**
   - 新增 `WM_POWERBROADCAST` 处理：检测 `PBT_APMRESUMEAUTOMATIC` / `PBT_APMRESUMESUSPEND`，自动执行 `RemoveClipboardFormatListener` → `AddClipboardFormatListener` 重注册，重建 arboard 实例，重置 sequence tracking，清除所有卡死标志，并立即读取一次剪贴板。
   - 新增 30 秒心跳定时器（`SetTimer` / `WM_TIMER`）：每 30 秒通过 `GetClipboardSequenceNumber` 对比检测是否有漏掉的剪贴板变更（对抗 `GetMessageW` 僵死）；同时清除超过 5 秒的 stuck `skip_monitor`。
   - `GetMessageW` 不再过滤 hwnd（传 `None`），以便接收 `WM_POWERBROADCAST` 广播消息。
   - 清理阶段增加 `KillTimer`。

5. **新增 `force_sync` Tauri command**
   - 一键清除所有抑制标志（`skip_monitor`、`last_clipboard_write_ms`、`is_internal_pasting`、`ignore_signature`）。
   - 独立于监听线程，直接用新的 arboard 实例读取当前剪贴板内容并持久化。
   - 向监听窗口发 `WM_APP` 触发重启（或设 `monitor_alive=false` 让 supervisor 重生）。
   - 返回最新历史记录，前端可直接用返回值更新 UI。
   - 已注册到 `invoke_handler` 的 `generate_handler![]` 中。

6. **supervisor 清理代码**
   - 所有 `if let Ok(mut x) = lock()` 替换为 `safe_lock()`。
   - 增加 `is_internal_pasting` 的清理。

#### 前端 (`src/App.tsx`)

1. **新增 `forceSync()` 函数** — 调用后端 `force_sync`，失败时 fallback 到 `get_history`。
2. **监听 `listener-crashed` 事件** — 后端监听器崩溃时自动调用 `forceSync()` 恢复。
3. **窗口获焦自动同步** — `appWindow.onFocusChanged` 中，获得焦点时调用 `forceSync()`（覆盖休眠唤醒后用户首次呼出窗口的场景）。
4. **Filter 栏新增刷新按钮** — 在 "回到顶部" 按钮旁边新增 `<Icon name="refresh">` 按钮，点击调用 `forceSync()`，标题为 "强制同步 (修复卡死)"。

**未修改的部分**

- 所有 framer-motion 动画（卡片 spring 入场/退场、tab 滑块、删除弹窗、hover/tap 缩放）完全未触碰。
- UI 结构（toolbar、filter-bar、content-area、viewer、chat、settings 布局）未改变。
- Actix-web 服务器、SQLite 数据库操作、快捷键系统均未修改。

**验证结果**

- `cargo check` — 通过（仅 1 个预存的 dead_code warning）。
- `npx tsc --noEmit` — 通过，无错误。

**潜在问题与后续观察点**

| 风险 | 说明 | 后续对策 |
|---|---|---|
| `WM_POWERBROADCAST` 可能不送达 `HWND_MESSAGE` 窗口 | 微软文档对 message-only window 能否收到广播消息描述模糊。如果收不到，心跳定时器是最后防线。 | 测试休眠唤醒场景，观察日志中是否有 `⚡ System resumed` 输出。若没有，考虑改用普通隐藏窗口替代 `HWND_MESSAGE`。 |
| `forceSync` 在窗口获焦时频繁调用 | 每次 Alt+V 呼出窗口都会调用一次 `force_sync`，产生一次 SQLite 查询 + 可能的 `PostMessageW`。 | 目前性能影响可忽略（<5ms）。若发现卡顿，加一个 2 秒节流。 |
| 心跳定时器 30 秒间隔 | 极端情况下，休眠唤醒后最多需要 30 秒才能自动恢复。 | 用户可通过刷新按钮或 Alt+V 立即触发 `forceSync`。如果 30 秒太长，可改为 10 秒。 |
| 非 Windows 平台未测试 | 所有 Win32 改动都在 `#[cfg(target_os = "windows")]` 内，非 Windows 的 polling fallback 未修改。 | 目前 ClipFlow 是 Windows-only 产品，暂不影响。 |

**如果问题仍未解决**

下次排查时直接阅读本段即可衔接。重点关注：
1. 运行 `npm run tauri dev`，闲置 20 分钟后，检查终端日志中是否有 `⚡ System resumed` 或 `⚠️ Heartbeat detected missed clipboard change`。
2. 如果日志中都没有出现，说明 `WM_POWERBROADCAST` 和 `WM_TIMER` 都没送达 → 需要将 `HWND_MESSAGE` 改为普通隐藏窗口（`WS_EX_TOOLWINDOW` + `ShowWindow(SW_HIDE)`）。
3. 如果心跳日志正常但剪贴板仍不工作，检查 arboard `Clipboard::new()` 在休眠后是否始终失败 → 可能需要延迟重试或切换到纯 Win32 `OpenClipboard` API。
4. 如果前端 `invoke` 仍然卡死，说明问题不在 Rust 锁，而在 Tauri IPC 层 → 检查 WebView2 进程是否被系统挂起。
