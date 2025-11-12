use std::collections::HashMap;

use anyhow::{Context, Result};

mod audio;
use audio::{TtsEngine, CONFIG_PATH, MAX_CONCURRENT_ANNOUNCEMENTS};

// Message configuration constant
static MESSAGE_CONFIG_PATH: &str = "./config.json";

// Configuration types
#[derive(serde::Deserialize, Debug)]
struct MessageConfig {
    #[serde(flatten)]
    mappings: HashMap<String, String>,
}

// Config loading

/// Loads message mappings from config.json file
async fn load_message_config(path: &str) -> Result<MessageConfig> {
    let contents = tokio::fs::read_to_string(path)
        .await
        .context(format!("Failed to read config file: {}", path))?;

    let config: MessageConfig =
        serde_json::from_str(&contents).context("Failed to parse config JSON")?;

    Ok(config)
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting quarm_announce...");

    // Initialize TTS engine
    let tts_engine = TtsEngine::new(CONFIG_PATH, MAX_CONCURRENT_ANNOUNCEMENTS)
        .await
        .context("Failed to initialize TTS engine")?;

    println!("TTS engine initialized successfully");

    // Demo: Spawn concurrent announcements
    let test_messages = ["Charm break", "Root break", "Fetter break"];
    let mut handles = vec![];

    for message in test_messages {
        let engine = tts_engine.clone();
        let handle = tokio::spawn(async move {
            println!("Announcing: {}", message);
            engine.announce(message).await
        });
        handles.push(handle);
    }

    // Wait for all announcements to complete
    for (i, handle) in handles.into_iter().enumerate() {
        handle
            .await
            .context(format!("Failed to join task {}", i))?
            .context(format!("Announcement {} failed", i))?;
    }

    println!("All announcements completed successfully!");

    Ok(())
}
