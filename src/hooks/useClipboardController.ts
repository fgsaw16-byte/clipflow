import { useState, useRef, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { HistoryItem } from "../types";
import { getHistory, deleteItem, clearHistory as clearHistoryApi } from "../api/history";
import { smartCopy, pasteItem, setQueue, pasteQueueNext } from "../api/paste";
import { forceSync as forceSyncApi } from "../api/sync";
import { appWindow } from "../lib/window";

interface UseClipboardControllerParams {
  isPastingRef: React.MutableRefObject<boolean>;
  isPinned: boolean;
  editingId: number | null;
  settingsAutoPastePinned: string;
  settingsStayOnCopy: string;
  settingsQueueToggleShortcut: string;
  showToast: (msg: string) => void;
}

export function useClipboardController({
  isPastingRef,
  isPinned,
  editingId,
  settingsAutoPastePinned,
  settingsStayOnCopy,
  settingsQueueToggleShortcut,
  showToast,
}: UseClipboardControllerParams) {
  const [history, setHistory] = useState<HistoryItem[]>([]);
  const [pendingDeleteIds, setPendingDeleteIds] = useState<Set<number>>(new Set());
  const [isQueueMode, setIsQueueMode] = useState(false);
  const [queueIds, setQueueIds] = useState<number[]>([]);
  const [showDeleteModal, setShowDeleteModal] = useState(false);

  const isQueueModeRef = useRef(isQueueMode);
  const queueIdsRef = useRef(queueIds);

  useEffect(() => { isQueueModeRef.current = isQueueMode; }, [isQueueMode]);
  useEffect(() => { queueIdsRef.current = queueIds; }, [queueIds]);

  async function loadHistory() {
    setHistory(await getHistory());
  }

  async function forceSync() {
    try {
      const items = await forceSyncApi();
      setHistory(items);
    } catch (e) {
      console.error("force_sync failed, falling back to get_history:", e);
      try { setHistory(await getHistory()); } catch (_) {}
    }
  }

  async function handleCopy(item: HistoryItem) {
    if (editingId !== null) return;
    if (isPastingRef.current) return;

    if (isQueueMode) {
      const currentQueue = queueIdsRef.current;
      const nextQueue = currentQueue.includes(item.id)
        ? currentQueue.filter((id) => id !== item.id)
        : [...currentQueue, item.id];
      queueIdsRef.current = nextQueue;
      setQueueIds(nextQueue);
      await setQueue(nextQueue);
      return;
    }

    isPastingRef.current = true;

    try {
      if (isPinned && settingsAutoPastePinned === 'true') {
        await pasteItem(item.id);
        isPastingRef.current = false;
      } else {
        await smartCopy(item.id);
        if (!isPinned && settingsStayOnCopy !== 'true') {
          await appWindow.hide();
        }
        isPastingRef.current = false;
      }
    } catch (e) {
      console.error("Copy Failed:", e);
      isPastingRef.current = false;
    }
  }

  async function handleDelete(e: React.MouseEvent, id: number) {
    e.stopPropagation();
    setPendingDeleteIds(prev => new Set(prev).add(id));
    try {
      await deleteItem(id);
    } catch (err) {
      console.error("delete_item failed:", err);
    }
    setHistory(p => p.filter(i => i.id !== id));
    setPendingDeleteIds(prev => { const next = new Set(prev); next.delete(id); return next; });
  }

  async function handleClearAll() { setShowDeleteModal(true); }
  const closeDeleteModal = () => setShowDeleteModal(false);

  async function confirmClearAll() {
    await clearHistoryApi();
    setHistory([]);
    closeDeleteModal();
  }

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setupListener = async () => {
      if (unlisten) unlisten();

      unlisten = await listen('trigger-queue-paste', async () => {
        if (!isQueueModeRef.current) return;
        if (isPastingRef.current) {
          console.log("🚫 Paste Blocked: Too fast / Already pasting");
          return;
        }
        isPastingRef.current = true;
        console.log("✅ Executing Queue Paste...");
        try {
          await pasteQueueNext();
        } catch (err) {
          console.error(err);
        } finally {
          setTimeout(() => { isPastingRef.current = false; }, 300);
        }
      });
    };

    setupListener();
    return () => { if (unlisten) unlisten(); };
  }, []);

  useEffect(() => {
    const unlistenQueue = listen<number>("queue-consumed", () => setQueueIds(prev => {
      const next = prev.slice(1);
      queueIdsRef.current = next;
      return next;
    }));
    return () => { unlistenQueue.then(f => f()); };
  }, []);

  useEffect(() => {
    const shortcut = (settingsQueueToggleShortcut || 'Alt+1').toString();
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
          setQueue([]);
        }
        showToast(newMode ? '队列模式已开启' : '队列模式已关闭');
        return newMode;
      });
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [settingsQueueToggleShortcut]);

  return {
    history,
    pendingDeleteIds,
    isQueueMode,
    queueIds,
    queueIdsRef,
    loadHistory,
    forceSync,
    handleCopy,
    handleDelete,
    handleClearAll,
    confirmClearAll,
    showDeleteModal,
    closeDeleteModal,
    setIsQueueMode,
    setQueueIds,
    setHistory,
  };
}
