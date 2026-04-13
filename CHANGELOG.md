# ClipFlow 修改日志

---

## 2026-04-12: 前端模块化重构 + 主题切换入口补齐

**类型**: 重构 + 功能补齐

**概述**: 将前端 `src/App.tsx` 从单体组件重构为模块化结构，拆分为类型、常量、工具、API、Hooks 与视图组件；在验证阶段补齐设置页的主题模式下拉框，使方案中的 14 项功能清单全部通过。

**改动原因**

原始前端长期将视图、状态、事件、Tauri 调用与交互逻辑堆叠在单一 `App.tsx` 中，导致：
1. 修改某一类行为（如设置、翻译、剪贴板、Viewer）时需要在同一大文件中频繁跳转。
2. 逻辑耦合严重，复用与验证成本高，容易在小改动中引入回归。
3. 后续 AI / 人工协作难以快速定位职责边界。

在最终功能验收时，又发现“深色/浅色模式切换”在后端设置逻辑中已经完整存在，但设置页没有暴露 UI 入口，需要补齐以满足已批准方案中的功能清单。

**改动内容**

#### 前端模块拆分（`src/`）

1. **共享基础层**
   - `types/index.ts`：抽出 `HistoryItem`、`ChatMessage`、`ViewType`
   - `constants/index.ts`：抽出 `TAB_ORDER`、`CATEGORIES`
   - `lib/window.ts`：集中导出 `appWindow` 单例
   - `utils/format.ts`：抽出 `getTruncatedText()`、`formatTime()`
   - `utils/Icon.tsx`：抽出本地图标包装组件

2. **Tauri API 包装层**
   - `api/history.ts`：历史记录 CRUD 与分类更新
   - `api/paste.ts`：复制、粘贴、队列相关调用
   - `api/settings.ts`：设置与保存路径调用
   - `api/sync.ts`：强制同步、发送到手机、翻译、IP 获取

3. **状态与行为 Hooks**
   - `useToast.ts`：Toast 消息管理
   - `useSettings.ts`：设置加载、`applyTheme()`、开机自启、目录选择
   - `useTabAnimation.ts`：Tab 切换动画状态机
   - `useTranslations.ts`：翻译状态、全文翻译、选区翻译
   - `useViewer.ts`：Viewer 缩放、拖拽、编辑保存
   - `useClipboardController.ts`：历史记录加载、复制、删除、清空、队列模式、强制同步
   - `usePullRefresh.ts`：下拉刷新 refs 与 DOM 交互
   - `useClipboardEvents.ts`：Tauri 事件监听、窗口 focus 同步、全局 mouseup

4. **UI 组件层**
   - 通用组件：`Toast.tsx`、`DeleteModal.tsx`、`FilterBar.tsx`、`Toolbar.tsx`、`HistoryCard.tsx`、`HistoryList.tsx`
   - 视图组件：`views/HomeView.tsx`、`views/ViewerView.tsx`、`views/ChatView.tsx`、`views/SettingsView.tsx`

5. **`App.tsx` 重写为组合根组件**
   - 从约 900 行级单体收缩到约 223 行
   - 只保留跨视图状态、Hook 编排、顶层路由与少量胶水逻辑
   - 保留原有 DOM 层级、className、framer-motion 行为与 `App.css` 单文件样式模式

#### 主题切换入口补齐

1. **在 `src/components/views/SettingsView.tsx` 新增“主题模式”下拉框**
   - 放置在“常规”分组中
   - 选项：`跟随系统` / `浅色` / `深色`
   - 绑定 `settings.theme || 'system'`
   - 通过 `updateSetting('theme', value)` 写入设置

2. **复用既有主题逻辑，无额外扩散修改**
   - `useSettings.ts` 中既有 `applyTheme()` 已支持 `system` / `light` / `dark`
   - 未新增依赖
   - 未修改 `App.css`

**关键架构细节**

- `App.tsx` 负责组合 8 个 Hooks，并将视图切换、共享 viewer state、聊天状态和少量跨模块胶水逻辑保留在根组件。
- `viewerContent` / `setViewerContent` / `textareaRef` 被提升到 `App.tsx`，用于打破 `useViewer` 与 `useTranslations` 间的循环依赖。
- `HistoryList` 通过回调注册 `scrollToTop`，由 `HomeView` / `FilterBar` 上层触发，避免把滚动控制硬编码进根组件。
- 分类切换与编辑保存使用 `setHistory(...)` 乐观更新，再调用 `setCategory(...)` 落库，保持交互即时性。
- 主题模式 UI 只是补齐缺失入口，不是新引入的主题系统；真实主题切换逻辑一直存在于 `useSettings.ts`。

**未修改的部分**

- `src/App.css`（保持零增删改）
- 所有已有 framer-motion hover / tap 数值与 DOM 结构约束
- Rust 后端、Tauri 配置、数据库 schema
- 任何第三方依赖版本

**验证结果**

- `npx tsc --noEmit` — 通过
- `npm run build` — 通过
- 像素级审计 7 项 — 全部通过
- 方案功能清单 14/14 — 全部通过（补齐主题切换入口后完成）

**快速定位**

- 前端组合根：`src/App.tsx`
- 设置与主题逻辑：`src/hooks/useSettings.ts`
- 主题切换 UI：`src/components/views/SettingsView.tsx`
- 剪贴板主控：`src/hooks/useClipboardController.ts`
- Viewer 行为：`src/hooks/useViewer.ts`

---

## 2026-04-10: 后端多模块重构

**类型**: 重构

**概述**: 将 `src-tauri/src/lib.rs`（2151 行）拆分为 15 个文件的多模块系统，消除"上帝类反模式"。纯结构重构，未修改任何业务逻辑。

**改动原因**

后端所有逻辑耦合在单一 `lib.rs` 中，职责混杂导致：
1. 添加新功能时难以定位修改点。
2. 排查 Bug 时需要在 2000+ 行中来回跳转。
3. 不同领域的代码（剪贴板监听、HTTP 服务器、粘贴逻辑、数据库操作）紧密交织，无法独立理解。

**改动内容**

将 `lib.rs` 拆分为以下模块结构：

| 模块 | 文件 | 职责 |
|------|------|------|
| 核心类型 | `state.rs` | `AppState`、`HistoryItem`、`safe_lock()`、`CREATE_NO_WINDOW` |
| 数据库 | `db.rs` | `get_db_path`、`init_db`、`detect_category`、`signature_for`、`read_setting_sync` |
| 窗口管理 | `window.rs` | `position_window`、`position_window_at_mouse`、`build_tray` |
| 剪贴板 | `clipboard/operations.rs` | `write_to_clipboard_inner`、`write_image_bytes_to_clipboard`、`read_and_persist_clipboard` |
| 剪贴板 | `clipboard/monitor.rs` | `run_clipboard_monitor`（Win32 消息循环 + 非 Windows 轮询） |
| 剪贴板 | `clipboard/supervisor.rs` | `spawn_clipboard_supervisor`（崩溃检测自动重启） |
| 命令 | `commands/history.rs` | `get_history`、`delete_item`、`clear_history`、`set_category`、`update_history_content` |
| 命令 | `commands/paste.rs` | `write_clipboard`、`smart_copy`、`trigger_paste`、`paste_item`、`set_queue`、`paste_queue_next`、`copy_image_to_clipboard` |
| 命令 | `commands/settings.rs` | `get_all_settings`、`save_setting`、`get_file_save_path`、`set_save_path` |
| 命令 | `commands/sync.rs` | `force_sync`、`restart_clipboard_monitor`、`send_to_phone`、`translate_text`、`get_local_ip`、`upload_file` |
| 服务器 | `server/handlers.rs` | `sse_events`、`broadcast_event`、`receive_file`、`receive_image`、`receive_data` |
| 服务器 | `server/mobile_ui.rs` | `web_home`（手机端页面，使用 `include_str!("mobile.html")`） |
| 服务器 | `server/mobile.html` | 手机同步 Web UI（~255 行，从 `lib.rs` 原始字符串中提取） |
| 引导 | `lib.rs`（~143 行） | `mod` 声明 + `run()` 引导函数 + foreground radar 线程 |

模块依赖关系（自底向上）：

```
state  ←  db  ←  clipboard/*  ←  commands/*
                              ←  server/*
window（独立，仅依赖 tauri + windows crate）
lib.rs（依赖所有模块，编排 run()）
```

**关键架构细节**

- `commands/mod.rs` 通过 `pub use *` 重导出全部 23 个 Tauri 命令，`lib.rs` 中 `use commands::*` 后在 `generate_handler![]` 宏中引用。
- `broadcast_event` 定义在 `server/handlers.rs`（`pub(crate)`），通过 `server/mod.rs` 重导出，`commands/sync.rs` 以 `crate::server::broadcast_event(...)` 调用。
- `paste_item_inner` 接收 `state: AppState`（非 `tauri::State<AppState>`），`paste_item` 通过 `state.inner().clone()` 调用。
- Win32 FFI `AttachThreadInput` 的 `extern "system"` 声明位于 `commands/paste.rs`。
- `PBT_APMRESUMEAUTOMATIC` / `PBT_APMRESUMESUSPEND` 常量保留在 `lib.rs`，`clipboard/monitor.rs` 通过 `crate::PBT_APM...` 引用。
- 每个模块各自保留 `#[cfg(target_os = "windows")]` 守卫，未做合并。

**未修改的部分**

- `main.rs`（3 行，仅调用 `lib::run()`）
- 前端代码（`App.tsx`、`App.css`、`main.tsx`）
- 所有业务逻辑——纯结构重构，不改变行为
- `Cargo.toml`（无新依赖）
- `tauri.conf.json`、`capabilities/`

**验证结果**

- `cargo check` — 通过
- `cargo build` — 通过（48s）
- `npx tsc --noEmit` — 通过

---

## 2026-03-24: 下拉刷新 / 删除优化 / 幽灵复活修复

**类型**: 功能改进 + Bug 修复

**概述**: 用下拉刷新替代 Filter 栏刷新按钮；修复 `force_sync` 幽灵复活问题；实现乐观删除消除闪烁。

**改动原因**

03-23 加入的 Filter 栏刷新按钮挤压了分类 Tab 标签的文字空间，导致标签文本被截断。同时发现以下问题：
1. `force_sync` 中 `read_and_persist_clipboard` 使用 `last_seq=0` 导致已删除的条目从剪贴板重新插入数据库（"幽灵复活"）。
2. 监听线程收到 `WM_APP` 主动退出后，supervisor 误报 `CRITICAL: Clipboard monitor crashed`。
3. 列表删除时存在短暂闪烁（条目先消失再回来再消失）。

**改动内容**

#### Rust 后端

1. **新增 `recently_deleted_sigs` 已删除签名黑名单**
   - `AppState` 新增字段 `recently_deleted_sigs: Arc<Mutex<HashSet<String>>>`。
   - `delete_item` 命令：删除前计算 `signature_for(&content)` 存入黑名单，防止 `force_sync` 将同一内容从剪贴板重新写入。
   - `clear_history` 命令：清空黑名单。

2. **`force_sync` 增加幽灵复活双重防护**
   - **防护 A**：读取剪贴板后检查 `recently_deleted_sigs` 黑名单，签名匹配则跳过。
   - **防护 B**：黑名单未命中时，查询数据库 `SELECT COUNT(*) > 0 FROM history WHERE content = ?` 检查是否已存在。
   - 仅当两道检查都通过时才执行 `INSERT`。

3. **`delete_item` / `clear_history` 签名变更**
   - 两个命令新增 `state: tauri::State<'_, AppState>` 参数以访问黑名单。

4. **`run_clipboard_monitor` 增加 `voluntary_exit` 标志**
   - 消息循环收到 `WM_APP` 时设置 `voluntary_exit = true`。
   - 清理阶段仅在 `!voluntary_exit` 时设置 `monitor_alive = false`，避免 supervisor 误报崩溃。

#### 前端 (`src/App.tsx`)

1. **移除 Filter 栏刷新按钮** — 解决标签文字截断问题。

2. **新增鼠标下拉刷新（Pull-to-Refresh）**
   - 在 `.list-wrapper` 上监听 `onMouseDown` / `onMouseMove` / `onMouseUp`。
   - 使用 `useRef` + 直接 DOM 操作，避免 mousemove 触发 React 重渲染。
   - 关键参数：触发阈值 60px，阻尼系数 0.4。
   - 新增 refs：`pullDistanceRef`、`pullStartYRef`、`isPullingRef`、`pullIndicatorRef`、`pullIconRef`、`pullTextRef`。

3. **新增 `pendingDeleteIds` 乐观删除黑名单**
   - `handleDelete`：先将 ID 加入 `pendingDeleteIds`（立即从 UI 隐藏），再 `await invoke("delete_item")`，完成后从 history state 中移除。
   - `filteredHistory` 计算时过滤 `pendingDeleteIds`，确保被删条目瞬间消失，无闪烁。

4. **移除列表项 framer-motion 动画**
   - 列表项从 `<motion.div>` 降级为普通 `<div>`，移除 `<AnimatePresence>` 包裹。
   - 删除按钮保留 `<motion.button>` 的 hover/tap 缩放效果。

#### CSS (`src/App.css`)

- 新增 `.pull-refresh-indicator` 样式和 `@keyframes pullSpin` 旋转动画。

**未修改的部分**

- Actix-web 服务器、SQLite schema、快捷键系统、手机同步功能。
- Win32 消息循环核心逻辑。
- 所有 `#[cfg(target_os = "windows")]` 守卫。

**验证结果**

- `cargo check` — 通过（仅 1 个预存的 `write_image_bytes_to_clipboard` dead_code warning）
- `npx tsc --noEmit` — 通过

**潜在问题**

| 风险 | 说明 | 后续对策 |
|------|------|---------|
| `recently_deleted_sigs` 无界增长 | `HashSet` 只增不减（除 `clear_history` 外） | 后续可加 LRU 缓存或定期清理 |
| 下拉刷新仅支持鼠标 | 依赖 `onMouseDown/Move/Up`，无触屏支持 | ClipFlow 是桌面应用，暂无影响 |
| 列表删除无动画 | 为极致响应速度移除了所有列表动画 | 如需动画可重新引入 AnimatePresence（建议 ≤150ms） |

**快速定位**

- 下拉刷新：搜索 `pullDistanceRef`
- 乐观删除：搜索 `pendingDeleteIds`
- 签名黑名单：搜索 `recently_deleted_sigs`
- 幽灵复活防护：搜索 `ghost resurrection`
- `voluntary_exit`：搜索 `voluntary_exit`

---

## 2026-03-23: 死锁深度修复

**类型**: Bug 修复

**概述**: 修复休眠唤醒后剪贴板监听失效 + 前端 `invoke()` 卡死的致命问题。新增 `safe_lock()`、`force_sync` 命令、电源事件处理和心跳定时器。

**改动原因**

将剪贴板监听从定时轮询重构为 Win32 `AddClipboardFormatListener` 后，电脑闲置约 20 分钟（经历息屏/休眠）后出现致命症状：
1. 后台剪贴板监听彻底失效，不再捕获任何新内容。
2. 前端 `invoke()` 调用完全卡死，拿不到返回值。

**根因诊断（三杀组合）**

| # | 杀手 | 机制 |
|---|------|------|
| 1 | `GetMessageW` 僵死 | 休眠唤醒后，`HWND_MESSAGE` 消息窗口的消息链被系统掐断。`GetMessageW` 永远阻塞，但线程没有 panic，`monitor_alive` 仍为 `true`，supervisor 不会重启。 |
| 2 | `skip_monitor` 泄漏 | `receive_image` 子线程 panic 后 `release_monitor()` 不执行，`skip_monitor` 永远卡在 `true`。 |
| 3 | Mutex 毒化静默失败 | `restart_clipboard_monitor` 使用 `.lock().map().unwrap_or(0)`，锁被毒化时拿到 0，什么都不做。 |

**改动内容**

#### Rust 后端

1. **新增 `safe_lock()` 辅助函数**
   - 使用 `.unwrap_or_else(|poisoned| poisoned.into_inner())` 从毒化锁中恢复数据。
   - 所有关键路径的 `.lock()` 调用已替换为 `safe_lock()`。

2. **`receive_image` 子线程加 `catch_unwind` 保护**
   - 无论正常退出、返回错误、还是 panic，都保证执行 `release_monitor()`。

3. **`restart_clipboard_monitor` 命令增强**
   - 调用时主动清除 `skip_monitor`、`last_clipboard_write_ms`、`is_internal_pasting` 三个抑制标志。
   - 若 HWND 为 0，主动将 `monitor_alive` 设为 `false`，让 supervisor 重启。

4. **`run_clipboard_monitor` 消息循环重写**
   - 新增 `WM_POWERBROADCAST` 处理：检测休眠唤醒事件，自动重注册剪贴板监听器、重建 arboard 实例、清除所有卡死标志。
   - 新增 30 秒心跳定时器：通过 `GetClipboardSequenceNumber` 检测漏掉的剪贴板变更；清除超过 5 秒的 stuck `skip_monitor`。

5. **新增 `force_sync` Tauri command**
   - 一键清除所有抑制标志，独立读取剪贴板并持久化，向监听窗口发 `WM_APP` 触发重启。
   - 返回最新历史记录，前端可直接更新 UI。

6. **supervisor 清理代码** — 所有锁操作替换为 `safe_lock()`。

#### 前端 (`src/App.tsx`)

1. **新增 `forceSync()` 函数** — 调用 `force_sync`，失败时 fallback 到 `get_history`。
2. **监听 `listener-crashed` 事件** — 自动调用 `forceSync()` 恢复。
3. **窗口获焦自动同步** — `onFocusChanged` 中获得焦点时调用 `forceSync()`。
4. **Filter 栏新增刷新按钮** — 标题为 "强制同步 (修复卡死)"。

**未修改的部分**

- 所有 framer-motion 动画、UI 结构。
- Actix-web 服务器、SQLite 操作、快捷键系统。

**验证结果**

- `cargo check` — 通过（仅 1 个预存的 dead_code warning）
- `npx tsc --noEmit` — 通过

**潜在问题**

| 风险 | 说明 | 后续对策 |
|------|------|---------|
| `WM_POWERBROADCAST` 可能不送达 `HWND_MESSAGE` 窗口 | 微软文档描述模糊 | 心跳定时器是最后防线；如收不到考虑改用普通隐藏窗口 |
| `forceSync` 窗口获焦时频繁调用 | 每次 Alt+V 触发一次 | 性能影响 <5ms，如需可加 2 秒节流 |
| 心跳 30 秒间隔 | 极端情况下最多 30 秒恢复 | 用户可通过刷新按钮或 Alt+V 立即触发 |

**排查指南**

1. 运行 `npm run tauri dev`，闲置 20 分钟后检查日志中是否有 `⚡ System resumed` 或 `⚠️ Heartbeat detected missed clipboard change`。
2. 如果都没有 → `WM_POWERBROADCAST` 和 `WM_TIMER` 都没送达 → 将 `HWND_MESSAGE` 改为普通隐藏窗口。
3. 如果心跳正常但剪贴板仍不工作 → 检查 arboard `Clipboard::new()` 在休眠后是否失败。
4. 如果前端 `invoke` 仍卡死 → 问题在 Tauri IPC 层 → 检查 WebView2 进程是否被系统挂起。
