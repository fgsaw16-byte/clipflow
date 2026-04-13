import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { appWindow } from "../lib/window";
import { ChatMessage } from "../types";

interface UseClipboardEventsParams {
  forceSync: () => Promise<void>;
  loadHistory: () => Promise<void>;
  settingsTheme: string;
  isPinned: boolean;
  isInteractingRef: React.MutableRefObject<boolean>;
  isDraggingRef: React.MutableRefObject<boolean>;
  applyTheme: (theme: string) => void;
  setIsConnected: React.Dispatch<React.SetStateAction<boolean>>;
  setChatMessages: React.Dispatch<React.SetStateAction<ChatMessage[]>>;
}

export function useClipboardEvents({
  forceSync,
  loadHistory,
  settingsTheme,
  isPinned,
  isInteractingRef,
  isDraggingRef,
  applyTheme,
  setIsConnected,
  setChatMessages,
}: UseClipboardEventsParams) {
  useEffect(() => {
    const unlistenClip = listen("clipboard-monitor", () => loadHistory());
    const unlistenNew = listen("new_message", () => loadHistory());
    const unlistenMobile = listen("mobile-connected", () => setIsConnected(true));
    const unlistenMsg = listen<string>("mobile-msg", (e) => setChatMessages(p => [...p, { text: e.payload, isMe: false }]));
    const unlistenCrash = listen("listener-crashed", () => {
      console.warn("[clipflow] Listener crashed — triggering force_sync recovery");
      forceSync();
    });

    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    const handleSystemThemeChange = () => {
      if (settingsTheme === 'system' || !settingsTheme) applyTheme('system');
    };
    mediaQuery.addEventListener('change', handleSystemThemeChange);

    const handleGlobalMouseUp = () => {
      isInteractingRef.current = false;
      isDraggingRef.current = false;
    };
    window.addEventListener('mouseup', handleGlobalMouseUp);

    const unlistenFocus = appWindow.onFocusChanged(({ payload: focused }) => {
      if (isDraggingRef.current) return;
      if (focused) {
        forceSync();
      }
      if (!focused && !isPinned && !isInteractingRef.current) {
        setTimeout(() => appWindow.hide(), 50);
      }
    });

    return () => {
      unlistenClip.then(f => f());
      unlistenNew.then(f => f());
      unlistenMobile.then(f => f());
      unlistenMsg.then(f => f());
      unlistenCrash.then(f => f());
      unlistenFocus.then(f => f());
      mediaQuery.removeEventListener('change', handleSystemThemeChange);
      window.removeEventListener('mouseup', handleGlobalMouseUp);
    };
  }, [settingsTheme, isPinned]);
}
