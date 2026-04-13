interface ToastProps {
  toastMsg: string;
  isDarkMode: boolean;
}

export function Toast({ toastMsg, isDarkMode }: ToastProps) {
  return (
    <div
      style={{
        position: 'fixed',
        bottom: '48px',
        left: '50%',
        transform: toastMsg ? 'translateX(-50%) translateY(0)' : 'translateX(-50%) translateY(16px)',
        zIndex: 100,
        transition: 'all 300ms',
        opacity: toastMsg ? 1 : 0,
        pointerEvents: toastMsg ? 'auto' : 'none',
      }}
    >
      <div
        style={{
          display: 'flex',
          width: 'fit-content',
          alignItems: 'center',
          justifyContent: 'center',
          borderRadius: '9999px',
          border: '1px solid rgba(255, 255, 255, 0.2)',
          backgroundColor: isDarkMode ? 'rgba(31, 41, 55, 0.8)' : 'rgba(255, 255, 255, 0.8)',
          padding: '8px 24px',
          boxShadow: '0 12px 32px rgba(0,0,0,0.18)',
          backdropFilter: 'blur(12px)',
          WebkitBackdropFilter: 'blur(12px)',
        }}
      >
        <span style={{ fontSize: '13px', fontWeight: 600, color: isDarkMode ? '#e5e7eb' : '#374151' }}>
          {toastMsg}
        </span>
      </div>
    </div>
  );
}
