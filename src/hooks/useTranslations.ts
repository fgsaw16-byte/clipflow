import { useState } from "react";
import { translateText } from "../api/sync";
import { HistoryItem } from "../types";

interface UseTranslationsParams {
  viewingItem: HistoryItem | null;
  viewerContent: string;
  setViewerContent: React.Dispatch<React.SetStateAction<string>>;
  textareaRef: React.RefObject<HTMLTextAreaElement | null>;
}

export function useTranslations({ viewingItem, viewerContent, setViewerContent, textareaRef }: UseTranslationsParams) {
  const [translations, setTranslations] = useState<Record<number, string>>({});
  const [isTranslating, setIsTranslating] = useState(false);
  const [selectionRestore, setSelectionRestore] = useState<{start: number; translatedLength: number; originalText: string;} | null>(null);

  const handleToggleTranslate = async (e: React.MouseEvent, item: HistoryItem) => {
    e.stopPropagation();
    if (translations[item.id]) {
      const newTrans = { ...translations };
      delete newTrans[item.id];
      setTranslations(newTrans);
    } else {
      try {
        const res = await translateText(item.content);
        setTranslations(prev => ({ ...prev, [item.id]: res }));
      } catch (err) {
        console.error(err);
      }
    }
  };

  const handleViewerTranslateToggle = async () => {
    if (!viewingItem || isTranslating) return;
    if (translations[viewingItem.id]) {
      const newTrans = { ...translations };
      delete newTrans[viewingItem.id];
      setTranslations(newTrans);
      setViewerContent(viewingItem.content);
    } else {
      setIsTranslating(true);
      try {
        const res = await translateText(viewingItem.content);
        setTranslations(prev => ({ ...prev, [viewingItem.id]: res }));
        setViewerContent(res);
      } catch (err) {
        console.error(err);
      }
      setIsTranslating(false);
    }
  };

  const handleTranslateSelection = async () => {
    if (!textareaRef.current || isTranslating) return;
    if (selectionRestore) {
      const { start, translatedLength, originalText } = selectionRestore;
      const end = start + translatedLength;
      const newContent = viewerContent.substring(0, start) + originalText + viewerContent.substring(end);
      setViewerContent(newContent);
      setSelectionRestore(null);
      return;
    }
    const textarea = textareaRef.current;
    const start = textarea.selectionStart;
    const end = textarea.selectionEnd;
    if (start === end) return;
    const selectedText = viewerContent.substring(start, end);
    setIsTranslating(true);
    try {
      const translatedText = await translateText(selectedText);
      const newContent = viewerContent.substring(0, start) + translatedText + viewerContent.substring(end);
      setViewerContent(newContent);
      setSelectionRestore({ start: start, translatedLength: translatedText.length, originalText: selectedText });
    } catch (err) {
      console.error(err);
    }
    setIsTranslating(false);
  };

  return {
    translations,
    setTranslations,
    isTranslating,
    selectionRestore,
    setSelectionRestore,
    handleToggleTranslate,
    handleViewerTranslateToggle,
    handleTranslateSelection,
  };
}
