import { invoke } from "@tauri-apps/api/core";
import { HistoryItem } from "../types";

export async function getHistory(): Promise<HistoryItem[]> {
  return invoke<HistoryItem[]>("get_history");
}

export async function deleteItem(id: number): Promise<void> {
  await invoke("delete_item", { id });
}

export async function clearHistory(): Promise<void> {
  await invoke("clear_history");
}

export async function setCategory(id: number, category: string): Promise<void> {
  await invoke("set_category", { id, category });
}

export async function updateHistoryContent(id: number, content: string): Promise<void> {
  await invoke("update_history_content", { id, content });
}
