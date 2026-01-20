use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use quarm_audio::TtsEngine;
use quarm_config::Config;

/// Application state shared across Tauri commands
pub struct AppState {
    /// Application configuration
    pub config: Arc<Mutex<Option<Config>>>,
    /// TTS engine for audio announcements
    pub tts_engine: Arc<Mutex<Option<TtsEngine>>>,
    /// Handle to the log monitor task
    pub monitor_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    /// Flag indicating if monitoring is currently active
    pub is_monitoring: Arc<AtomicBool>,
}

impl AppState {
    /// Creates a new AppState with default values
    pub fn new() -> Self {
        Self {
            config: Arc::new(Mutex::new(None)),
            tts_engine: Arc::new(Mutex::new(None)),
            monitor_handle: Arc::new(Mutex::new(None)),
            is_monitoring: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
