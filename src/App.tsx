import { useState, useRef, useMemo, useEffect } from "react";
import "./App.css";

import { HistoryItem, ViewType } from "./types";

import { useToast } from "./hooks/useToast";
import { useSettings } from "./hooks/useSettings";
import { useTabAnimation } from "./hooks/useTabAnimation";
import { useTranslations } from "./hooks/useTranslations";
import { useViewer } from "./hooks/useViewer";
import { useClipboardController } from "./hooks/useClipboardController";
import { usePullRefresh } from "./hooks/usePullRefresh";
import { useClipboardEvents } from "./hooks/useClipboardEvents";

import { Toast } from "./components/Toast";
import { DeleteModal } from "./components/DeleteModal";
import { Toolbar } from "./components/Toolbar";
import { HomeView } from "./components/views/HomeView";
import { ViewerView } from "./components/views/ViewerView";
import { ChatView } from "./components/views/ChatView";
import { SettingsView } from "./components/views/SettingsView";

import { appWindow } from "./lib/window";
import { setCategory } from "./api/history";

function App() {
  // ─── Cross-view state ───
  const [currentView, setCurrentView] = useState<ViewType>('home');
  const [viewingItem, setViewingItem] = useState<HistoryItem | null>(null);
  const [isPinned, setIsPinned] = useState(false);
  const [searchText, setSearchText] = useState("");
  const [editingId, setEditingId] = useState<number | null>(null);
  const [editValue, setEditValue] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  // ─── Chat state (owned here, passed to ChatView) ───
  const [isConnected, setIsConnected] = useState(false);
  const [chatMessages, setChatMessages] = useState<{ text: string; isMe: boolean }[]>([]);

  // ─── Shared viewer state (breaks circular dep between useTranslations <-> useViewer) ───
  const [viewerContent, setViewerContent] = useState("");
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // ─── Interaction lock refs ───
  const isPastingRef = useRef(false);
  const isInteractingRef = useRef(false);
  const isDraggingRef = useRef(false);

  // ─── Hooks ───
  const { toastMsg, showToast } = useToast();

  const {
    settings, isDarkMode, autoStart, fileSavePath,
    loadSettings, updateSetting, checkAutoStart, toggleAutoStart, applyTheme, handlePickFolder,
  } = useSettings();

  const { activeTab, renderTab, listStyle, handleTabChange } = useTabAnimation();

  const {
    translations, setTranslations, isTranslating, selectionRestore, setSelectionRestore,
    handleToggleTranslate, handleViewerTranslateToggle, handleTranslateSelection,
  } = useTranslations({ viewingItem, viewerContent, setViewerContent, textareaRef });

  const {
    history, pendingDeleteIds, isQueueMode, queueIds,
    loadHistory, forceSync,
    handleCopy, handleDelete, handleClearAll, confirmClearAll,
    showDeleteModal, closeDeleteModal, setIsQueueMode, setQueueIds, setHistory,
  } = useClipboardController({
    isPastingRef,
    isPinned,
    editingId,
    settingsAutoPastePinned: settings.auto_paste_pinned || '',
    settingsStayOnCopy: settings.stay_on_copy || '',
    settingsQueueToggleShortcut: settings.queue_toggle_shortcut || 'Alt+1',
    showToast,
  });

  const viewer = useViewer({
    viewingItem, setViewingItem, setCurrentView,
    translations, setSelectionRestore, loadHistory, setTranslations,
    viewerContent, setViewerContent,
  });

  const pull = usePullRefresh({ forceSync });

  useClipboardEvents({
    forceSync, loadHistory,
    settingsTheme: settings.theme || '',
    isPinned,
    isInteractingRef, isDraggingRef,
    applyTheme,
    setIsConnected, setChatMessages,
  });

  // ─── Init: load settings and autostart on mount ───
  useEffect(() => { loadSettings(); checkAutoStart(); }, []);

  // ─── Toolbar drag handler ───
  const handleToolbarMouseDown = (e: React.MouseEvent) => {
    const target = e.target as HTMLElement;
    if (target.closest('.no-drag')) return;
    isDraggingRef.current = true;
    appWindow.startDragging()
      .catch((err: unknown) => { console.error(err); })
      .finally(() => { setTimeout(() => { isDraggingRef.current = false; }, 100); });
  };

  // ─── Category editing ───
  const toggleCategory = async (e: React.MouseEvent, item: HistoryItem) => {
    e.stopPropagation();
    const cats = ['text', 'code', 'custom'];
    let next = 'text';
    if (cats.includes(item.category)) { next = cats[(cats.indexOf(item.category) + 1) % cats.length]; }
    if (item.category === 'image') return;
    setHistory(prev => prev.map(h => h.id === item.id ? { ...h, category: next } : h));
    await setCategory(item.id, next);
  };
  const handleStartEdit = (e: React.MouseEvent, item: HistoryItem) => { e.stopPropagation(); setEditingId(item.id); setEditValue(item.category); };
  const handleSaveEdit = async (id: number) => { const val = editValue.trim() || "text"; setHistory(p => p.map(h => h.id === id ? { ...h, category: val } : h)); await setCategory(id, val); setEditingId(null); };
  const handleKeyDown = (e: React.KeyboardEvent, id: number) => { if (e.key === 'Enter') handleSaveEdit(id); else if (e.key === 'Escape') setEditingId(null); };

  // ─── Scroll to top (HistoryList registers its scroll fn via callback) ───
  const scrollToTopRef = useRef<(() => void) | null>(null);
  const onScrollToTop = () => { scrollToTopRef.current?.(); };
  const onRegisterScrollToTop = (fn: () => void) => { scrollToTopRef.current = fn; };

  // ─── Filtered history ───
  const filteredHistory = useMemo(() => {
    return history.filter((item) => {
      if (pendingDeleteIds.has(item.id)) return false;
      if (renderTab === 'custom') { if (['text', 'image', 'code'].includes(item.category)) return false; }
      else if (renderTab !== 'all') { if (item.category !== renderTab) return false; }
      if (!searchText) return true;
      const lowerSearch = searchText.toLowerCase();
      if (item.category === 'image') return false;
      const contentToCheck = translations[item.id] || item.content;
      return contentToCheck.toLowerCase().includes(lowerSearch) || item.category.toLowerCase().includes(lowerSearch);
    });
  }, [history, pendingDeleteIds, renderTab, searchText, translations]);

  const renderedHistory = filteredHistory.slice(0, 50);

  return (
    <div className={`app-container ${!isPinned ? 'unpinned' : ''}`}>
      <Toast toastMsg={toastMsg} isDarkMode={isDarkMode} />

      <DeleteModal show={showDeleteModal} onClose={closeDeleteModal} onConfirm={confirmClearAll} />

      <Toolbar
        currentView={currentView} setCurrentView={setCurrentView}
        isPinned={isPinned} setIsPinned={setIsPinned}
        isDarkMode={isDarkMode}
        searchText={searchText} setSearchText={setSearchText}
        settingsDisableSearch={settings.disable_search || ''}
        handleCloseViewer={viewer.handleCloseViewer}
        handleClearAll={handleClearAll}
        handleToolbarMouseDown={handleToolbarMouseDown}
        viewingItem={viewingItem}
      />

      <div className="content-area" style={{ position: 'relative' }}>
        <ViewerView
          viewingItem={viewingItem} currentView={currentView}
          viewerContent={viewerContent} setViewerContent={setViewerContent}
          imgScale={viewer.imgScale} pan={viewer.pan} isDragging={viewer.isDragging}
          textareaRef={textareaRef}
          isTranslating={isTranslating} translations={translations} selectionRestore={selectionRestore}
          handleWheel={viewer.handleWheel} handleMouseDown={viewer.handleMouseDown}
          handleMouseMove={viewer.handleMouseMove} handleMouseUp={viewer.handleMouseUp}
          handleViewerTranslateToggle={handleViewerTranslateToggle}
          handleTranslateSelection={handleTranslateSelection}
          handleSaveViewerContent={viewer.handleSaveViewerContent}
          handleCopy={handleCopy}
        />

        {currentView === 'chat' && (
          <ChatView
            isConnected={isConnected} setIsConnected={setIsConnected}
            chatMessages={chatMessages} setChatMessages={setChatMessages}
            settings={settings}
          />
        )}

        {currentView === 'settings' && (
          <SettingsView
            settings={settings} updateSetting={updateSetting}
            autoStart={autoStart} toggleAutoStart={toggleAutoStart}
            isQueueMode={isQueueMode} setIsQueueMode={setIsQueueMode}
            setQueueIds={setQueueIds}
            showToast={showToast}
            handleClearAll={handleClearAll}
            fileSavePath={fileSavePath} handlePickFolder={handlePickFolder}
          />
        )}

        {currentView === 'home' && (
          <HomeView
            activeTab={activeTab} onTabChange={handleTabChange} onScrollToTop={onScrollToTop}
            filteredHistory={filteredHistory} renderedHistory={renderedHistory}
            renderTab={renderTab} listStyle={listStyle}
            searchText={searchText} currentView={currentView}
            isQueueMode={isQueueMode} queueIds={queueIds}
            editingId={editingId} editValue={editValue} setEditValue={setEditValue} inputRef={inputRef}
            translations={translations}
            isRefreshing={pull.isRefreshing}
            pullIndicatorRef={pull.pullIndicatorRef} pullIconRef={pull.pullIconRef} pullTextRef={pull.pullTextRef}
            handlePullMouseDown={pull.handlePullMouseDown} handlePullMouseMove={pull.handlePullMouseMove}
            handlePullMouseUp={pull.handlePullMouseUp} handlePullMouseLeave={pull.handlePullMouseLeave}
            handleCopy={handleCopy} handleDelete={handleDelete}
            handleOpenViewer={viewer.handleOpenViewer}
            handleToggleTranslate={handleToggleTranslate}
            handleStartEdit={handleStartEdit} handleSaveEdit={handleSaveEdit}
            handleKeyDown={handleKeyDown} toggleCategory={toggleCategory}
            onRegisterScrollToTop={onRegisterScrollToTop}
          />
        )}
      </div>
    </div>
  );
}

export default App;
