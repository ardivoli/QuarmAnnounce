use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Path to the default configuration file
pub static DEFAULT_CONFIG_PATH: &str = "./config.json";

/// Message configuration variants
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
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
#[derive(Deserialize, Serialize, Debug, Clone)]
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

    /// Saves configuration to the specified path
    pub async fn save(&self, path: &str) -> Result<()> {
        let json = serde_json::to_string_pretty(self)
            .context("Failed to serialize config to JSON")?;

        tokio::fs::write(path, json)
            .await
            .context(format!("Failed to write config file: {}", path))?;

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            game_directory: String::new(),
            messages: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_config_pattern() {
        let simple = MessageConfig::Simple {
            pattern: "test pattern".to_string(),
            announcement: "test announcement".to_string(),
        };
        assert_eq!(simple.pattern(), "test pattern");

        let timed = MessageConfig::TimedDelay {
            pattern: "timed pattern".to_string(),
            announcement: "timed announcement".to_string(),
            timer_delay_in_seconds: 30,
        };
        assert_eq!(timed.pattern(), "timed pattern");
    }

    #[test]
    fn test_message_config_announcement() {
        let simple = MessageConfig::Simple {
            pattern: "test pattern".to_string(),
            announcement: "test announcement".to_string(),
        };
        assert_eq!(simple.announcement(), "test announcement");

        let timed = MessageConfig::TimedDelay {
            pattern: "timed pattern".to_string(),
            announcement: "timed announcement".to_string(),
            timer_delay_in_seconds: 30,
        };
        assert_eq!(timed.announcement(), "timed announcement");
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.game_directory, "");
        assert_eq!(config.messages.len(), 0);
    }
}
