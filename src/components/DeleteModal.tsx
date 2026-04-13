import { AnimatePresence, motion } from "framer-motion";

interface DeleteModalProps {
  show: boolean;
  onClose: () => void;
  onConfirm: () => void;
}

export function DeleteModal({ show, onClose, onConfirm }: DeleteModalProps) {
  return (
    <AnimatePresence>
      {show && (
        <div className="glass-modal-root">
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="glass-modal-backdrop"
            onClick={onClose}
          />

          <motion.div
            initial={{ opacity: 0, scale: 0.9, y: 20 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.9, y: 20 }}
            transition={{ type: "spring", damping: 25, stiffness: 300 }}
            className="glass-modal-card"
          >
          <h3 className="glass-modal-title">清空历史</h3>
          <p className="glass-modal-desc">
            确定要删除所有记录吗？<br />此操作无法撤销。
          </p>

          <div className="glass-modal-actions">
            <button className="glass-btn ghost" onClick={onClose}>
              取消
            </button>

            <button className="glass-btn danger" onClick={onConfirm}>
              确定清空
            </button>
          </div>
          </motion.div>
        </div>
      )}
    </AnimatePresence>
  );
}
