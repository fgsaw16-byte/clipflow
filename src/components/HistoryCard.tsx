import { motion } from "framer-motion";
import { Icon } from "../utils/Icon";
import { getTruncatedText, formatTime } from "../utils/format";
import { HistoryItem } from "../types";

interface HistoryCardProps {
  item: HistoryItem;
  isQueueMode: boolean;
  queueIndex: number;
  editingId: number | null;
  editValue: string;
  setEditValue: React.Dispatch<React.SetStateAction<string>>;
  inputRef: React.RefObject<HTMLInputElement | null>;
  translations: Record<number, string>;
  handleCopy: (item: HistoryItem) => void;
  handleDelete: (e: React.MouseEvent, id: number) => void;
  handleOpenViewer: (e: React.MouseEvent, item: HistoryItem) => void;
  handleToggleTranslate: (e: React.MouseEvent, item: HistoryItem) => void;
  handleStartEdit: (e: React.MouseEvent, item: HistoryItem) => void;
  handleSaveEdit: (id: number) => void;
  handleKeyDown: (e: React.KeyboardEvent, id: number) => void;
  toggleCategory: (e: React.MouseEvent, item: HistoryItem) => void;
}

export function HistoryCard({
  item, isQueueMode, queueIndex, editingId, editValue, setEditValue, inputRef,
  translations, handleCopy, handleDelete, handleOpenViewer, handleToggleTranslate,
  handleStartEdit, handleSaveEdit, handleKeyDown, toggleCategory,
}: HistoryCardProps) {
  const isTranslated = !!translations[item.id];
  const contentToShow = isTranslated ? translations[item.id]! : item.content;
  const displayText = getTruncatedText(contentToShow);

  return (
    <div
      className="card"
      onClick={() => handleCopy(item)}
    >
      {isQueueMode && queueIndex !== -1 && (
        <div className="queue-index-badge">
          {queueIndex + 1}
        </div>
      )}
      <div className="card-header">
        {editingId === item.id ? (
          <input ref={inputRef} type="text" className="category-input" value={editValue} onChange={(e) => setEditValue(e.target.value)} onBlur={() => handleSaveEdit(item.id)} onKeyDown={(e) => handleKeyDown(e, item.id)} onClick={(e) => e.stopPropagation()}/>
        ) : (
          <span className={`source-tag ${['text', 'image', 'code', 'custom'].includes(item.category) ? 'tag-' + item.category : ''}`} onClick={(e) => toggleCategory(e, item)} style={{display:'flex', alignItems:'center', gap:'4px'}}><Icon name={item.category} size={12} /> {item.category.toUpperCase()}</span>
        )}
        <div style={{ flex: 1 }}></div>
        <span className="time-tag">{formatTime(item.created_at)}</span>
      </div>
      <div className="card-body">{item.content.startsWith("data:image") ? <div className="item-image-container"><img src={item.content} className="clipboard-img" /></div> : <div className="item-text">{displayText}</div>}</div>
      <div className="card-actions">
        <button className="action-btn view-btn" onClick={(e) => handleOpenViewer(e, item)} style={{display:'flex', alignItems:'center', gap:'4px'}}><Icon name="eye" size={14}/> 预览</button><div style={{flex:1}}></div>
        {item.category !== 'image' && ( <button className={`action-btn translate-btn ${isTranslated ? 'active-translate' : ''}`} onClick={(e) => handleToggleTranslate(e, item)} title={isTranslated ? "还原原文" : "翻译"}> <Icon name="globe" size={14} /> </button> )}
        <button className="action-btn edit-btn" onClick={(e) => handleStartEdit(e, item)} style={{display:'flex', alignItems:'center'}}><Icon name="edit" size={14} /></button>
        <motion.button
          className="action-btn delete-btn"
          onClick={(e) => handleDelete(e, item.id)}
          style={{display:'flex', alignItems:'center'}}
          whileHover={{ scale: 1.1, color: '#ef4444' }}
          whileTap={{ scale: 0.9 }}
        >
          <Icon name="trash" size={14} />
        </motion.button>
      </div>
    </div>
  );
}
