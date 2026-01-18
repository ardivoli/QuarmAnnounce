use anyhow::{Context, Result};

mod audio;
mod config;
mod log_monitor;

use audio::TtsEngine;
use log_monitor::LogMonitor;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting quarm_announce...");

    // Load configuration
    let config = config::Config::load(config::CONFIG_PATH)
        .await
        .context("Failed to load configuration")?;

    println!("Configuration loaded successfully");
    println!("Game directory: {}", config.game_directory);
    println!("Monitoring {} message patterns", config.messages.len());

    // Initialize TTS engine
    let mut tts_engine = TtsEngine::new(audio::CONFIG_PATH)
        .await
        .context("Failed to initialize TTS engine")?;

    // Pre-cache all announcement audio for faster playback
    tts_engine
        .precache(config.messages.iter().map(|m| m.announcement()))
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
