# ClipFlow 前端模块化重构方案

> **状态**: ✅ 已完成 (2026-04-12)
> **创建时间**: 2026-04-11
> **范围**: `src/App.tsx` (917行) → 模块化组件树架构
> **红线**: App.css 零修改 / DOM 输出像素级等价 / 无新 npm 依赖 / 不改变任何功能行为

---

## 一、现状分析

### 1.1 当前 App.tsx 结构

| 指标 | 数值 |
|------|------|
| 总行数 | 917 |
| state 变量 | 22 个 |
| useEffect | 6 个 |
| useRef | 16 个 |
| invoke() 调用 | 23 处 |
| 内联视图 | 4 个 (home / chat / settings / viewer) |
| 事件监听器 (listen) | 7 个 |

### 1.2 核心痛点

1. **UI / State / API 三层耦合**: 所有 invoke 调用直接散落在组件内，无抽象层。
2. **God Component**: 22 个 state + 16 个 ref + 6 个 effect + 4 个视图全部内联。
3. **跨切面依赖密集**: 14 个 state/ref/function 跨越多个逻辑边界（详见 §1.3）。

### 1.3 跨切面依赖完整图谱

经 Metis 深度分析，共识别 **14 个跨切面依赖**：

| # | 变量/函数 | 严重度 | 消费者 |
|---|----------|--------|--------|
| 1 | `settings` 对象 | CRITICAL | 几乎所有 hook + 所有组件 |
| 2 | `isQueueMode` + `queueIds` + refs | CRITICAL | handleCopy / HistoryCard / SettingsView / queue-consumed 监听 |
| 3 | `handleCopy()` | HIGH | 读取 7 个 state 域；被 HistoryCard + ViewerView 调用 |
| 4 | `isPinned` | HIGH | focus handler + handleCopy + Toolbar |
| 5 | `forceSync()` | HIGH | pull-refresh + listener-crashed + window focus |
| 6 | `loadHistory()` | HIGH | clipboard-monitor 监听 + ViewerView 保存 |
| 7 | `translations` | HIGH | filteredHistory / HistoryCard / ViewerView / handleOpenViewer |
| 8 | `isPastingRef` | HIGH | handleCopy + queue paste listener |
| 9 | `isInteractingRef` | MEDIUM | focus handler + global mouseup |
| 10 | `isDraggingRef` | MEDIUM | focus handler + Toolbar mousedown + global mouseup |
| 11 | `showToast()` | MEDIUM | useQueueMode toggle + SettingsView toggle |
| 12 | `isDarkMode` | MEDIUM | Toast 内联样式 + Toolbar 品牌标题 |
| 13 | `editingId` + `editValue` | MEDIUM | handleCopy guard + HistoryCard 渲染 |
| 14 | `appWindow` 单例 | MEDIUM | Toolbar drag / focus handler / handleCopy hide / close / pin |

---

## 二、目标架构

```
src/
├── main.tsx                          # 不变
├── App.tsx                           # ~150 行：Hook 编排 + 视图路由 + 跨切面接线
├── App.css                           # 不变（红线）
│
├── types/
│   └── index.ts                      # HistoryItem, ChatMessage, ViewType
│
├── constants/
│   └── index.ts                      # TAB_ORDER, CATEGORIES
│
├── lib/
│   └── window.ts                     # appWindow = getCurrentWindow() 单例
│
├── utils/
│   ├── format.ts                     # getTruncatedText(), formatTime()
│   └── Icon.tsx                      # Icon 组件（纯 name→lucide 映射）
│
├── api/                              # 所有 invoke() 封装，带类型签名
│   ├── history.ts                    # getHistory, deleteItem, clearHistory, setCategory, updateHistoryContent
│   ├── paste.ts                      # smartCopy, pasteItem, setQueue, pasteQueueNext
│   ├── settings.ts                   # getAllSettings, saveSetting, getFileSavePath, setSavePath
│   └── sync.ts                       # forceSync, sendToPhone, translateText, getLocalIp
│
├── hooks/
│   ├── useClipboardController.ts     # history + queue + copy/paste 统一控制器
│   ├── useSettings.ts                # settings / theme / autoStart / fileSavePath
│   ├── useClipboardEvents.ts         # 所有 listen() + window focus + global mouseup
│   ├── useTabAnimation.ts            # activeTab / renderTab / listStyle 状态机
│   ├── useTranslations.ts            # translations / isTranslating / toggle / selection translate
│   ├── useViewer.ts                  # viewingItem / viewerContent / imgScale / pan / isDragging
│   ├── usePullRefresh.ts             # pull-to-refresh DOM refs + handlers
│   └── useToast.ts                   # toastMsg / showToast / timer ref
│
└── components/
    ├── Toast.tsx                      # Toast 浮层
    ├── DeleteModal.tsx                # 清空确认对话框
    ├── Toolbar.tsx                    # 顶部工具栏
    ├── FilterBar.tsx                  # 分类 Tab 栏
    ├── HistoryCard.tsx                # 单张历史卡片
    ├── HistoryList.tsx                # 列表容器（含 pull-to-refresh）
    └── views/
        ├── HomeView.tsx              # FilterBar + HistoryList 组合
        ├── ChatView.tsx              # 手机传输助手
        ├── SettingsView.tsx          # 设置页
        └── ViewerView.tsx            # 图片/文本预览编辑
```

---

## 三、架构决策记录 (ADR)

以下决策基于 Oracle 架构审查和 Metis 前置分析的联合结论。

### ADR-1: 不使用 React Context

**决策**: 所有跨切面依赖通过显式 props/参数传递，不引入 Context。

**理由**:
- Context 会在 settings 每次变化时重渲染所有消费者，除非额外加 selector 模式。
- 组件树只有 2-3 层深度，props drilling 可控。
- 结构重构中显式依赖更安全，便于追踪。

**升级触发器**: 如出现 4+ 层 props 穿透且涉及多个分支 → 引入窄 `SettingsContext`。

### ADR-2: settings 按字段传递，不传整个对象

**决策**: 每个 hook 只接收它需要的 settings 字段（如 `theme`, `queue_toggle_shortcut`），不传递整个 settings 对象。

**理由**: 避免无关 settings 字段变化触发不必要的 effect 重执行或重渲染。

### ADR-3: 合并 useHistory + useQueueMode → useClipboardController

**决策**: `handleCopy` 是最复杂的函数，读取 7 个 state 域（editingId, isPastingRef, isQueueMode, queueIdsRef, isPinned, settings, appWindow）。将 history CRUD 和 queue 操作合并到一个 controller hook 中。

**理由**:
- handleCopy 中的 queue 分支直接操作 queueIdsRef，与 history 操作通过 isPastingRef 紧密耦合。
- 分成两个 hook 会产生循环依赖或过度参数传递。

**hook 签名**:
```ts
function useClipboardController(params: {
  isPastingRef: React.MutableRefObject<boolean>;
  isPinned: boolean;
  editingId: number | null;
  settingsAutoPastePinned: string;
  settingsStayOnCopy: string;
  settingsQueueToggleShortcut: string;
  showToast: (msg: string) => void;
}): {
  history: HistoryItem[];
  pendingDeleteIds: Set<number>;
  isQueueMode: boolean;
  queueIds: number[];
  queueIdsRef: React.MutableRefObject<number[]>;
  loadHistory: () => Promise<void>;
  forceSync: () => Promise<void>;
  handleCopy: (item: HistoryItem) => Promise<void>;
  handleDelete: (e: React.MouseEvent, id: number) => Promise<void>;
  handleClearAll: () => void;
  confirmClearAll: () => Promise<void>;
  showDeleteModal: boolean;
  closeDeleteModal: () => void;
  setIsQueueMode: React.Dispatch<React.SetStateAction<boolean>>;
  setQueueIds: React.Dispatch<React.SetStateAction<number[]>>;
}
```

### ADR-4: Tab 动画提取为独立 useTabAnimation

**决策**: `activeTab` / `renderTab` / `listStyle` / `tabSwitchTimerRef` / `handleTabChange` 构成一个紧耦合的动画状态机，必须作为整体提取。

**归属**: HomeView 或 App.tsx（渲染 tab 面板的组件拥有此 hook），**不是 FilterBar**。FilterBar 只接收 `activeTab` 和 `onTabChange` props。

**hook 签名**:
```ts
function useTabAnimation(): {
  activeTab: string;
  renderTab: string;
  listStyle: { opacity: number; transform: string; transition: string };
  handleTabChange: (id: string) => void;
}
```

### ADR-5: Viewer 状态封装为单一 useViewer

**决策**: 将 viewingItem / viewerContent / imgScale / pan / isDragging / textareaRef 以及相关 handlers 封装到一个 `useViewer` hook。

**升级触发器**: 只有当 image zoom 代码独立膨胀到影响可读性时，才分出 `useImageZoom`。

### ADR-6: usePullRefresh 归属 HistoryList 内部

**决策**: pull-to-refresh 逻辑（DOM refs + 直接 DOM 操控）在 HistoryList 组件内调用 usePullRefresh，返回 refs 和事件 handlers。

**理由**: DOM 操控逻辑必须靠近它操控的 DOM 节点。如果父组件也需要 listRef，用 callback ref 组合。

### ADR-7: 大 useEffect 按独立关注点拆分

**决策**: 当前主 useEffect（第 158-205 行）注册 7 个 listen + 2 个事件 handler + 1 个 focus handler。拆分规则：

- **useClipboardEvents**: clipboard-monitor / new_message / mobile-connected / mobile-msg / listener-crashed 的 listen，以及 window focus handler 和 global mouseup handler
- **useClipboardController**: queue-consumed listen 和 trigger-queue-paste listen（因为它们操作 queueIds）
- **useSettings**: 系统主题 mediaQuery 监听
- 各 hook 的 mount-time init（loadHistory / loadIp / loadSettings / checkAutoStart）由各自 hook 管理

**约束**: 如果多个 listener 之间存在共享 ref 交互（如 focus handler 读 isDraggingRef），必须在同一个 effect 中。

### ADR-8: App.tsx 目标 ~150 行

**决策**: 80 行不现实。App.tsx 需要持有：
- 3 个交互锁 ref（isPastingRef, isInteractingRef, isDraggingRef）
- 跨视图 state（isPinned, currentView, viewingItem, editingId/editValue, searchText）
- filteredHistory useMemo 计算
- 视图路由 JSX + props 接线

---

## 四、State 归属地图

### App.tsx 持有（跨切面）

| State / Ref | 类型 | 消费者 |
|-------------|------|--------|
| `isPastingRef` | Ref | useClipboardController, useClipboardEvents |
| `isInteractingRef` | Ref | useClipboardEvents |
| `isDraggingRef` | Ref | useClipboardEvents, Toolbar |
| `isPinned` / `setIsPinned` | State | useClipboardController, useClipboardEvents, Toolbar |
| `currentView` / `setCurrentView` | State | App 路由, Toolbar, ViewerView |
| `viewingItem` / `setViewingItem` | State | useViewer, ViewerView |
| `editingId` / `editValue` + setters | State | useClipboardController (guard), HistoryCard |
| `searchText` / `setSearchText` | State | Toolbar, filteredHistory 计算 |
| `isDarkMode` | Derived | Toast, Toolbar |

### useClipboardController 持有

| State / Ref | 消费者 |
|-------------|--------|
| `history` / `setHistory` | HistoryList, filteredHistory |
| `pendingDeleteIds` | filteredHistory |
| `isQueueMode` / `setIsQueueMode` | HistoryCard, SettingsView |
| `queueIds` / `setQueueIds` | HistoryCard |
| `queueIdsRef` / `isQueueModeRef` | 内部 |
| `showDeleteModal` | DeleteModal |
| `handleCopy` | HistoryCard, ViewerView |
| `forceSync` | usePullRefresh, useClipboardEvents |
| `loadHistory` | useClipboardEvents |

### useSettings 持有

| State | 消费者 |
|-------|--------|
| `settings` / `updateSetting` | SettingsView (全部), 各 hook (按字段) |
| `autoStart` / `toggleAutoStart` | SettingsView |
| `fileSavePath` / `handlePickFolder` | SettingsView |
| `applyTheme` | 内部 |

### useTabAnimation 持有

| State | 消费者 |
|-------|--------|
| `activeTab` | FilterBar |
| `renderTab` | filteredHistory, HistoryList (key) |
| `listStyle` | HistoryList (style) |
| `handleTabChange` | FilterBar |

### useTranslations 持有

| State | 消费者 |
|-------|--------|
| `translations` | filteredHistory, HistoryCard, ViewerView |
| `isTranslating` | ViewerView |
| `handleToggleTranslate` | HistoryCard |
| `handleViewerTranslateToggle` | ViewerView |
| `handleTranslateSelection` | ViewerView |
| `selectionRestore` / `setSelectionRestore` | ViewerView |

### useViewer 持有

| State | 消费者 |
|-------|--------|
| `viewerContent` / `setViewerContent` | ViewerView |
| `imgScale` / `pan` / `isDragging` | ViewerView |
| `textareaRef` | ViewerView |
| `handleWheel` / `handleMouseDown` / `handleMouseMove` / `handleMouseUp` | ViewerView |
| `handleSaveViewerContent` | ViewerView |
| `handleOpenViewer` | HistoryCard |
| `handleCloseViewer` | Toolbar (返回按钮) |

### useToast 持有

| State | 消费者 |
|-------|--------|
| `toastMsg` | Toast |
| `showToast` | useClipboardController, SettingsView |

### usePullRefresh 持有（HistoryList 内部）

| Ref / Function | 消费者 |
|----------------|--------|
| `pullIndicatorRef` / `pullIconRef` / `pullTextRef` | HistoryList JSX |
| `isRefreshing` | HistoryList JSX |
| pull 事件 handlers | HistoryList 事件绑定 |

---

## 五、执行阶段

### Phase 0: 基础设施（无依赖）

- [x] **P0-A**: 创建 `src/types/index.ts` — 提取 `HistoryItem`, `ChatMessage`, `ViewType` 接口
- [x] **P0-B**: 创建 `src/constants/index.ts` — 提取 `TAB_ORDER`, `CATEGORIES` 常量
- [x] **P0-C**: 创建 `src/lib/window.ts` — 导出 `appWindow = getCurrentWindow()` 单例
- [x] **P0-D**: 创建 `src/utils/format.ts` — 提取 `getTruncatedText()`, `formatTime()`
- [x] **P0-E**: 创建 `src/utils/Icon.tsx` — 提取 `Icon` 组件

**验证**: `npx tsc --noEmit` 通过

### Phase 1: API 层（依赖 types）

- [x] **P1-A**: 创建 `src/api/history.ts` — 封装 `get_history`, `delete_item`, `clear_history`, `set_category`, `update_history_content`
- [x] **P1-B**: 创建 `src/api/paste.ts` — 封装 `smart_copy`, `paste_item`, `set_queue`, `paste_queue_next`
- [x] **P1-C**: 创建 `src/api/settings.ts` — 封装 `get_all_settings`, `save_setting`, `get_file_save_path`, `set_save_path`
- [x] **P1-D**: 创建 `src/api/sync.ts` — 封装 `force_sync`, `send_to_phone`, `translate_text`, `get_local_ip`

**约束**: 保持 invoke 泛型签名完全一致（如 `invoke<HistoryItem[]>("get_history")`）

**验证**: `npx tsc --noEmit` 通过

### Phase 2: 自定义 Hooks（依赖 types + api）

- [x] **P2-A**: 创建 `src/hooks/useToast.ts` — 独立，无外部依赖
- [x] **P2-B**: 创建 `src/hooks/useSettings.ts` — 独立，管理自身 mount-time init
- [x] **P2-C**: 创建 `src/hooks/useTabAnimation.ts` — 独立状态机
- [x] **P2-D**: 创建 `src/hooks/useTranslations.ts` — 依赖 api/sync
- [x] **P2-E**: 创建 `src/hooks/useViewer.ts` — 依赖 api/history, useTranslations 的 translations
- [x] **P2-F**: 创建 `src/hooks/useClipboardController.ts` — 依赖 api/history + api/paste，接收跨切面 refs/state 为参数
- [x] **P2-G**: 创建 `src/hooks/usePullRefresh.ts` — 接收 forceSync 为参数
- [x] **P2-H**: 创建 `src/hooks/useClipboardEvents.ts` — 接收 forceSync, loadHistory, settings 字段, isPinned, 交互锁 refs 为参数

**验证**: `npx tsc --noEmit` 通过

### Phase 3: UI 组件（依赖 hooks + utils）

- [x] **P3-A**: 创建 `src/components/Toast.tsx` — 接收 `msg: string`, `isDarkMode: boolean`，保持精确内联样式
- [x] **P3-B**: 创建 `src/components/DeleteModal.tsx` — 接收 `show`, `onClose`, `onConfirm`
- [x] **P3-C**: 创建 `src/components/Toolbar.tsx` — 接收 currentView, isPinned, isDarkMode, settings (具体字段), searchText, 各 handler
- [x] **P3-D**: 创建 `src/components/FilterBar.tsx` — 接收 activeTab, onTabChange, onScrollToTop
- [x] **P3-E**: 创建 `src/components/HistoryCard.tsx` — 接收 item + 全部 action handlers + queue/translate/edit state
- [x] **P3-F**: 创建 `src/components/HistoryList.tsx` — 内含 usePullRefresh，接收 filteredHistory, renderedHistory, handlers
- [x] **P3-G**: 创建 `src/components/views/ViewerView.tsx` — 使用 useViewer 的返回值
- [x] **P3-H**: 创建 `src/components/views/ChatView.tsx` — 接收 chat state + handlers
- [x] **P3-I**: 创建 `src/components/views/SettingsView.tsx` — 接收 settings + handlers
- [x] **P3-J**: 创建 `src/components/views/HomeView.tsx` — 组合 FilterBar + HistoryList

**验证**: `npx tsc --noEmit` 通过; `npm run build` 通过

### Phase 4: App.tsx 重写 + 旧代码删除

- [x] **P4-A**: 重写 `App.tsx` — 调用所有 hooks，计算 filteredHistory，按 currentView 渲染视图组件
- [x] **P4-B**: 验证 motion 元素数量 — `ast_grep_search` 确认 7 个 `motion.div` + 1 个 `motion.button`
- [x] **P4-C**: 验证 CSS class 名完整性 — grep 确认所有原 className 字符串存在于新组件中
- [x] **P4-D**: `npx tsc --noEmit` + `npm run build` 通过
- [ ] **P4-E**: `npm run tauri dev` 手动功能验证 (需用户手动执行)

---

## 六、AI 执行红线清单

每个 Phase 完成后必须逐条检查。违反任何一条即回滚。

### 绝对禁止

1. **禁止修改 App.css** — 零增删改
2. **禁止改变 CSS class name 字符串** — 所有 className 原样保留
3. **禁止将 `motion.div` / `motion.button` 降级为普通 HTML 元素**
4. **禁止将内联样式转为 CSS class 或反向转换**
5. **禁止新增包裹 div 改变 DOM 嵌套层级**（CSS 选择器可能依赖结构）
6. **禁止改变 `useEffect` 依赖数组的值** — 只移动 effect，不改 deps
7. **禁止将 `updatePullUI` 的 DOM 直接操控转为 React state**（会导致拖动时性能退化）
8. **禁止改变 `renderedHistory = filteredHistory.slice(0, 50)` 上限**

### 必须保留

1. `key={renderTab}` 在 `<div className="cards-container">` 上
2. 根 div: `className={`app-container ${!isPinned ? 'unpinned' : ''}`}` 留在 App.tsx
3. Toast 内联样式对象精确传递（引用 `isDarkMode` 和 `toastMsg`）
4. `queueIdsRef.current` 同步在所有修改 `queueIds` 的地方保持
5. `e.stopPropagation()` 在 HistoryCard 各按钮中保留
6. `appWindow` 通过 `src/lib/window.ts` 单例导入，不重复创建
7. 所有 `whileHover` / `whileTap` framer-motion props 原样保留

---

## 七、验证方案

### 自动验证（每个 Phase 后）

```bash
npx tsc --noEmit          # TypeScript 类型检查
npm run build             # Vite 构建
```

### 结构验证（Phase 4 后）

```bash
# 验证 motion 元素数量（ast-grep）
# 期望: 6 个 motion.div + 1 个 motion.button

# 验证所有 CSS class name 完整性（grep）
# 从原始 App.tsx 提取所有 className，确认新组件中全部存在
```

### 功能验证（Phase 4 后，`npm run tauri dev`）

- [ ] 复制条目（单击卡片）→ 内容写入剪贴板
- [ ] PIN 模式 + 自动粘贴工作
- [ ] 队列模式 (Alt+1) 开启/关闭/顺序粘贴
- [ ] 翻译按钮（列表 + Viewer 全文 + Viewer 选中翻译）
- [ ] 下拉刷新触发 force_sync
- [ ] Tab 切换动画流畅无断裂
- [ ] 设置页所有开关/输入持久化
- [ ] 手机传输页 QR 码、聊天收发
- [ ] 图片 Viewer 缩放/拖拽
- [ ] 深色/浅色模式切换
- [ ] 失焦自动隐藏（非 PIN 模式）
- [ ] 清空历史对话框弹出/取消/确认
- [ ] 删除单条（乐观删除，无闪烁）
- [ ] Toast 通知正常显示和消失

---

## 八、风险矩阵

| 风险 | 严重度 | 缓解措施 |
|------|--------|---------|
| `handleCopy` 提取后闭包引用 stale `editingId` | HIGH | 在 useClipboardController 内通过 ref mirror 或 useCallback 正确捕获 |
| Tab 动画状态机拆分后时序错乱 | HIGH | 作为整体提取到 useTabAnimation，保持 setTimeout + rAF 序列不变 |
| `motion.div` 被误降级为 `div` | HIGH | Phase 4 后用 ast-grep 计数验证 |
| mount-time init 执行顺序改变 | MEDIUM | 各 hook 独立管理 init；验证 loadHistory/loadSettings/loadIp/checkAutoStart 无顺序依赖 |
| `forceSync` 被多处重复定义 | MEDIUM | 仅在 useClipboardController 中定义，通过返回值传递给其他消费者 |
| pull-to-refresh DOM refs 断连 | MEDIUM | usePullRefresh 在 HistoryList 内调用，refs 直接绑定到 JSX |
| `e.stopPropagation()` 在组件提取后失效 | MEDIUM | HistoryCard 内各按钮保持 onClick 中的 stopPropagation 调用 |
| useEffect cleanup 顺序改变导致 listener 泄漏 | MEDIUM | 每个 hook 自行管理 cleanup；不依赖跨 hook cleanup 顺序 |
| `appWindow` 单例被重复实例化 | LOW | 统一从 src/lib/window.ts 导入 |

---

## 九、工作量估算

| Phase | 文件数 | 预估难度 | 预估时间 |
|-------|--------|---------|---------|
| P0 基础设施 | 5 | 低 | - |
| P1 API 层 | 4 | 低 | - |
| P2 Hooks | 8 | 中-高 | - |
| P3 组件 | 10 | 中 | - |
| P4 组合 | 1 + 验证 | 中 | - |
| **总计** | **~28 个新文件** | 中 | 1-2 天 |
