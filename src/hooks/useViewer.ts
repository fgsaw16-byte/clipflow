import { useState } from "react";
import { HistoryItem } from "../types";
import { ViewType } from "../types";
import { updateHistoryContent } from "../api/history";

interface UseViewerParams {
  viewingItem: HistoryItem | null;
  setViewingItem: React.Dispatch<React.SetStateAction<HistoryItem | null>>;
  setCurrentView: React.Dispatch<React.SetStateAction<ViewType>>;
  translations: Record<number, string>;
  setSelectionRestore: React.Dispatch<React.SetStateAction<{start: number; translatedLength: number; originalText: string;} | null>>;
  loadHistory: () => Promise<void>;
  setTranslations: React.Dispatch<React.SetStateAction<Record<number, string>>>;
  viewerContent: string;
  setViewerContent: React.Dispatch<React.SetStateAction<string>>;
}

export function useViewer({
  viewingItem, setViewingItem, setCurrentView, translations, setSelectionRestore, loadHistory, setTranslations,
  viewerContent, setViewerContent,
}: UseViewerParams) {
  const [imgScale, setImgScale] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [isDragging, setIsDragging] = useState(false);

  const handleOpenViewer = (e: React.MouseEvent, item: HistoryItem) => {
    e.stopPropagation();
    setViewingItem(item);
    const currentContent = translations[item.id] || item.content;
    setViewerContent(currentContent);
    setImgScale(1);
    setPan({ x: 0, y: 0 });
    setSelectionRestore(null);
    setCurrentView('viewer');
  };

  const handleCloseViewer = () => {
    setCurrentView('home');
    setViewingItem(null);
    setSelectionRestore(null);
  };

  const handleSaveViewerContent = async () => {
    if (!viewingItem) return;
    await updateHistoryContent(viewingItem.id, viewerContent);
    if (translations[viewingItem.id]) {
      const newTrans = { ...translations };
      delete newTrans[viewingItem.id];
      setTranslations(newTrans);
    }
    setCurrentView('home');
    loadHistory();
  };

  const handleWheel = (e: React.WheelEvent) => {
    if (viewingItem?.category === 'image') {
      setImgScale(prev => Math.max(0.1, prev + (e.deltaY > 0 ? -0.1 : 0.1)));
    }
  };

  const handleMouseDown = (e: React.MouseEvent) => {
    if (viewingItem?.category !== 'image') return;
    e.preventDefault();
    setIsDragging(true);
  };

  const handleMouseMove = (e: React.MouseEvent) => {
    if (!isDragging) return;
    setPan(prev => ({ x: prev.x + e.movementX, y: prev.y + e.movementY }));
  };

  const handleMouseUp = () => setIsDragging(false);

  return {
    imgScale,
    pan,
    isDragging,
    handleOpenViewer,
    handleCloseViewer,
    handleSaveViewerContent,
    handleWheel,
    handleMouseDown,
    handleMouseMove,
    handleMouseUp,
  };
}
