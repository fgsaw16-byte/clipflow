import { useState, useEffect, useRef, useLayoutEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import { QRCodeSVG } from "qrcode.react";
import { enable, disable, isEnabled } from "@tauri-apps/plugin-autostart";
import { motion, AnimatePresence } from "framer-motion";
import "./App.css";

import { 
  Search, Trash2, Settings, Smartphone, X,
  ArrowUp, Image as ImageIcon, FileText, Code, 
  Star, ArrowLeft, Pin, Globe, Edit, Eye, RefreshCw, Plus, Ban, Languages, Folder
} from "lucide-react";

const appWindow = getCurrentWindow();

interface HistoryItem { id: number; content: string; created_at: string; category: string; }
interface ChatMessage { text: string; isMe: boolean; } 
type ViewType = 'home' | 'chat' | 'settings' | 'viewer';

 const TAB_ORDER = ['all', 'text', 'image', 'code', 'custom'];

const CATEGORIES = [
  { id: 'all', label: '全部' },
  { id: 'text', label: '文本' },
  { id: 'image', label: '图片' },
  { id: 'code', label: '代码' },
  { id: 'custom', label: '自定义' }, 
];

const Icon = ({ name, size = 16, style = {} }: any) => {
  const props = { size, style };
  switch (name) {
    case 'search': return <Search {...props} />;
    case 'trash': return <Trash2 {...props} />;
    case 'settings': return <Settings {...props} />;
    case 'phone': return <Smartphone {...props} />;
    case 'close': return <X {...props} />;
    case 'back': return <ArrowLeft {...props} />;
    case 'arrow-up': return <ArrowUp {...props} />;
    case 'eye': return <Eye {...props} />;
    case 'edit': return <Edit {...props} />;
    case 'globe': return <Globe {...props} />;
    case 'ban': return <Ban {...props} />;
    case 'text': return <FileText {...props} />;
    case 'image': return <ImageIcon {...props} />;
    case 'code': return <Code {...props} />;
    case 'custom': return <Star {...props} />;
    case 'refresh': return <RefreshCw {...props} />;
    case 'translate-selection': return <Languages {...props} />;
    case 'plus-img': return <Plus {...props} />;
    case 'pin': return <Pin {...props} />;
    default: return null;
  }
};

const getTruncatedText = (text: string, maxLength: number = 150) => {
  if (text.startsWith("data:image")) return "🖼️ [图片数据]";
  if (text.length <= maxLength) return text;
  return text.substring(0, maxLength) + "...";
};

const formatTime = (d: string) => { 
  const diff = (Date.now()-new Date(d+"Z").getTime())/60000; 
  if(diff<1)return"刚刚";
  if(diff<60)return`${Math.floor(diff)}分钟前`; 
  const dt=new Date(d+"Z"); 
  return`${dt.getHours()}:${dt.getMinutes().toString().padStart(2,'0')}`; 
};

function App() {
  const [history, setHistory] = useState<HistoryItem[]>([]);
  const [searchText, setSearchText] = useState("");
  const [activeTab, setActiveTab] = useState('all');
  const [renderTab, setRenderTab] = useState('all');
  const [listStyle, setListStyle] = useState({
    opacity: 1,
    transform: 'translateX(0px)',
    transition: 'none'
  });
  const [currentView, setCurrentView] = useState<ViewType>('home');

  const tabSwitchTimerRef = useRef<number | null>(null);
  const [viewingItem, setViewingItem] = useState<HistoryItem | null>(null);
  
  const [isPinned, setIsPinned] = useState(false);
  
  // 🔴 核心：交互状态锁
  const isInteractingRef = useRef(false);
  // 🔴 核心：粘贴防抖锁 (防止连点)
  const isPastingRef = useRef(false);
  // 🔴 拖动锁：拖动时阻断失焦隐藏
  const isDraggingRef = useRef(false);

  const [settings, setSettings] = useState<any>({});
  const [autoStart, setAutoStart] = useState(false);
  const [ipAddress, setIpAddress] = useState("127.0.0.1");
  const [isConnected, setIsConnected] = useState(false);
  const [chatMessages, setChatMessages] = useState<ChatMessage[]>([]);
  const [chatInput, setChatInput] = useState("");
  const [isDarkMode, setIsDarkMode] = useState(false);
  const [editingId, setEditingId] = useState<number | null>(null); 
  const [editValue, setEditValue] = useState("");
  const [viewerContent, setViewerContent] = useState("");
  const [translations, setTranslations] = useState<Record<number, string>>({});
  const [isTranslating, setIsTranslating] = useState(false);
  const [selectionRestore, setSelectionRestore] = useState<{start: number; translatedLength: number; originalText: string;} | null>(null);
  const [showDeleteModal, setShowDeleteModal] = useState(false);
  const [imgScale, setImgScale] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [isDragging, setIsDragging] = useState(false);
  const [isQueueMode, setIsQueueMode] = useState(false);
  const [queueIds, setQueueIds] = useState<number[]>([]);
  const [fileSavePath, setFileSavePath] = useState("");
  const [toastMsg, setToastMsg] = useState("");
  const toastTimerRef = useRef<number | null>(null);
  const isQueueModeRef = useRef(isQueueMode);
  const queueIdsRef = useRef(queueIds);

  const listRef = useRef<HTMLDivElement>(null); 
  const scrollPosRef = useRef(0);
  const chatEndRef = useRef<HTMLDivElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    loadHistory(); loadIp(); loadSettings(); checkAutoStart(); 
    const unlistenClip = listen("clipboard-monitor", () => loadHistory());
    const unlistenNew = listen("new_message", () => loadHistory());
    const unlistenMobile = listen("mobile-connected", () => setIsConnected(true));
    const unlistenMsg = listen<string>("mobile-msg", (e) => setChatMessages(p => [...p, { text: e.payload, isMe: false }]));
    const unlistenQueue = listen<number>("queue-consumed", () => setQueueIds(prev => {
      const next = prev.slice(1);
      queueIdsRef.current = next;
      return next;
    }));
    // Auto-recover when the backend clipboard monitor crashes or gets stuck
    const unlistenCrash = listen("listener-crashed", () => {
      console.warn("[clipflow] Listener crashed — triggering force_sync recovery");
      forceSync();
    });

    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    const handleSystemThemeChange = () => { if (settings.theme === 'system' || !settings.theme) applyTheme('system'); };
    mediaQuery.addEventListener('change', handleSystemThemeChange);
    
    // 🔴 全局 MouseUp：防止拖动出窗口后卡死状态
    const handleGlobalMouseUp = () => { 
      isInteractingRef.current = false; 
      isDraggingRef.current = false;
    };
    window.addEventListener('mouseup', handleGlobalMouseUp);

    const unlistenFocus = appWindow.onFocusChanged(({ payload: focused }) => {
      // 正在拖动，直接忽略
      if (isDraggingRef.current) return;
      if (focused) {
        // Window gained focus — force sync to recover from any sleep/hibernate lockup
        forceSync();
      }
      // 🔴 失焦逻辑修正：
      // 只有在【没钉住】且【没在交互/拖动】时，才隐藏
      if (!focused && !isPinned && !isInteractingRef.current) {
          setTimeout(() => appWindow.hide(), 50); 
      }
    });

    return () => { 
        unlistenClip.then(f=>f()); unlistenNew.then(f=>f()); unlistenMobile.then(f=>f()); unlistenMsg.then(f=>f()); unlistenQueue.then(f=>f()); unlistenCrash.then(f=>f()); unlistenFocus.then(f=>f()); 
        mediaQuery.removeEventListener('change', handleSystemThemeChange); 
        window.removeEventListener('mouseup', handleGlobalMouseUp);
    };
  }, [settings.theme, isPinned]); // 移除了 stay_on_copy 依赖，避免逻辑混乱

  useEffect(() => {
    isQueueModeRef.current = isQueueMode;
  }, [isQueueMode]);

  useEffect(() => {
    queueIdsRef.current = queueIds;
  }, [queueIds]);

  const handlePasteQueue = async () => {
    await invoke("paste_queue_next");
  };

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setupListener = async () => {
      // Safety: ensure previous listener (if any) is removed
      if (unlisten) unlisten();

      unlisten = await listen('trigger-queue-paste', async () => {
        // ---  ️ GUARD 1: Mode Check ---
        if (!isQueueModeRef.current) return;

        // --- 🛡️ GUARD 2: Anti-Spam Lock ---
        if (isPastingRef.current) {
          console.log("🚫 Paste Blocked: Too fast / Already pasting");
          return;
        }

        // 🔒 LOCK
        isPastingRef.current = true;
        console.log("✅ Executing Queue Paste...");

        try {
          await handlePasteQueue();
        } catch (err) {
          console.error(err);
        } finally {
          setTimeout(() => {
            isPastingRef.current = false;
          }, 300);
        }
      });
    };

    setupListener();
    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  // ✅ Safe keyboard listener: toggle Queue Mode via settings.queue_toggle_shortcut
  useEffect(() => {
    const shortcut = (settings.queue_toggle_shortcut || 'Alt+1').toString();
    const keys = shortcut.toLowerCase().split('+').map((k: string) => k.trim());
    const mainKey = keys.find((k: string) => !['alt', 'ctrl', 'shift'].includes(k));

    const handleKeyDown = (e: KeyboardEvent) => {
      const needsAlt = keys.includes('alt');
      const needsCtrl = keys.includes('ctrl');
      const needsShift = keys.includes('shift');

      if (needsAlt && !e.altKey) return;
      if (needsCtrl && !e.ctrlKey) return;
      if (needsShift && !e.shiftKey) return;
      if (mainKey && e.key.toLowerCase() !== mainKey) return;

      if (!needsAlt && !needsCtrl && !needsShift) return;

      e.preventDefault();
      setIsQueueMode((prev) => {
        const newMode = !prev;
        if (!newMode) {
          queueIdsRef.current = [];
          setQueueIds([]);
          invoke("set_queue", { ids: [] });
        }
        showToast(newMode ? '队列模式已开启' : '队列模式已关闭');
        return newMode;
      });
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [settings.queue_toggle_shortcut]);

  // ✅ Cleanup: avoid dangling toast timer
  useEffect(() => {
    return () => {
      if (toastTimerRef.current) window.clearTimeout(toastTimerRef.current);
    };
  }, []);

  const handleToolbarMouseDown = (e: React.MouseEvent) => {
    const target = e.target as HTMLElement;
    if (target.closest('.no-drag')) return;
    isDraggingRef.current = true;
    appWindow.startDragging()
      .catch((err: unknown) => { console.error(err); })
      .finally(() => { setTimeout(() => { isDraggingRef.current = false; }, 100); });
  };

  useEffect(() => { if (currentView === 'chat') chatEndRef.current?.scrollIntoView({ behavior: "smooth" }); }, [chatMessages, currentView]);

  useLayoutEffect(() => { 
    if (currentView === 'home' && listRef.current) {
      listRef.current.scrollTop = scrollPosRef.current; 
    }
  }, [currentView]);

  async function loadHistory() { setHistory(await invoke<HistoryItem[]>("get_history")); }

  // Hard-reset: clears stuck locks, force-reads clipboard, restarts monitor, returns fresh history.
  // Used by pull-to-refresh and listener-crashed auto-recovery.
  async function forceSync() {
    try {
      const items = await invoke<HistoryItem[]>("force_sync");
      setHistory(items);
    } catch (e) {
      console.error("force_sync failed, falling back to get_history:", e);
      try { setHistory(await invoke<HistoryItem[]>("get_history")); } catch (_) {}
    }
  }
  async function loadIp() { setIpAddress(await invoke("get_local_ip")); }
  async function loadSettings() { 
    const s = await invoke<any>("get_all_settings"); 
    setSettings(s); 
    applyTheme(s.theme || 'system'); 
    // Load file save path
    const path = await invoke<string>("get_file_save_path");
    setFileSavePath(path);
  }
  
  async function handlePickFolder() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "选择文件接收位置",
      });
      if (typeof selected === "string" && selected) {
        setFileSavePath(selected);
        await invoke("set_save_path", { path: selected });
      }
    } catch (e) {
      console.error("Failed to pick folder:", e);
    }
  }
  
  // 🔥🔥🔥 复制 + 粘贴 逻辑 (防抖优化) 🔥🔥🔥
  async function handleCopy(item: HistoryItem) {
    if (editingId !== null) return;
    if (isPastingRef.current) return; // 🔴 防止连点卡死

    if (isQueueMode) {
      const currentQueue = queueIdsRef.current;
      const nextQueue = currentQueue.includes(item.id)
        ? currentQueue.filter((id) => id !== item.id)
        : [...currentQueue, item.id];
      queueIdsRef.current = nextQueue;
      setQueueIds(nextQueue);
      await invoke("set_queue", { ids: nextQueue });
      return;
    }

    isPastingRef.current = true; // 上锁

    try {
      if (isPinned && settings.auto_paste_pinned === 'true') {
        // 固定模式 + 自动粘贴：调用原子化粘贴命令（后端完成所有操作）
        await invoke("paste_item", { id: item.id });
        isPastingRef.current = false; // 解锁
      } else {
        // 非固定模式或未开启自动粘贴：仅执行智能复制
        await invoke("smart_copy", { id: item.id });
        
        if (!isPinned && settings.stay_on_copy !== 'true') {
          await appWindow.hide();
        }
        isPastingRef.current = false;
      }
    } catch (e) {
      console.error("Copy Failed:", e);
      isPastingRef.current = false;
    }
  }

  async function updateSetting(key: string, value: string) { setSettings((p:any)=>({...p,[key]:value})); await invoke("save_setting", { key, value }); if (key === 'theme') applyTheme(value); }
  async function checkAutoStart() { setAutoStart(await isEnabled()); }
  async function toggleAutoStart() { try { if (autoStart) { await disable(); setAutoStart(false); } else { await enable(); setAutoStart(true); } } catch (e) { console.error(e); } }
  function applyTheme(theme: string) { const isDark = theme === 'dark' || (theme === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches); if (isDark) document.body.classList.add('dark-mode'); else document.body.classList.remove('dark-mode'); setIsDarkMode(isDark); }
  const showToast = (message: string) => {
    setToastMsg(message);
    if (toastTimerRef.current) window.clearTimeout(toastTimerRef.current);
    toastTimerRef.current = window.setTimeout(() => setToastMsg(""), 1800);
  };

  const scrollToTop = () => { if (listRef.current) { listRef.current.scrollTo({ top: 0, behavior: 'smooth' }); scrollPosRef.current = 0; } };

  const handlePcSend = async () => { if (!chatInput.trim()) return; const t = chatInput; await invoke("send_to_phone", { content: t }); setChatMessages(p => [...p, { text: t, isMe: true }]); setChatInput(""); };
  const handleImageUpload = (e: React.ChangeEvent<HTMLInputElement>) => { const file = e.target.files?.[0]; if (!file) return; const reader = new FileReader(); reader.onload = async (event) => { const base64 = event.target?.result as string; if (base64) { await invoke("send_to_phone", { content: base64 }); setChatMessages(p => [...p, { text: base64, isMe: true }]); } }; reader.readAsDataURL(file); e.target.value = ''; };
  const handleChatPaste = (e: React.ClipboardEvent) => { const items = e.clipboardData.items; for (let i = 0; i < items.length; i++) { if (items[i].type.indexOf("image") !== -1) { e.preventDefault(); const blob = items[i].getAsFile(); if (blob) { const reader = new FileReader(); reader.onload = async (event) => { const base64 = event.target?.result as string; if (base64) { await invoke("send_to_phone", { content: base64 }); setChatMessages(p => [...p, { text: base64, isMe: true }]); } }; reader.readAsDataURL(blob); } return; } } };
  const handleChatKeyDown = (e: React.KeyboardEvent) => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); handlePcSend(); } };
  async function handleDelete(e: React.MouseEvent, id: number) { e.stopPropagation(); await invoke("delete_item", { id }); setHistory(p => p.filter(i => i.id !== id)); }
  async function handleClearAll() { setShowDeleteModal(true); }
  const closeDeleteModal = () => setShowDeleteModal(false);

  async function confirmClearAll() {
    await invoke("clear_history");
    setHistory([]);
    closeDeleteModal();
  }
  async function toggleCategory(e: React.MouseEvent, item: HistoryItem) { e.stopPropagation(); const cats = ['text', 'code', 'custom']; let next = 'text'; if (cats.includes(item.category)) { next = cats[(cats.indexOf(item.category)+1)%cats.length]; } if (item.category === 'image') return; setHistory(prev => prev.map(h => h.id === item.id ? { ...h, category: next } : h)); await invoke("set_category", { id: item.id, category: next }); }
  const handleStartEdit = (e: React.MouseEvent, item: HistoryItem) => { e.stopPropagation(); setEditingId(item.id); setEditValue(item.category); };
  const handleSaveEdit = async (id: number) => { const val = editValue.trim() || "text"; setHistory(p => p.map(h => h.id === id ? { ...h, category: val } : h)); await invoke("set_category", { id: id, category: val }); setEditingId(null); };
  const handleKeyDown = (e: React.KeyboardEvent, id: number) => { if (e.key === 'Enter') handleSaveEdit(id); else if (e.key === 'Escape') setEditingId(null); };
  const handleOpenViewer = (e: React.MouseEvent, item: HistoryItem) => { e.stopPropagation(); setViewingItem(item); const currentContent = translations[item.id] || item.content; setViewerContent(currentContent); setImgScale(1); setPan({ x: 0, y: 0 }); setSelectionRestore(null); setCurrentView('viewer'); };
  const handleCloseViewer = () => { setCurrentView('home'); setViewingItem(null); setSelectionRestore(null); };
  const handleSaveViewerContent = async () => { if (!viewingItem) return; await invoke("update_history_content", { id: viewingItem.id, content: viewerContent }); if (translations[viewingItem.id]) { const newTrans = { ...translations }; delete newTrans[viewingItem.id]; setTranslations(newTrans); } setCurrentView('home'); loadHistory(); };
  const handleWheel = (e: React.WheelEvent) => { if (viewingItem?.category === 'image') { setImgScale(prev => Math.max(0.1, prev + (e.deltaY > 0 ? -0.1 : 0.1))); } };
  const handleMouseDown = (e: React.MouseEvent) => { if (viewingItem?.category !== 'image') return; e.preventDefault(); setIsDragging(true); };
  const handleMouseMove = (e: React.MouseEvent) => { if (!isDragging) return; setPan(prev => ({ x: prev.x + e.movementX, y: prev.y + e.movementY })); };
  const handleMouseUp = () => setIsDragging(false);

  const handleTabChange = (id: string) => {
    const newIndex = TAB_ORDER.indexOf(id);
    const oldIndex = TAB_ORDER.indexOf(activeTab);
    const direction = newIndex > oldIndex ? -1 : 1;

    setActiveTab(id);

    if (id !== renderTab) {
      setListStyle({
        opacity: 0,
        transform: `translateX(${direction * -30}px)`,
        transition: 'all 0.2s ease-in'
      });

      if (tabSwitchTimerRef.current) {
        window.clearTimeout(tabSwitchTimerRef.current);
      }

      tabSwitchTimerRef.current = window.setTimeout(() => {
        setRenderTab(id);
        setListStyle({
          opacity: 0,
          transform: `translateX(${direction * 30}px)`,
          transition: 'none'
        });

        requestAnimationFrame(() => {
          requestAnimationFrame(() => {
            setListStyle({
              opacity: 1,
              transform: 'translateX(0px)',
              transition: 'all 0.25s cubic-bezier(0.2, 0.8, 0.2, 1)'
            });
          });
        });

        tabSwitchTimerRef.current = null;
      }, 200);
    }
  };

  useEffect(() => {
    return () => {
      if (tabSwitchTimerRef.current) {
        window.clearTimeout(tabSwitchTimerRef.current);
        tabSwitchTimerRef.current = null;
      }
    };
  }, []);
  const handleToggleTranslate = async (e: React.MouseEvent, item: HistoryItem) => { e.stopPropagation(); if (translations[item.id]) { const newTrans = { ...translations }; delete newTrans[item.id]; setTranslations(newTrans); } else { try { const res = await invoke<string>('translate_text', { content: item.content }); setTranslations(prev => ({ ...prev, [item.id]: res })); } catch (err) { console.error(err); } } };
  const handleViewerTranslateToggle = async () => { if (!viewingItem || isTranslating) return; if (translations[viewingItem.id]) { const newTrans = { ...translations }; delete newTrans[viewingItem.id]; setTranslations(newTrans); setViewerContent(viewingItem.content); } else { setIsTranslating(true); try { const res = await invoke<string>('translate_text', { content: viewingItem.content }); setTranslations(prev => ({ ...prev, [viewingItem.id]: res })); setViewerContent(res); } catch (err) { console.error(err); } setIsTranslating(false); } };
  const handleTranslateSelection = async () => { if (!textareaRef.current || isTranslating) return; if (selectionRestore) { const { start, translatedLength, originalText } = selectionRestore; const end = start + translatedLength; const newContent = viewerContent.substring(0, start) + originalText + viewerContent.substring(end); setViewerContent(newContent); setSelectionRestore(null); return; } const textarea = textareaRef.current; const start = textarea.selectionStart; const end = textarea.selectionEnd; if (start === end) return; const selectedText = viewerContent.substring(start, end); setIsTranslating(true); try { const translatedText = await invoke<string>('translate_text', { content: selectedText }); const newContent = viewerContent.substring(0, start) + translatedText + viewerContent.substring(end); setViewerContent(newContent); setSelectionRestore({ start: start, translatedLength: translatedText.length, originalText: selectedText }); } catch (err) { console.error(err); } setIsTranslating(false); };

  const filteredHistory = history.filter((item) => {
    if (renderTab === 'custom') { if (['text', 'image', 'code'].includes(item.category)) return false; }
    else if (renderTab !== 'all') { if (item.category !== renderTab) return false; }
    if (!searchText) return true;
    const lowerSearch = searchText.toLowerCase();
    if (item.category === 'image') return false;
    const contentToCheck = translations[item.id] || item.content;
    return contentToCheck.toLowerCase().includes(lowerSearch) || item.category.toLowerCase().includes(lowerSearch);
  });

  const renderedHistory = filteredHistory.slice(0, 50);

  return (
    <div className={`app-container ${!isPinned ? 'unpinned' : ''}`}>
      <div
        style={{
          position: 'fixed',
          bottom: '48px',
          left: '50%',
          transform: toastMsg ? 'translateX(-50%) translateY(0)' : 'translateX(-50%) translateY(16px)',
          zIndex: 100,
          transition: 'all 300ms',
          opacity: toastMsg ? 1 : 0,
          pointerEvents: toastMsg ? 'auto' : 'none',
        }}
      >
        <div
          style={{
            display: 'flex',
            width: 'fit-content',
            alignItems: 'center',
            justifyContent: 'center',
            borderRadius: '9999px',
            border: '1px solid rgba(255, 255, 255, 0.2)',
            backgroundColor: isDarkMode ? 'rgba(31, 41, 55, 0.8)' : 'rgba(255, 255, 255, 0.8)',
            padding: '8px 24px',
            boxShadow: '0 12px 32px rgba(0,0,0,0.18)',
            backdropFilter: 'blur(12px)',
            WebkitBackdropFilter: 'blur(12px)',
          }}
        >
          <span style={{ fontSize: '13px', fontWeight: 600, color: isDarkMode ? '#e5e7eb' : '#374151' }}>
            {toastMsg}
          </span>
        </div>
      </div>
      
      <AnimatePresence>
        {showDeleteModal && (
          <div className="glass-modal-root">
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              className="glass-modal-backdrop"
              onClick={closeDeleteModal}
            />

            <motion.div
              initial={{ opacity: 0, scale: 0.9, y: 20 }}
              animate={{ opacity: 1, scale: 1, y: 0 }}
              exit={{ opacity: 0, scale: 0.9, y: 20 }}
              transition={{ type: "spring", damping: 25, stiffness: 300 }}
              className="glass-modal-card"
            >
            <h3 className="glass-modal-title">清空历史</h3>
            <p className="glass-modal-desc">
              确定要删除所有记录吗？<br />此操作无法撤销。
            </p>

            <div className="glass-modal-actions">
              <button className="glass-btn ghost" onClick={closeDeleteModal}>
                取消
              </button>

              <button className="glass-btn danger" onClick={confirmClearAll}>
                确定清空
              </button>
            </div>
            </motion.div>
          </div>
        )}
      </AnimatePresence>
      
      <div 
        className="toolbar"
        onMouseDown={handleToolbarMouseDown}
      >
        {currentView === 'home' ? (
            settings.disable_search === 'true' ? (
                <div className="brand-header no-drag" style={{display:'flex', alignItems:'center', paddingLeft:'5px'}}><span style={{fontWeight:'bold', fontSize:'16px', letterSpacing:'1px', color: isDarkMode ? '#f0f0f0' : '#333'}}>CLIPFLOW</span></div>
            ) : (
                // 搜索框区域阻止冒泡，防止无法输入文字
                <div className="search-bar-container no-drag" style={{ width: '160px' }} onMouseDown={(e) => e.stopPropagation()}>
                    <span className="search-icon"><Icon name="search" size={14} /></span>
                    <input type="text" className="search-input" placeholder="搜索内容..." value={searchText} onChange={(e) => setSearchText(e.target.value)}/>
                </div>
            )
        ) : (
            <div style={{display:'flex', alignItems:'center', gap:'8px'}}>
                {/* 按钮阻止冒泡，否则点按钮也会触发拖动 */}
                <div className="toolbar-btn no-drag" onMouseDown={(e) => e.stopPropagation()} onClick={currentView === 'viewer' ? handleCloseViewer : () => setCurrentView('home')} title="返回"><Icon name="back" size={18} /></div>
                <div className="page-title">{currentView === 'chat' ? '传输助手' : (currentView === 'settings' ? '设置' : (viewingItem?.category === 'image' ? '图片预览' : '文本编辑'))}</div>
            </div>
        )}
        <div style={{ flex: 1 }}></div>
        {currentView === 'home' && (<>
            <motion.div
              className={`toolbar-btn no-drag ${isPinned ? 'active-pin' : ''}`}
              onMouseDown={(e) => e.stopPropagation()}
              onClick={async () => {
                const next = !isPinned;
                setIsPinned(next);
                try {
                  await appWindow.setAlwaysOnTop(next);
                } catch (err) {
                  console.error(err);
                }
              }}
              title={isPinned ? "取消钉住" : "钉住窗口"}
              whileHover={{ scale: 1.1 }}
              whileTap={{ scale: 0.9 }}
            >
              <Icon name="pin" size={16} style={{ transform: isPinned ? 'rotate(-45deg)' : 'none', transition: 'transform 0.2s' }} />
            </motion.div>
            <motion.div
              className="toolbar-btn no-drag"
              onMouseDown={(e) => e.stopPropagation()}
              onClick={() => setCurrentView('settings')}
              title="设置"
              whileHover={{ scale: 1.1 }}
              whileTap={{ scale: 0.9 }}
            >
              <Icon name="settings" size={18} />
            </motion.div>
            <motion.div
              className="toolbar-btn no-drag"
              onMouseDown={(e) => e.stopPropagation()}
              onClick={() => setCurrentView('chat')}
              title="传输助手"
              whileHover={{ scale: 1.1 }}
              whileTap={{ scale: 0.9 }}
            >
              <Icon name="phone" size={18} />
            </motion.div>
            <motion.div
              className="toolbar-btn no-drag"
              onMouseDown={(e) => e.stopPropagation()}
              onClick={handleClearAll}
              title="清空历史"
              whileHover={{ scale: 1.1, rotate: 5 }}
              whileTap={{ scale: 0.9 }}
            >
              <Icon name="trash" size={18} />
            </motion.div>
        </>)}
        <div className="toolbar-btn no-drag close-btn-top" onMouseDown={(e) => e.stopPropagation()} onClick={() => appWindow.hide()}><Icon name="close" size={18} /></div>
      </div>

      {/* --- Filter Bar --- */}
      {currentView === 'home' && (
        <div className="filter-bar">
            <div className="filter-list">
              {CATEGORIES.map((c) => {
                const isActive = activeTab === c.id;
                return (
                  <div
                    key={c.id}
                    onClick={() => handleTabChange(c.id)}
                    className={`filter-chip ${isActive ? 'active' : ''}`}
                    role="button"
                    tabIndex={0}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter' || e.key === ' ') handleTabChange(c.id);
                    }}
                  >
                    {isActive && (
                      <motion.div
                        layoutId="activeTabBackground"
                        className="active-indicator"
                        transition={{ type: 'spring', stiffness: 500, damping: 30 }}
                      />
                    )}
                    <span className="filter-chip-content">
                      {c.id !== 'all' && <Icon name={c.id} size={12} className="filter-chip-icon" />}
                      {c.label}
                    </span>
                  </div>
                );
              })}
            </div>
            <div className="toolbar-btn" onClick={scrollToTop} style={{marginRight: '4px', display: 'flex', alignItems: 'center', justifyContent: 'center'}} title="回到顶部"><Icon name="arrow-up" size={18} /></div>
            <div className="toolbar-btn" onClick={() => forceSync()} style={{marginRight: '4px', display: 'flex', alignItems: 'center', justifyContent: 'center'}} title="强制同步 (修复卡死)"><Icon name="refresh" size={16} /></div>
        </div>
      )}
      <div className="content-area" style={{ position: 'relative' }}>
        <AnimatePresence mode="wait" initial={false}>
          {/* --- 1. Viewer (Keep existing logic) --- */}
          {currentView === 'viewer' && viewingItem && (
            <div
              key="viewer"
              className="viewer-page"
              style={{ position: 'fixed', top: 48, left: 0, right: 0, bottom: 0, height: 'calc(100% - 48px)', zIndex: 50 }}
            >
              {viewingItem.category === 'image' ? (
                <div className="viewer-body image-mode" onWheel={handleWheel} onMouseDown={handleMouseDown} onMouseMove={handleMouseMove} onMouseUp={handleMouseUp} onMouseLeave={handleMouseUp} style={{ cursor: isDragging ? 'grabbing' : 'grab' }}>
                  <img src={viewingItem.content} style={{ transform: `translate(${pan.x}px, ${pan.y}px) scale(${imgScale})`, transition: isDragging ? 'none' : 'transform 0.1s' }} draggable={false} />
                  <div className="viewer-tip">滚轮缩放 / 拖拽移动</div>
                </div>
              ) : (
                <div className="viewer-body text-mode"><textarea ref={textareaRef} className="viewer-textarea" value={viewerContent} onChange={e => setViewerContent(e.target.value)} spellCheck={false} placeholder="在这里编辑文本..."></textarea></div>
              )}
              <div className="viewer-footer">
                <span className="viewer-info">{viewingItem.category === 'image' ? `${Math.round(imgScale * 100)}%` : `${viewerContent.length} 字符`}</span><div style={{flex:1}}></div>
                {viewingItem.category !== 'image' && (<>
                  <button className={`viewer-btn ${translations[viewingItem.id] ? 'active-translate' : ''}`} onClick={handleViewerTranslateToggle} disabled={isTranslating} style={{display:'flex', alignItems:'center', gap:'4px'}} title="全文翻译/还原"><Icon name="globe" size={14} /> {translations[viewingItem.id] ? "还原" : "翻译"}</button>
                  <button className={`viewer-btn ${selectionRestore ? 'active-translate' : ''}`} onClick={handleTranslateSelection} disabled={isTranslating} style={{display:'flex', alignItems:'center', gap:'4px'}} title={selectionRestore ? "还原选中翻译" : "翻译选中的文本"}><Icon name={selectionRestore ? "ban" : "translate-selection"} size={14} /> {selectionRestore ? "取消" : "选中"}</button>
                </>)}
                {viewingItem.category !== 'image' ? ( <button className="viewer-btn primary" onClick={handleSaveViewerContent}>确定</button> ) : ( <button className="viewer-btn" onClick={() => handleCopy(viewingItem)}>仅复制</button> )}
              </div>
            </div>
          )}
        </AnimatePresence>

        {currentView === 'chat' && (
          <div key="chat" className="phone-connect-panel">
            {!isConnected ? (
              <>
                <div className="qr-wrapper"><QRCodeSVG value={`http://${ipAddress}:${settings.server_port || 19527}`} size={130} /></div>
                <h3>扫码进入聊天</h3>
                <p>确保在同一 Wi-Fi 下</p>
                <div className="ip-info">{ipAddress}:{settings.server_port || 19527}</div>
              </>
            ) : (
              <div className="chat-window">
                <div className="chat-header">🟢 已连接 <span className="disconnect-btn" onClick={() => {setIsConnected(false); setChatMessages([])}}><Icon name="refresh" size={16} /></span></div>
                <div className="chat-messages">{chatMessages.map((msg, idx) => (<div key={idx} className={`chat-bubble-row ${msg.isMe ? 'me' : 'other'}`}><div className="chat-bubble">{msg.text.startsWith('data:image') ? <img src={msg.text} className="chat-img"/> : msg.text}</div></div>))}<div ref={chatEndRef} /></div>
                <div className="chat-input-bar"><div className="chat-icon-btn" onClick={() => fileInputRef.current?.click()}><Icon name="plus-img" size={20} /></div><input type="file" accept="image/*" style={{display: 'none'}} ref={fileInputRef} onChange={handleImageUpload} /><input type="text" className="chat-input" value={chatInput} onChange={(e) => setChatInput(e.target.value)} onKeyDown={handleChatKeyDown} onPaste={handleChatPaste} /><button className="chat-send-btn" onClick={handlePcSend}>发送</button></div>
              </div>
            )}
          </div>
        )}

        {currentView === 'settings' && (
          <div key="settings" className="settings-page" style={{ animation: 'slideIn 0.2s ease' }}>
            <div className="settings-group">
                  <div className="settings-title">常规</div>
                  <div className="settings-item"><div className="item-label">隐藏搜索栏</div><label className="toggle-switch"><input type="checkbox" checked={settings.disable_search === 'true'} onChange={(e) => updateSetting('disable_search', e.target.checked ? 'true' : 'false')} /><span className="slider"></span></label></div>
                  <div className="settings-item"><div className="item-label">跟随鼠标指针呼出<div className="item-desc">按 Alt+V 呼出时移动到鼠标位置</div></div><label className="toggle-switch"><input type="checkbox" checked={settings.follow_mouse === 'true'} onChange={(e) => updateSetting('follow_mouse', e.target.checked ? 'true' : 'false')} /><span className="slider"></span></label></div>
                  <div className="settings-item"><div className="item-label">固定模式自动粘贴</div><label className="toggle-switch"><input type="checkbox" checked={settings.auto_paste_pinned === 'true'} onChange={(e) => updateSetting('auto_paste_pinned', e.target.checked ? 'true' : 'false')} /><span className="slider"></span></label></div>
                  <div className="settings-item"><div className="item-label">未固定时复制不隐藏</div><label className="toggle-switch"><input type="checkbox" checked={settings.stay_on_copy === 'true'} onChange={(e) => updateSetting('stay_on_copy', e.target.checked ? 'true' : 'false')} /><span className="slider"></span></label></div>
                  <div className="settings-item"><div className="item-label">开机自启</div><label className="toggle-switch"><input type="checkbox" checked={autoStart} onChange={toggleAutoStart} /><span className="slider"></span></label></div>
                  <div className="settings-item">
                        <div className="item-label">队列模式
                            <div className="item-desc">按顺序粘贴多条记录</div>
                        </div>
                        <div style={{display:'flex', alignItems:'center', gap:'12px'}}>
                            <input 
                                type="text" 
                                className="settings-input shortcut-input" 
                                style={{width: '80px', textAlign: 'center'}} 
                                value={settings.queue_toggle_shortcut || 'Alt+1'} 
                                onChange={(e) => updateSetting('queue_toggle_shortcut', e.target.value)}
                                title="快捷键切换队列模式"
                            />
                            <label className="toggle-switch">
                              <input type="checkbox" checked={isQueueMode} onChange={() => {
                                setIsQueueMode(prev => {
                                  const newMode = !prev;
                                  if (!newMode) {
                                    setQueueIds([]);
                                    invoke("set_queue", { ids: [] });
                                  }
                                  showToast(newMode ? '队列模式已开启' : '队列模式已关闭');
                                  return newMode;
                                });
                              }} />
                              <span className="slider"></span>
                          </label>
                      </div>
                  </div>
            </div>
            <div className="settings-group">
                  <div className="settings-title">存储与隐私</div>
                  <div className="settings-item"><div className="item-label">隐私模式 (不记录)</div><label className="toggle-switch"><input type="checkbox" checked={settings.privacy_mode === 'true'} onChange={(e) => updateSetting('privacy_mode', e.target.checked ? 'true' : 'false')} /><span className="slider"></span></label></div>
                  <div className="settings-item"><div className="item-label">历史记录上限</div><select className="settings-select" value={settings.history_limit || '200'} onChange={(e) => updateSetting('history_limit', e.target.value)}><option value="50">50 条</option><option value="200">200 条</option><option value="0">无限制</option></select></div>
                  <div className="settings-item danger" onClick={handleClearAll}><div className="item-label">清空所有数据</div><span className="item-arrow"><Icon name="trash" size={14} /></span></div>
            </div>
            <div className="settings-group">
                  <div className="settings-title">文件接收</div>
                  <div className="settings-item no-drag" onClick={handlePickFolder}>
                    <div className="item-label" style={{display: 'flex', alignItems: 'center', gap: '8px'}}><Folder size={16} /> 文件接收位置</div>
                    <div style={{display:'flex', alignItems:'center', gap:'8px'}}>
                          <span className="settings-value" style={{fontSize: '12px', color: '#666', maxWidth: '150px', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap'}}>
                              {fileSavePath || '默认下载文件夹'}
                          </span>
                          <span className="item-arrow"><Icon name="edit" size={14} /></span>
                      </div>
                  </div>
            </div>
            <div className="settings-group">
                  <div className="settings-title">高级 (需重启生效)</div>
                  <div className="settings-item"><div className="item-label">服务端口</div><input type="number" className="settings-input" value={settings.server_port || '19527'} onChange={(e) => updateSetting('server_port', e.target.value)} /></div>
                  <div className="settings-item"><div className="item-label">唤醒快捷键</div><input type="text" className="settings-input" style={{width: '80px'}} value={settings.shortcut || 'Alt+V'} onChange={(e) => updateSetting('shortcut', e.target.value)} /></div>
            </div>
            <div className="settings-footer">ClipFlow v3.1.1 修复版3<br/>by Alex</div>
          </div>
        )}

        {currentView === 'home' && (
          <div
            className="list-wrapper"
            ref={listRef}
            onScroll={(e) => (scrollPosRef.current = e.currentTarget.scrollTop)}
            style={{
              ...listStyle,
              height: '100%',
              overflowY: 'auto',
              paddingBottom: '20px'
            }}
          >
            {filteredHistory.length === 0 ? (
              <div className="empty-state"><Icon name="text" size={48} style={{opacity:0.3, marginBottom:10}} /><div>{searchText ? "无匹配" : "无记录"}</div></div>
            ) : (
              <div className="cards-container" key={renderTab}>
                <AnimatePresence initial={false} mode="popLayout">
                  {renderedHistory.map((item) => {
                    const isTranslated = !!translations[item.id];
                    const contentToShow = isTranslated ? translations[item.id]! : item.content;
                    const displayText = getTruncatedText(contentToShow);
                    const queueIndex = queueIds.indexOf(item.id);

                    return (
                      <motion.div
                        key={item.id}
                        layout
                        layoutId={`card-${item.id}`}
                        initial={{ opacity: 0, scale: 0.9, height: 0 }}
                        animate={{ opacity: 1, scale: 1, height: 'auto' }}
                        exit={{
                          opacity: 0,
                          scale: 0.5,
                          height: 0,
                          transition: { duration: 0.2 }
                        }}
                        transition={{ type: "spring", stiffness: 400, damping: 30 }}
                        whileHover={{ scale: 1.005 }}
                        whileTap={{ scale: 0.98 }}
                        className="card"
                        onClick={() => handleCopy(item)}
                      >
                        {isQueueMode && queueIndex !== -1 && (
                          <div className="queue-index-badge">
                            {queueIndex + 1}
                          </div>
                        )}
                        <div className="card-header">
                          {editingId === item.id ? (
                            <input ref={inputRef} type="text" className="category-input" value={editValue} onChange={(e) => setEditValue(e.target.value)} onBlur={() => handleSaveEdit(item.id)} onKeyDown={(e) => handleKeyDown(e, item.id)} onClick={(e) => e.stopPropagation()}/>
                          ) : (
                            <span className={`source-tag ${['text', 'image', 'code', 'custom'].includes(item.category) ? 'tag-' + item.category : ''}`} onClick={(e) => toggleCategory(e, item)} style={{display:'flex', alignItems:'center', gap:'4px'}}><Icon name={item.category} size={12} /> {item.category.toUpperCase()}</span>
                          )}
                          <div style={{ flex: 1 }}></div>
                          <span className="time-tag">{formatTime(item.created_at)}</span>
                        </div>
                        <div className="card-body">{item.content.startsWith("data:image") ? <div className="item-image-container"><img src={item.content} className="clipboard-img" /></div> : <div className="item-text">{displayText}</div>}</div>
                        <div className="card-actions">
                          <button className="action-btn view-btn" onClick={(e) => handleOpenViewer(e, item)} style={{display:'flex', alignItems:'center', gap:'4px'}}><Icon name="eye" size={14}/> 预览</button><div style={{flex:1}}></div>
                          {item.category !== 'image' && ( <button className={`action-btn translate-btn ${isTranslated ? 'active-translate' : ''}`} onClick={(e) => handleToggleTranslate(e, item)} title={isTranslated ? "还原原文" : "翻译"}> <Icon name="globe" size={14} /> </button> )}
                          <button className="action-btn edit-btn" onClick={(e) => handleStartEdit(e, item)} style={{display:'flex', alignItems:'center'}}><Icon name="edit" size={14} /></button>
                          <motion.button
                            className="action-btn delete-btn"
                            onClick={(e) => handleDelete(e, item.id)}
                            style={{display:'flex', alignItems:'center'}}
                            whileHover={{ scale: 1.1, color: '#ef4444' }}
                            whileTap={{ scale: 0.9 }}
                          >
                            <Icon name="trash" size={14} />
                          </motion.button>
                        </div>
                      </motion.div>
                    );
                  })}
                </AnimatePresence>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

export default App;
