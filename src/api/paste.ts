import { invoke } from "@tauri-apps/api/core";

export async function smartCopy(id: number): Promise<void> {
  await invoke("smart_copy", { id });
}

export async function pasteItem(id: number): Promise<void> {
  await invoke("paste_item", { id });
}

export async function setQueue(ids: number[]): Promise<void> {
  await invoke("set_queue", { ids });
}

export async function pasteQueueNext(): Promise<void> {
  await invoke("paste_queue_next");
}
