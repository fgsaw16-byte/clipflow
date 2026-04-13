import { Folder } from "lucide-react";
import { Icon } from "../../utils/Icon";
import { setQueue } from "../../api/paste";

interface SettingsViewProps {
  settings: any;
  updateSetting: (key: string, value: string) => void;
  autoStart: boolean;
  toggleAutoStart: () => void;
  isQueueMode: boolean;
  setIsQueueMode: React.Dispatch<React.SetStateAction<boolean>>;
  setQueueIds: React.Dispatch<React.SetStateAction<number[]>>;
  showToast: (msg: string) => void;
  handleClearAll: () => void;
  fileSavePath: string;
  handlePickFolder: () => void;
}

export function SettingsView({
  settings, updateSetting, autoStart, toggleAutoStart,
  isQueueMode, setIsQueueMode, setQueueIds, showToast,
  handleClearAll, fileSavePath, handlePickFolder,
}: SettingsViewProps) {
  return (
    <div key="settings" className="settings-page" style={{ animation: 'slideIn 0.2s ease' }}>
      <div className="settings-group">
            <div className="settings-title">常规</div>
            <div className="settings-item"><div className="item-label">隐藏搜索栏</div><label className="toggle-switch"><input type="checkbox" checked={settings.disable_search === 'true'} onChange={(e) => updateSetting('disable_search', e.target.checked ? 'true' : 'false')} /><span className="slider"></span></label></div>
            <div className="settings-item"><div className="item-label">跟随鼠标指针呼出<div className="item-desc">按 Alt+V 呼出时移动到鼠标位置</div></div><label className="toggle-switch"><input type="checkbox" checked={settings.follow_mouse === 'true'} onChange={(e) => updateSetting('follow_mouse', e.target.checked ? 'true' : 'false')} /><span className="slider"></span></label></div>
            <div className="settings-item"><div className="item-label">固定模式自动粘贴</div><label className="toggle-switch"><input type="checkbox" checked={settings.auto_paste_pinned === 'true'} onChange={(e) => updateSetting('auto_paste_pinned', e.target.checked ? 'true' : 'false')} /><span className="slider"></span></label></div>
            <div className="settings-item"><div className="item-label">未固定时复制不隐藏</div><label className="toggle-switch"><input type="checkbox" checked={settings.stay_on_copy === 'true'} onChange={(e) => updateSetting('stay_on_copy', e.target.checked ? 'true' : 'false')} /><span className="slider"></span></label></div>
            <div className="settings-item"><div className="item-label">开机自启</div><label className="toggle-switch"><input type="checkbox" checked={autoStart} onChange={toggleAutoStart} /><span className="slider"></span></label></div>
            <div className="settings-item"><div className="item-label">主题模式</div><select className="settings-select" value={settings.theme || 'system'} onChange={(e) => updateSetting('theme', e.target.value)}><option value="system">跟随系统</option><option value="light">浅色</option><option value="dark">深色</option></select></div>
            <div className="settings-item">
                  <div className="item-label">队列模式
                      <div className="item-desc">按顺序粘贴多条记录</div>
                  </div>
                  <div style={{display:'flex', alignItems:'center', gap:'12px'}}>
                      <input 
                          type="text" 
                          className="settings-input shortcut-input" 
                          style={{width: '80px', textAlign: 'center'}} 
                          value={settings.queue_toggle_shortcut || 'Alt+1'} 
                          onChange={(e) => updateSetting('queue_toggle_shortcut', e.target.value)}
                          title="快捷键切换队列模式"
                      />
                      <label className="toggle-switch">
                        <input type="checkbox" checked={isQueueMode} onChange={() => {
                          setIsQueueMode(prev => {
                            const newMode = !prev;
                            if (!newMode) {
                              setQueueIds([]);
                              setQueue([]);
                            }
                            showToast(newMode ? '队列模式已开启' : '队列模式已关闭');
                            return newMode;
                          });
                        }} />
                        <span className="slider"></span>
                    </label>
                </div>
            </div>
      </div>
      <div className="settings-group">
            <div className="settings-title">存储与隐私</div>
            <div className="settings-item"><div className="item-label">隐私模式 (不记录)</div><label className="toggle-switch"><input type="checkbox" checked={settings.privacy_mode === 'true'} onChange={(e) => updateSetting('privacy_mode', e.target.checked ? 'true' : 'false')} /><span className="slider"></span></label></div>
            <div className="settings-item"><div className="item-label">历史记录上限</div><select className="settings-select" value={settings.history_limit || '200'} onChange={(e) => updateSetting('history_limit', e.target.value)}><option value="50">50 条</option><option value="200">200 条</option><option value="0">无限制</option></select></div>
            <div className="settings-item danger" onClick={handleClearAll}><div className="item-label">清空所有数据</div><span className="item-arrow"><Icon name="trash" size={14} /></span></div>
      </div>
      <div className="settings-group">
            <div className="settings-title">文件接收</div>
            <div className="settings-item no-drag" onClick={handlePickFolder}>
              <div className="item-label" style={{display: 'flex', alignItems: 'center', gap: '8px'}}><Folder size={16} /> 文件接收位置</div>
              <div style={{display:'flex', alignItems:'center', gap:'8px'}}>
                    <span className="settings-value" style={{fontSize: '12px', color: '#666', maxWidth: '150px', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap'}}>
                        {fileSavePath || '默认下载文件夹'}
                    </span>
                    <span className="item-arrow"><Icon name="edit" size={14} /></span>
                </div>
            </div>
      </div>
      <div className="settings-group">
            <div className="settings-title">高级 (需重启生效)</div>
            <div className="settings-item"><div className="item-label">服务端口</div><input type="number" className="settings-input" value={settings.server_port || '19527'} onChange={(e) => updateSetting('server_port', e.target.value)} /></div>
            <div className="settings-item"><div className="item-label">唤醒快捷键</div><input type="text" className="settings-input" style={{width: '80px'}} value={settings.shortcut || 'Alt+V'} onChange={(e) => updateSetting('shortcut', e.target.value)} /></div>
      </div>
      <div className="settings-footer">ClipFlow v3.1.1 修复版3<br/>by Alex</div>
    </div>
  );
}
