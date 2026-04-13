import { invoke } from "@tauri-apps/api/core";
import { HistoryItem } from "../types";

export async function forceSync(): Promise<HistoryItem[]> {
  return invoke<HistoryItem[]>("force_sync");
}

export async function sendToPhone(content: string): Promise<void> {
  await invoke("send_to_phone", { content });
}

export async function translateText(content: string): Promise<string> {
  return invoke<string>("translate_text", { content });
}

export async function getLocalIp(): Promise<string> {
  return invoke<string>("get_local_ip");
}
