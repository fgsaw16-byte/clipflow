import { useState } from "react";
import { enable, disable, isEnabled } from "@tauri-apps/plugin-autostart";
import { open } from "@tauri-apps/plugin-dialog";
import { getAllSettings, saveSetting, getFileSavePath, setSavePath } from "../api/settings";

export function useSettings() {
  const [settings, setSettings] = useState<any>({});
  const [autoStart, setAutoStart] = useState(false);
  const [fileSavePath, setFileSavePath] = useState("");
  const [isDarkMode, setIsDarkMode] = useState(false);

  function applyTheme(theme: string) {
    const isDark = theme === 'dark' || (theme === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches);
    if (isDark) document.body.classList.add('dark-mode');
    else document.body.classList.remove('dark-mode');
    setIsDarkMode(isDark);
  }

  async function loadSettings() {
    const s = await getAllSettings();
    setSettings(s);
    applyTheme(s.theme || 'system');
    const path = await getFileSavePath();
    setFileSavePath(path);
  }

  async function updateSetting(key: string, value: string) {
    setSettings((p: any) => ({ ...p, [key]: value }));
    await saveSetting(key, value);
    if (key === 'theme') applyTheme(value);
  }

  async function checkAutoStart() {
    setAutoStart(await isEnabled());
  }

  async function toggleAutoStart() {
    try {
      if (autoStart) { await disable(); setAutoStart(false); }
      else { await enable(); setAutoStart(true); }
    } catch (e) { console.error(e); }
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
        await setSavePath(selected);
      }
    } catch (e) {
      console.error("Failed to pick folder:", e);
    }
  }

  return {
    settings,
    isDarkMode,
    autoStart,
    fileSavePath,
    loadSettings,
    updateSetting,
    checkAutoStart,
    toggleAutoStart,
    applyTheme,
    handlePickFolder,
  };
}
