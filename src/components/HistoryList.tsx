import { useRef, useLayoutEffect, useEffect } from "react";
import { RefreshCw } from "lucide-react";
import { Icon } from "../utils/Icon";
import { HistoryItem } from "../types";
import { HistoryCard } from "./HistoryCard";

interface HistoryListProps {
  filteredHistory: HistoryItem[];
  renderedHistory: HistoryItem[];
  renderTab: string;
  listStyle: { opacity: number; transform: string; transition: string };
  searchText: string;
  currentView: string;
  // Queue state
  isQueueMode: boolean;
  queueIds: number[];
  // Edit state
  editingId: number | null;
  editValue: string;
  setEditValue: React.Dispatch<React.SetStateAction<string>>;
  inputRef: React.RefObject<HTMLInputElement | null>;
  // Translations
  translations: Record<number, string>;
  // Pull refresh
  isRefreshing: boolean;
  pullIndicatorRef: React.RefObject<HTMLDivElement | null>;
  pullIconRef: React.RefObject<HTMLDivElement | null>;
  pullTextRef: React.RefObject<HTMLSpanElement | null>;
  handlePullMouseDown: (e: React.MouseEvent, listRef: React.RefObject<HTMLDivElement | null>) => void;
  handlePullMouseMove: (e: React.MouseEvent, listRef: React.RefObject<HTMLDivElement | null>) => void;
  handlePullMouseUp: () => void;
  handlePullMouseLeave: () => void;
  // Card handlers
  handleCopy: (item: HistoryItem) => void;
  handleDelete: (e: React.MouseEvent, id: number) => void;
  handleOpenViewer: (e: React.MouseEvent, item: HistoryItem) => void;
  handleToggleTranslate: (e: React.MouseEvent, item: HistoryItem) => void;
  handleStartEdit: (e: React.MouseEvent, item: HistoryItem) => void;
  handleSaveEdit: (id: number) => void;
  handleKeyDown: (e: React.KeyboardEvent, id: number) => void;
  toggleCategory: (e: React.MouseEvent, item: HistoryItem) => void;
  // Scroll-to-top registration
  onRegisterScrollToTop?: (fn: () => void) => void;
}

export function HistoryList({
  filteredHistory, renderedHistory, renderTab, listStyle, searchText, currentView,
  isQueueMode, queueIds, editingId, editValue, setEditValue, inputRef, translations,
  isRefreshing, pullIndicatorRef, pullIconRef, pullTextRef,
  handlePullMouseDown, handlePullMouseMove, handlePullMouseUp, handlePullMouseLeave,
  handleCopy, handleDelete, handleOpenViewer, handleToggleTranslate,
  handleStartEdit, handleSaveEdit, handleKeyDown, toggleCategory,
  onRegisterScrollToTop,
}: HistoryListProps) {
  const listRef = useRef<HTMLDivElement>(null);
  const scrollPosRef = useRef(0);

  useLayoutEffect(() => {
    if (currentView === 'home' && listRef.current) {
      listRef.current.scrollTop = scrollPosRef.current;
    }
  }, [currentView]);

  useEffect(() => {
    if (onRegisterScrollToTop) {
      onRegisterScrollToTop(() => {
        if (listRef.current) {
          listRef.current.scrollTo({ top: 0, behavior: 'smooth' });
          scrollPosRef.current = 0;
        }
      });
    }
  }, [onRegisterScrollToTop]);

  return (
    <div
      className="list-wrapper"
      ref={listRef}
      onScroll={(e) => (scrollPosRef.current = e.currentTarget.scrollTop)}
      onMouseDown={(e) => handlePullMouseDown(e, listRef)}
      onMouseMove={(e) => handlePullMouseMove(e, listRef)}
      onMouseUp={handlePullMouseUp}
      onMouseLeave={handlePullMouseLeave}
      style={{
        ...listStyle,
        height: '100%',
        overflowY: 'auto',
        paddingBottom: '20px'
      }}
    >
      {/* Pull-to-refresh indicator */}
      <div
        ref={pullIndicatorRef}
        className="pull-refresh-indicator"
        style={{ height: 0, opacity: 0 }}
      >
        <div ref={pullIconRef} style={{ display: 'flex', alignItems: 'center' }}>
          <RefreshCw size={18} className={isRefreshing ? 'spin' : ''} />
        </div>
        <span ref={pullTextRef} style={{ fontSize: 12, marginLeft: 6 }}>
          {isRefreshing ? '刷新中...' : '下拉刷新'}
        </span>
      </div>
      {filteredHistory.length === 0 ? (
        <div className="empty-state"><Icon name="text" size={48} style={{opacity:0.3, marginBottom:10}} /><div>{searchText ? "无匹配" : "无记录"}</div></div>
      ) : (
        <div className="cards-container" key={renderTab}>
            {renderedHistory.map((item) => (
              <HistoryCard
                key={item.id}
                item={item}
                isQueueMode={isQueueMode}
                queueIndex={queueIds.indexOf(item.id)}
                editingId={editingId}
                editValue={editValue}
                setEditValue={setEditValue}
                inputRef={inputRef}
                translations={translations}
                handleCopy={handleCopy}
                handleDelete={handleDelete}
                handleOpenViewer={handleOpenViewer}
                handleToggleTranslate={handleToggleTranslate}
                handleStartEdit={handleStartEdit}
                handleSaveEdit={handleSaveEdit}
                handleKeyDown={handleKeyDown}
                toggleCategory={toggleCategory}
              />
            ))}
        </div>
      )}
    </div>
  );
}
