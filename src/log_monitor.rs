use std::collections::{HashMap, HashSet};
use std::io::SeekFrom;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, BufReader};

use crate::Config;
use crate::audio::TtsEngine;

// Timeout for checking if more lines are immediately available when batching
const BATCH_READ_TIMEOUT: Duration = Duration::from_millis(10);

// Wait time when no data is available (EOF reached)
const IDLE_RETRY_DELAY: Duration = Duration::from_millis(50);

pub struct LogMonitor {
    log_path: PathBuf,
    message_map: HashMap<String, String>,
    tts_engine: TtsEngine,
}

impl LogMonitor {
    /// Creates a new LogMonitor from config and TTS engine
    pub fn new(config: Config, tts_engine: TtsEngine) -> Self {
        Self {
            log_path: PathBuf::from(config.log_file_path),
            message_map: config.message_announcements,
            tts_engine,
        }
    }

    /// Starts monitoring the log file for configured messages
    /// This function runs forever until an error occurs or the program is terminated
    pub async fn start_monitoring(&self) -> Result<()> {
        println!("Opening log file: {:?}", self.log_path);

        // Open the log file (error if doesn't exist)
        let file = tokio::fs::File::open(&self.log_path)
            .await
            .context(format!(
                "Failed to open log file: {}",
                self.log_path.display()
            ))?;

        let mut reader = BufReader::new(file);

        // Seek to end of file to only read new lines
        reader
            .seek(SeekFrom::End(0))
            .await
            .context("Failed to seek to end of log file")?;

        println!("Monitoring log file for new messages...");

        // Process log lines in an infinite loop
        self.process_log_lines(&mut reader).await
    }

    /// Processes log lines in an infinite loop, announcing matches
    /// Reads lines in batches and deduplicates announcements to avoid repeating the same message
    async fn process_log_lines<R>(&self, reader: &mut R) -> Result<()>
    where
        R: AsyncBufReadExt + Unpin,
    {
        let mut line = String::new();

        loop {
            // Process one batch of log lines
            match self.process_one_batch(reader, &mut line).await? {
                None => {
                    // EOF reached - wait briefly and retry
                    tokio::time::sleep(IDLE_RETRY_DELAY).await;
                }
                Some(unique_announcements) => {
                    // Spawn announcement tasks for all unique messages in this batch
                    for announcement in unique_announcements {
                        let engine = self.tts_engine.clone();
                        tokio::spawn(async move {
                            if let Err(e) = engine.announce(&announcement).await {
                                eprintln!("Failed to announce message: {}", e);
                            }
                        });
                    }
                }
            }
        }
    }

    /// Processes one batch of log lines, collecting unique announcements
    ///
    /// Returns:
    /// - `Ok(None)` if EOF is reached immediately (caller should sleep and retry)
    /// - `Ok(Some(HashSet))` if data was read (empty if no matches found)
    /// - `Err` on read errors
    async fn process_one_batch<R>(
        &self,
        reader: &mut R,
        line_buffer: &mut String,
    ) -> Result<Option<HashSet<String>>>
    where
        R: AsyncBufReadExt + Unpin,
    {
        line_buffer.clear();

        // Try to read the first line
        let bytes_read = reader
            .read_line(line_buffer)
            .await
            .context("Failed to read line from log file")?;

        if bytes_read == 0 {
            // EOF reached - signal caller to sleep
            return Ok(None);
        }

        // We got at least one line - start batch collection
        let mut unique_announcements = HashSet::new();

        // Check if this first line matches
        if let Some(announcement) = self.match_message(line_buffer) {
            println!(
                "Match found! Log: '{}' -> Announcing: '{}'",
                line_buffer.trim(),
                announcement
            );
            unique_announcements.insert(announcement.to_string());
        }

        // Try to read more lines with timeout to batch collect immediately available data
        loop {
            line_buffer.clear();

            // Use timeout to check if more data is immediately available
            match tokio::time::timeout(BATCH_READ_TIMEOUT, reader.read_line(line_buffer)).await {
                Ok(Ok(bytes)) if bytes > 0 => {
                    // Got another line - check for match
                    if let Some(announcement) = self.match_message(line_buffer) {
                        println!(
                            "Match found! Log: '{}' -> Announcing: '{}'",
                            line_buffer.trim(),
                            announcement
                        );
                        unique_announcements.insert(announcement.to_string());
                    }
                }
                Ok(Ok(_)) => {
                    // EOF reached - stop batching
                    break;
                }
                Ok(Err(e)) => {
                    // Read error
                    return Err(e).context("Failed to read line from log file");
                }
                Err(_) => {
                    // Timeout - no more immediately available data
                    break;
                }
            }
        }

        Ok(Some(unique_announcements))
    }

    /// Checks if a log line matches any configured message
    /// Returns the announcement text if a match is found
    fn match_message(&self, line: &str) -> Option<&str> {
        for (log_message, announcement) in &self.message_map {
            if line.contains(log_message) {
                return Some(announcement);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tokio::io::BufReader;

    // Helper function to create a test LogMonitor with custom message mappings
    fn create_test_monitor(message_map: HashMap<String, String>) -> LogMonitor {
        LogMonitor {
            log_path: PathBuf::from("/test/path"),
            message_map,
            // Create a mock TtsEngine - it won't be used in process_one_batch tests
            // but is required for struct construction
            tts_engine: TtsEngine::new_mock().expect("Failed to create mock TTS engine"),
        }
    }

    #[tokio::test]
    async fn test_deduplicates_identical_messages() {
        // Setup: 5 identical charm messages
        let mut message_map = HashMap::new();
        message_map.insert(
            "charm spell has worn off".to_string(),
            "charm break".to_string(),
        );

        let monitor = create_test_monitor(message_map);

        let log_data = "Your charm spell has worn off.\n".repeat(5);
        let mut reader = BufReader::new(log_data.as_bytes());
        let mut line_buffer = String::new();

        // Act
        let result = monitor
            .process_one_batch(&mut reader, &mut line_buffer)
            .await
            .unwrap();

        // Assert: Should get Some(HashSet) with only 1 unique announcement
        assert!(result.is_some());
        let announcements = result.unwrap();
        assert_eq!(announcements.len(), 1);
        assert!(announcements.contains("charm break"));
    }

    #[tokio::test]
    async fn test_preserves_different_message_types() {
        // Setup: Mix of charm and root messages
        let mut message_map = HashMap::new();
        message_map.insert(
            "charm spell has worn off".to_string(),
            "charm break".to_string(),
        );
        message_map.insert("Root spell has worn off".to_string(), "root break".to_string());

        let monitor = create_test_monitor(message_map);

        let log_data = "Your charm spell has worn off.\n\
                       Your Root spell has worn off.\n\
                       Your charm spell has worn off.\n\
                       Your charm spell has worn off.\n";
        let mut reader = BufReader::new(log_data.as_bytes());
        let mut line_buffer = String::new();

        // Act
        let result = monitor
            .process_one_batch(&mut reader, &mut line_buffer)
            .await
            .unwrap();

        // Assert: Should get 2 unique announcements (charm + root)
        assert!(result.is_some());
        let announcements = result.unwrap();
        assert_eq!(announcements.len(), 2);
        assert!(announcements.contains("charm break"));
        assert!(announcements.contains("root break"));
    }

    #[tokio::test]
    async fn test_single_line_announcement() {
        // Setup: Single matching line
        let mut message_map = HashMap::new();
        message_map.insert(
            "charm spell has worn off".to_string(),
            "charm break".to_string(),
        );

        let monitor = create_test_monitor(message_map);

        let log_data = "Your charm spell has worn off.\n";
        let mut reader = BufReader::new(log_data.as_bytes());
        let mut line_buffer = String::new();

        // Act
        let result = monitor
            .process_one_batch(&mut reader, &mut line_buffer)
            .await
            .unwrap();

        // Assert
        assert!(result.is_some());
        let announcements = result.unwrap();
        assert_eq!(announcements.len(), 1);
        assert!(announcements.contains("charm break"));
    }

    #[tokio::test]
    async fn test_no_matching_lines() {
        // Setup
        let mut message_map = HashMap::new();
        message_map.insert(
            "charm spell has worn off".to_string(),
            "charm break".to_string(),
        );

        let monitor = create_test_monitor(message_map);

        let log_data = "Some random log message.\n\
                       Another unrelated message.\n";
        let mut reader = BufReader::new(log_data.as_bytes());
        let mut line_buffer = String::new();

        // Act
        let result = monitor
            .process_one_batch(&mut reader, &mut line_buffer)
            .await
            .unwrap();

        // Assert: Should get Some(empty HashSet)
        assert!(result.is_some());
        let announcements = result.unwrap();
        assert_eq!(announcements.len(), 0);
    }

    #[tokio::test]
    async fn test_eof_immediately() {
        // Setup: Empty data (immediate EOF)
        let mut message_map = HashMap::new();
        message_map.insert(
            "charm spell has worn off".to_string(),
            "charm break".to_string(),
        );

        let monitor = create_test_monitor(message_map);

        let log_data = "";
        let mut reader = BufReader::new(log_data.as_bytes());
        let mut line_buffer = String::new();

        // Act
        let result = monitor
            .process_one_batch(&mut reader, &mut line_buffer)
            .await
            .unwrap();

        // Assert: Should get None (EOF)
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_mixed_matches_and_non_matches() {
        // Setup
        let mut message_map = HashMap::new();
        message_map.insert(
            "charm spell has worn off".to_string(),
            "charm break".to_string(),
        );
        message_map.insert("snare".to_string(), "snare faded".to_string());

        let monitor = create_test_monitor(message_map);

        let log_data = "Your charm spell has worn off.\n\
                       Random unrelated message.\n\
                       Your charm spell has worn off.\n\
                       Your snare has faded.\n\
                       Another random message.\n\
                       Your charm spell has worn off.\n";
        let mut reader = BufReader::new(log_data.as_bytes());
        let mut line_buffer = String::new();

        // Act
        let result = monitor
            .process_one_batch(&mut reader, &mut line_buffer)
            .await
            .unwrap();

        // Assert: Should get 2 unique announcements despite 3 charm lines
        assert!(result.is_some());
        let announcements = result.unwrap();
        assert_eq!(announcements.len(), 2);
        assert!(announcements.contains("charm break"));
        assert!(announcements.contains("snare faded"));
    }

    #[tokio::test]
    async fn test_match_message() {
        // Test the match_message helper
        let mut message_map = HashMap::new();
        message_map.insert(
            "charm spell has worn off".to_string(),
            "charm break".to_string(),
        );

        let monitor = create_test_monitor(message_map);

        // Should match
        assert_eq!(
            monitor.match_message("Your charm spell has worn off."),
            Some("charm break")
        );

        // Should not match
        assert_eq!(monitor.match_message("Some other message"), None);
    }
}
