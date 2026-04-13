import { invoke } from "@tauri-apps/api/core";

export async function getAllSettings(): Promise<any> {
  return invoke<any>("get_all_settings");
}

export async function saveSetting(key: string, value: string): Promise<void> {
  await invoke("save_setting", { key, value });
}

export async function getFileSavePath(): Promise<string> {
  return invoke<string>("get_file_save_path");
}

export async function setSavePath(path: string): Promise<void> {
  await invoke("set_save_path", { path });
}
