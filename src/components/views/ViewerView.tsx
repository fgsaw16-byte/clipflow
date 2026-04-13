import { AnimatePresence } from "framer-motion";
import { Icon } from "../../utils/Icon";
import { HistoryItem } from "../../types";

interface ViewerViewProps {
  viewingItem: HistoryItem | null;
  currentView: string;
  viewerContent: string;
  setViewerContent: React.Dispatch<React.SetStateAction<string>>;
  imgScale: number;
  pan: { x: number; y: number };
  isDragging: boolean;
  textareaRef: React.RefObject<HTMLTextAreaElement | null>;
  isTranslating: boolean;
  translations: Record<number, string>;
  selectionRestore: { start: number; translatedLength: number; originalText: string } | null;
  handleWheel: (e: React.WheelEvent) => void;
  handleMouseDown: (e: React.MouseEvent) => void;
  handleMouseMove: (e: React.MouseEvent) => void;
  handleMouseUp: () => void;
  handleViewerTranslateToggle: () => void;
  handleTranslateSelection: () => void;
  handleSaveViewerContent: () => void;
  handleCopy: (item: HistoryItem) => void;
}

export function ViewerView({
  viewingItem, currentView, viewerContent, setViewerContent,
  imgScale, pan, isDragging, textareaRef, isTranslating,
  translations, selectionRestore,
  handleWheel, handleMouseDown, handleMouseMove, handleMouseUp,
  handleViewerTranslateToggle, handleTranslateSelection,
  handleSaveViewerContent, handleCopy,
}: ViewerViewProps) {
  return (
    <AnimatePresence mode="wait" initial={false}>
      {currentView === 'viewer' && viewingItem && (
        <div
          key="viewer"
          className="viewer-page"
          style={{ position: 'fixed', top: 48, left: 0, right: 0, bottom: 0, height: 'calc(100% - 48px)', zIndex: 50 }}
        >
          {viewingItem.category === 'image' ? (
            <div className="viewer-body image-mode" onWheel={handleWheel} onMouseDown={handleMouseDown} onMouseMove={handleMouseMove} onMouseUp={handleMouseUp} onMouseLeave={handleMouseUp} style={{ cursor: isDragging ? 'grabbing' : 'grab' }}>
              <img src={viewingItem.content} style={{ transform: `translate(${pan.x}px, ${pan.y}px) scale(${imgScale})`, transition: isDragging ? 'none' : 'transform 0.1s' }} draggable={false} />
              <div className="viewer-tip">滚轮缩放 / 拖拽移动</div>
            </div>
          ) : (
            <div className="viewer-body text-mode"><textarea ref={textareaRef} className="viewer-textarea" value={viewerContent} onChange={e => setViewerContent(e.target.value)} spellCheck={false} placeholder="在这里编辑文本..."></textarea></div>
          )}
          <div className="viewer-footer">
            <span className="viewer-info">{viewingItem.category === 'image' ? `${Math.round(imgScale * 100)}%` : `${viewerContent.length} 字符`}</span><div style={{flex:1}}></div>
            {viewingItem.category !== 'image' && (<>
              <button className={`viewer-btn ${translations[viewingItem.id] ? 'active-translate' : ''}`} onClick={handleViewerTranslateToggle} disabled={isTranslating} style={{display:'flex', alignItems:'center', gap:'4px'}} title="全文翻译/还原"><Icon name="globe" size={14} /> {translations[viewingItem.id] ? "还原" : "翻译"}</button>
              <button className={`viewer-btn ${selectionRestore ? 'active-translate' : ''}`} onClick={handleTranslateSelection} disabled={isTranslating} style={{display:'flex', alignItems:'center', gap:'4px'}} title={selectionRestore ? "还原选中翻译" : "翻译选中的文本"}><Icon name={selectionRestore ? "ban" : "translate-selection"} size={14} /> {selectionRestore ? "取消" : "选中"}</button>
            </>)}
            {viewingItem.category !== 'image' ? ( <button className="viewer-btn primary" onClick={handleSaveViewerContent}>确定</button> ) : ( <button className="viewer-btn" onClick={() => handleCopy(viewingItem)}>仅复制</button> )}
          </div>
        </div>
      )}
    </AnimatePresence>
  );
}
