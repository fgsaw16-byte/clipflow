import { motion } from "framer-motion";
import { CATEGORIES } from "../constants";
import { Icon } from "../utils/Icon";

interface FilterBarProps {
  activeTab: string;
  onTabChange: (id: string) => void;
  onScrollToTop: () => void;
}

export function FilterBar({ activeTab, onTabChange, onScrollToTop }: FilterBarProps) {
  return (
    <div className="filter-bar">
      <div className="filter-list">
        {CATEGORIES.map((c) => {
          const isActive = activeTab === c.id;
          return (
            <div
              key={c.id}
              onClick={() => onTabChange(c.id)}
              className={`filter-chip ${isActive ? 'active' : ''}`}
              role="button"
              tabIndex={0}
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') onTabChange(c.id);
              }}
            >
              {isActive && (
                <motion.div
                  layoutId="activeTabBackground"
                  className="active-indicator"
                  transition={{ type: 'spring', stiffness: 500, damping: 30 }}
                />
              )}
              <span className="filter-chip-content">
                {c.id !== 'all' && <Icon name={c.id} size={12} className="filter-chip-icon" />}
                {c.label}
              </span>
            </div>
          );
        })}
      </div>
      <div className="toolbar-btn" onClick={onScrollToTop} style={{marginRight: '4px', display: 'flex', alignItems: 'center', justifyContent: 'center'}} title="回到顶部"><Icon name="arrow-up" size={18} /></div>
    </div>
  );
}
