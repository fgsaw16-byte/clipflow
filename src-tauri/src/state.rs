use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{atomic::AtomicBool, Arc, Mutex};
use tokio::sync::broadcast;

pub const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Safely acquire a Mutex, recovering from poisoning.
/// If the Mutex was poisoned by a panicked thread, we still get the inner data.
pub fn safe_lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| {
        eprintln!("[clipflow] Recovered from poisoned Mutex");
        poisoned.into_inner()
    })
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HistoryItem {
    pub id: i64,
    pub content: String,
    pub created_at: String,
    pub category: String,
}

#[derive(Clone)]
pub struct AppState {
    pub(crate) last_content: Arc<Mutex<String>>,
    pub(crate) ignore_signature: Arc<Mutex<Option<String>>>,
    pub(crate) skip_monitor: Arc<Mutex<bool>>,
    pub(crate) last_clipboard_write_ms: Arc<Mutex<i64>>,
    pub(crate) last_image_upload_hash: Arc<Mutex<u64>>,
    pub(crate) last_image_upload_ms: Arc<Mutex<i64>>,
    pub(crate) paste_queue: Arc<Mutex<Vec<i64>>>,
    pub(crate) is_internal_pasting: Arc<Mutex<bool>>,
    pub(crate) file_save_path: Arc<Mutex<PathBuf>>,
    pub(crate) event_tx: broadcast::Sender<String>,
    /// true = monitor thread is alive and listening; false = crashed/not started
    pub(crate) monitor_alive: Arc<AtomicBool>,
    /// Stores the HWND of the current clipboard listener message window (as isize).
    /// Used by restart_clipboard_monitor to send WM_APP and trigger a clean restart.
    #[cfg(target_os = "windows")]
    pub(crate) monitor_hwnd: Arc<Mutex<isize>>,
    #[cfg(target_os = "windows")]
    pub(crate) last_external_handle: Arc<Mutex<isize>>,
    /// Signatures of content recently deleted by the user.
    /// Prevents force_sync from re-inserting clipboard content that the user just deleted.
    pub(crate) recently_deleted_sigs: Arc<Mutex<HashSet<String>>>,
}
