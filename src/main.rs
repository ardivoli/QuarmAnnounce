use std::collections::HashMap;

use anyhow::{Context, Result};

mod audio;
mod log_monitor;

use audio::{TtsEngine, CONFIG_PATH};
use log_monitor::LogMonitor;

// Message configuration constant
static MESSAGE_CONFIG_PATH: &str = "./config.json";

// Configuration types
#[derive(serde::Deserialize, Debug, Clone)]
pub struct Config {
    pub game_directory: String,
    pub message_announcements: HashMap<String, String>,
}

// Config loading

/// Loads configuration from config.json file
async fn load_config(path: &str) -> Result<Config> {
    let contents = tokio::fs::read_to_string(path)
        .await
        .context(format!("Failed to read config file: {}", path))?;

    let config: Config =
        serde_json::from_str(&contents).context("Failed to parse config JSON")?;

    Ok(config)
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting quarm_announce...");

    // Load configuration
    let config = load_config(MESSAGE_CONFIG_PATH)
        .await
        .context("Failed to load configuration")?;

    println!("Configuration loaded successfully");
    println!("Game directory: {}", config.game_directory);
    println!("Monitoring {} message patterns", config.message_announcements.len());

    // Initialize TTS engine
    let mut tts_engine = TtsEngine::new(CONFIG_PATH)
        .await
        .context("Failed to initialize TTS engine")?;

    // Pre-cache all announcement audio for faster playback
    tts_engine
        .precache(config.message_announcements.values())
        .await
        .context("Failed to pre-cache announcement audio")?;

    println!("TTS engine initialized successfully");

    // Create and start log monitor
    let monitor = LogMonitor::new(config, tts_engine);
    monitor
        .start_monitoring()
        .await
        .context("Log monitoring failed")?;

    Ok(())
}
