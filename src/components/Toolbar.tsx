import { motion } from "framer-motion";
import { Icon } from "../utils/Icon";
import { ViewType } from "../types";
import { appWindow } from "../lib/window";

interface ToolbarProps {
  currentView: ViewType;
  setCurrentView: React.Dispatch<React.SetStateAction<ViewType>>;
  isPinned: boolean;
  setIsPinned: React.Dispatch<React.SetStateAction<boolean>>;
  isDarkMode: boolean;
  searchText: string;
  setSearchText: React.Dispatch<React.SetStateAction<string>>;
  settingsDisableSearch: string;
  handleCloseViewer: () => void;
  handleClearAll: () => void;
  handleToolbarMouseDown: (e: React.MouseEvent) => void;
  viewingItem: { category: string } | null;
}

export function Toolbar({
  currentView, setCurrentView, isPinned, setIsPinned, isDarkMode,
  searchText, setSearchText, settingsDisableSearch,
  handleCloseViewer, handleClearAll, handleToolbarMouseDown, viewingItem,
}: ToolbarProps) {
  return (
    <div
      className="toolbar"
      onMouseDown={handleToolbarMouseDown}
    >
      {currentView === 'home' ? (
          settingsDisableSearch === 'true' ? (
              <div className="brand-header no-drag" style={{display:'flex', alignItems:'center', paddingLeft:'5px'}}><span style={{fontWeight:'bold', fontSize:'16px', letterSpacing:'1px', color: isDarkMode ? '#f0f0f0' : '#333'}}>CLIPFLOW</span></div>
          ) : (
              <div className="search-bar-container no-drag" style={{ width: '160px' }} onMouseDown={(e) => e.stopPropagation()}>
                  <span className="search-icon"><Icon name="search" size={14} /></span>
                  <input type="text" className="search-input" placeholder="搜索内容..." value={searchText} onChange={(e) => setSearchText(e.target.value)}/>
              </div>
          )
      ) : (
          <div style={{display:'flex', alignItems:'center', gap:'8px'}}>
              <div className="toolbar-btn no-drag" onMouseDown={(e) => e.stopPropagation()} onClick={currentView === 'viewer' ? handleCloseViewer : () => setCurrentView('home')} title="返回"><Icon name="back" size={18} /></div>
              <div className="page-title">{currentView === 'chat' ? '传输助手' : (currentView === 'settings' ? '设置' : (viewingItem?.category === 'image' ? '图片预览' : '文本编辑'))}</div>
          </div>
      )}
      <div style={{ flex: 1 }}></div>
      {currentView === 'home' && (<>
          <motion.div
            className={`toolbar-btn no-drag ${isPinned ? 'active-pin' : ''}`}
            onMouseDown={(e) => e.stopPropagation()}
            onClick={async () => {
              const next = !isPinned;
              setIsPinned(next);
              try {
                await appWindow.setAlwaysOnTop(next);
              } catch (err) {
                console.error(err);
              }
            }}
            title={isPinned ? "取消钉住" : "钉住窗口"}
            whileHover={{ scale: 1.1 }}
            whileTap={{ scale: 0.9 }}
          >
            <Icon name="pin" size={16} style={{ transform: isPinned ? 'rotate(-45deg)' : 'none', transition: 'transform 0.2s' }} />
          </motion.div>
          <motion.div
            className="toolbar-btn no-drag"
            onMouseDown={(e) => e.stopPropagation()}
            onClick={() => setCurrentView('settings')}
            title="设置"
            whileHover={{ scale: 1.1 }}
            whileTap={{ scale: 0.9 }}
          >
            <Icon name="settings" size={18} />
          </motion.div>
          <motion.div
            className="toolbar-btn no-drag"
            onMouseDown={(e) => e.stopPropagation()}
            onClick={() => setCurrentView('chat')}
            title="传输助手"
            whileHover={{ scale: 1.1 }}
            whileTap={{ scale: 0.9 }}
          >
            <Icon name="phone" size={18} />
          </motion.div>
          <motion.div
            className="toolbar-btn no-drag"
            onMouseDown={(e) => e.stopPropagation()}
            onClick={handleClearAll}
            title="清空历史"
            whileHover={{ scale: 1.1, rotate: 5 }}
            whileTap={{ scale: 0.9 }}
          >
            <Icon name="trash" size={18} />
          </motion.div>
      </>)}
      <div className="toolbar-btn no-drag close-btn-top" onMouseDown={(e) => e.stopPropagation()} onClick={() => appWindow.hide()}><Icon name="close" size={18} /></div>
    </div>
  );
}
