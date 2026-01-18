use anyhow::{Context, Result};
use serde::Deserialize;

/// Path to the configuration file
pub static CONFIG_PATH: &str = "./config.json";

/// Message configuration variants
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessageConfig {
    /// Immediate announcement when pattern matches
    Simple {
        pattern: String,
        announcement: String,
    },
    /// Delayed announcement triggered after a timer
    TimedDelay {
        pattern: String,
        announcement: String,
        timer_delay_in_seconds: u64,
    },
}

impl MessageConfig {
    /// Get the pattern for this message config
    pub fn pattern(&self) -> &str {
        match self {
            MessageConfig::Simple { pattern, .. } => pattern,
            MessageConfig::TimedDelay { pattern, .. } => pattern,
        }
    }

    /// Get the announcement for this message config
    pub fn announcement(&self) -> &str {
        match self {
            MessageConfig::Simple { announcement, .. } => announcement,
            MessageConfig::TimedDelay { announcement, .. } => announcement,
        }
    }
}

/// Application configuration
#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub game_directory: String,
    pub messages: Vec<MessageConfig>,
}

impl Config {
    /// Loads configuration from the specified path
    pub async fn load(path: &str) -> Result<Self> {
        let contents = tokio::fs::read_to_string(path)
            .await
            .context(format!("Failed to read config file: {}", path))?;

        let config: Config =
            serde_json::from_str(&contents).context("Failed to parse config JSON")?;

        Ok(config)
    }
}
