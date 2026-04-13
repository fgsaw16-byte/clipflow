export interface HistoryItem {
  id: number;
  content: string;
  created_at: string;
  category: string;
}

export interface ChatMessage {
  text: string;
  isMe: boolean;
}

export type ViewType = 'home' | 'chat' | 'settings' | 'viewer';
