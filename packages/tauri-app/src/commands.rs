use std::sync::atomic::Ordering;
use tauri::State;

use quarm_audio::TtsEngine;
use quarm_config::Config;
use quarm_monitor::LogMonitor;

use crate::state::AppState;

/// Load configuration from a JSON file
#[tauri::command]
pub async fn load_config(path: String, state: State<'_, AppState>) -> Result<Config, String> {
    let config = Config::load(&path)
        .await
        .map_err(|e| format!("Failed to load config: {}", e))?;

    // Store in state
    *state.config.lock().await = Some(config.clone());

    Ok(config)
}

/// Save configuration to a JSON file
#[tauri::command]
pub async fn save_config(
    path: String,
    config: Config,
    state: State<'_, AppState>,
) -> Result<(), String> {
    config
        .save(&path)
        .await
        .map_err(|e| format!("Failed to save config: {}", e))?;

    // Update state
    *state.config.lock().await = Some(config);

    Ok(())
}

/// Get the current configuration from state
#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<Config, String> {
    let config_lock = state.config.lock().await;
    config_lock
        .as_ref()
        .cloned()
        .ok_or_else(|| "No configuration loaded".to_string())
}

/// Initialize the TTS engine with a model file
#[tauri::command]
pub async fn init_tts(model_path: String, state: State<'_, AppState>) -> Result<(), String> {
    let engine = TtsEngine::new(&model_path)
        .await
        .map_err(|e| format!("Failed to initialize TTS engine: {}", e))?;

    // Pre-cache announcements if config is loaded
    let mut engine = engine;
    if let Some(config) = state.config.lock().await.as_ref() {
        let announcements: Vec<String> = config
            .messages
            .iter()
            .map(|m| m.announcement().to_string())
            .collect();
        engine
            .precache(announcements)
            .await
            .map_err(|e| format!("Failed to precache announcements: {}", e))?;
    }

    *state.tts_engine.lock().await = Some(engine);

    Ok(())
}

/// Test an announcement by playing it through TTS
#[tauri::command]
pub async fn test_announcement(text: String, state: State<'_, AppState>) -> Result<(), String> {
    let engine_lock = state.tts_engine.lock().await;
    let engine = engine_lock
        .as_ref()
        .ok_or_else(|| "TTS engine not initialized".to_string())?;

    engine
        .announce(&text)
        .await
        .map_err(|e| format!("Failed to play announcement: {}", e))?;

    Ok(())
}

/// Start monitoring log files
#[tauri::command]
pub async fn start_monitoring(state: State<'_, AppState>) -> Result<(), String> {
    // Check if already monitoring
    if state.is_monitoring.load(Ordering::SeqCst) {
        return Err("Already monitoring".to_string());
    }

    // Get config and TTS engine
    let config = state
        .config
        .lock()
        .await
        .as_ref()
        .cloned()
        .ok_or_else(|| "No configuration loaded".to_string())?;

    let tts_engine = state
        .tts_engine
        .lock()
        .await
        .as_ref()
        .cloned()
        .ok_or_else(|| "TTS engine not initialized".to_string())?;

    // Create monitor
    let monitor = LogMonitor::new(config, tts_engine);

    // Spawn monitoring task
    let is_monitoring = Arc::clone(&state.is_monitoring);
    let handle = tokio::spawn(async move {
        if let Err(e) = monitor.start_monitoring().await {
            eprintln!("Monitoring error: {}", e);
        }
        is_monitoring.store(false, Ordering::SeqCst);
    });

    // Store handle and set flag
    *state.monitor_handle.lock().await = Some(handle);
    state.is_monitoring.store(true, Ordering::SeqCst);

    Ok(())
}

/// Stop monitoring log files
#[tauri::command]
pub async fn stop_monitoring(state: State<'_, AppState>) -> Result<(), String> {
    // Check if monitoring
    if !state.is_monitoring.load(Ordering::SeqCst) {
        return Err("Not currently monitoring".to_string());
    }

    // Abort the monitoring task
    if let Some(handle) = state.monitor_handle.lock().await.take() {
        handle.abort();
    }

    // Clear flag
    state.is_monitoring.store(false, Ordering::SeqCst);

    Ok(())
}

/// Get the current monitoring status
#[tauri::command]
pub async fn get_monitoring_status(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.is_monitoring.load(Ordering::SeqCst))
}

// Re-export Arc for use in start_monitoring
use std::sync::Arc;
