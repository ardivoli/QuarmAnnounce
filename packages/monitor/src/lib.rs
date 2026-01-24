use std::collections::{HashMap, HashSet};
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, BufReader};
use tokio::task::JoinHandle;

use quarm_audio::TtsEngine;
use quarm_config::{Config, MessageConfig};

// Prefix for log files we're interested in
const LOG_FILE_PREFIX: &str = "eqlog_";

// Interval for checking if a different log file has become most recent
const MTIME_CHECK_INTERVAL: Duration = Duration::from_secs(1);

// Timeout for checking if more lines are immediately available when batching
const BATCH_READ_TIMEOUT: Duration = Duration::from_millis(10);

// Wait time when no data is available (EOF reached)
const IDLE_RETRY_DELAY: Duration = Duration::from_millis(50);

/// Scans the given directory for eqlog_* files and returns the most recently modified one.
/// Returns None if no matching log files are found.
fn find_most_recent_log(directory: &Path) -> Result<Option<PathBuf>> {
    let entries = std::fs::read_dir(directory)
        .context(format!("Failed to read directory: {}", directory.display()))?;

    let mut most_recent: Option<(PathBuf, std::time::SystemTime)> = None;

    for entry in entries.filter_map(|e| e.ok()) {
        let file_name = entry.file_name();
        let name_str = file_name.to_string_lossy();

        if name_str.starts_with(LOG_FILE_PREFIX)
            && let Ok(metadata) = entry.metadata()
                && let Ok(mtime) = metadata.modified() {
                    match &most_recent {
                        None => most_recent = Some((entry.path(), mtime)),
                        Some((_, prev_mtime)) if mtime > *prev_mtime => {
                            most_recent = Some((entry.path(), mtime));
                        }
                        _ => {}
                    }
                }
    }

    Ok(most_recent.map(|(path, _)| path))
}

/// Result of processing a batch of log lines
struct BatchResult {
    /// Immediate announcements to play now (Simple message types)
    immediate: Vec<String>,
    /// Timed delay announcements: pattern -> (announcement, delay_seconds)
    /// Pattern is used as key for batch-level deduplication
    timed_delay: HashMap<String, (String, u64)>,
}

pub struct LogMonitor {
    game_directory: PathBuf,
    messages: Vec<MessageConfig>,
    tts_engine: TtsEngine,
    /// Active timers tracked by pattern string
    /// Key: pattern, Value: JoinHandle for the timer task
    active_timers: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
}

impl LogMonitor {
    /// Creates a new LogMonitor from config and TTS engine
    pub fn new(config: Config, tts_engine: TtsEngine) -> Self {
        Self {
            game_directory: PathBuf::from(config.game_directory),
            messages: config.messages,
            tts_engine,
            active_timers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Starts monitoring log files for configured messages
    /// Automatically tracks the most recently modified eqlog_* file
    /// This function runs forever until an error occurs or the program is terminated
    pub async fn start_monitoring(&self) -> Result<()> {
        println!("Scanning directory: {:?}", self.game_directory);

        loop {
            // Find the most recent log file
            let log_path = match find_most_recent_log(&self.game_directory)? {
                Some(path) => path,
                None => {
                    println!("No eqlog_* files found, waiting...");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };

            println!("Monitoring: {:?}", log_path);

            // Open and seek to end
            let file = tokio::fs::File::open(&log_path)
                .await
                .context(format!("Failed to open: {}", log_path.display()))?;
            let mut reader = BufReader::new(file);
            reader
                .seek(SeekFrom::End(0))
                .await
                .context("Failed to seek to end of log file")?;

            // Monitor this file until a different file becomes most recent
            let mut last_mtime_check = std::time::Instant::now();
            let mut line_buffer = String::new();

            loop {
                match self.process_one_batch(&mut reader, &mut line_buffer).await? {
                    Some(batch_result) => {
                        // Spawn announcement tasks for immediate messages
                        for announcement in batch_result.immediate {
                            let engine = self.tts_engine.clone();
                            tokio::spawn(async move {
                                if let Err(e) = engine.announce(&announcement).await {
                                    eprintln!("Failed to announce message: {}", e);
                                }
                            });
                        }

                        // Schedule timed delay announcements
                        for (pattern, (announcement, delay_seconds)) in batch_result.timed_delay {
                            // Use pattern as key for debouncing
                            self.schedule_timed_delay(pattern, announcement, delay_seconds);
                        }
                    }
                    None => {
                        // EOF reached - check if we should switch files
                        if last_mtime_check.elapsed() >= MTIME_CHECK_INTERVAL {
                            last_mtime_check = std::time::Instant::now();
                            if let Some(new_path) = find_most_recent_log(&self.game_directory)?
                                && new_path != log_path {
                                    println!("Switching to: {:?}", new_path);
                                    break; // Break inner loop to reopen with new file
                                }
                        }
                        tokio::time::sleep(IDLE_RETRY_DELAY).await;
                    }
                }
            }
        }
    }

    /// Processes one batch of log lines, collecting unique announcements
    ///
    /// Returns:
    /// - `Ok(None)` if EOF is reached immediately (caller should sleep and retry)
    /// - `Ok(Some(BatchResult))` if data was read (categorized by message type)
    /// - `Err` on read errors
    async fn process_one_batch<R>(
        &self,
        reader: &mut R,
        line_buffer: &mut String,
    ) -> Result<Option<BatchResult>>
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
        // Use HashSet for deduplication of immediate announcements
        let mut immediate_set = HashSet::new();
        let mut timed_delay = HashMap::new();

        // Check if this first line matches any configured messages
        for config in self.match_message(line_buffer) {
            println!(
                "Match found! Log: '{}' -> Announcing: '{}'",
                line_buffer.trim(),
                config.announcement()
            );
            match config {
                MessageConfig::Simple { announcement, .. } => {
                    immediate_set.insert(announcement.clone());
                }
                MessageConfig::TimedDelay {
                    pattern,
                    announcement,
                    timer_delay_in_seconds,
                } => {
                    timed_delay.insert(
                        pattern.clone(),
                        (announcement.clone(), *timer_delay_in_seconds),
                    );
                }
            }
        }

        // Try to read more lines with timeout to batch collect immediately available data
        loop {
            line_buffer.clear();

            // Use timeout to check if more data is immediately available
            match tokio::time::timeout(BATCH_READ_TIMEOUT, reader.read_line(line_buffer)).await {
                Ok(Ok(bytes)) if bytes > 0 => {
                    // Got another line - check for matches
                    for config in self.match_message(line_buffer) {
                        println!(
                            "Match found! Log: '{}' -> Announcing: '{}'",
                            line_buffer.trim(),
                            config.announcement()
                        );
                        match config {
                            MessageConfig::Simple { announcement, .. } => {
                                immediate_set.insert(announcement.clone());
                            }
                            MessageConfig::TimedDelay {
                                pattern,
                                announcement,
                                timer_delay_in_seconds,
                            } => {
                                timed_delay.insert(
                                    pattern.clone(),
                                    (announcement.clone(), *timer_delay_in_seconds),
                                );
                            }
                        }
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

        Ok(Some(BatchResult {
            immediate: immediate_set.into_iter().collect(),
            timed_delay,
        }))
    }

    /// Schedules a timed delay announcement
    /// If a timer already exists for this pattern, it will be cancelled and replaced (debounce behavior)
    fn schedule_timed_delay(&self, pattern: String, announcement: String, delay_seconds: u64) {
        let timers = Arc::clone(&self.active_timers);
        let engine = self.tts_engine.clone();

        // Cancel existing timer for this pattern if present
        {
            let mut timers_map = timers.lock().unwrap();
            if let Some(old_handle) = timers_map.remove(&pattern) {
                old_handle.abort();
                println!("Cancelled existing timer for pattern: '{}'", pattern);
            }
        }

        // Clone for logging before moving into async block
        let pattern_clone = pattern.clone();
        let announcement_clone = announcement.clone();

        // Start new timer
        let handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(delay_seconds)).await;
            if let Err(e) = engine.announce(&announcement).await {
                eprintln!("Failed to announce timed message: {}", e);
            }
        });

        // Store the new timer handle
        {
            let mut timers_map = timers.lock().unwrap();
            timers_map.insert(pattern, handle);
        }

        println!(
            "Scheduled timer: '{}' -> '{}' ({}s)",
            pattern_clone, announcement_clone, delay_seconds
        );
    }

    /// Checks if a log line matches any configured messages
    /// Returns all matching MessageConfigs (supports same pattern with different types)
    fn match_message(&self, line: &str) -> Vec<&MessageConfig> {
        self.messages
            .iter()
            .filter(|message_config| line.contains(message_config.pattern()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::BufReader;

    // Helper function to create a test LogMonitor with custom message configs
    fn create_test_monitor(messages: Vec<MessageConfig>) -> LogMonitor {
        LogMonitor {
            game_directory: PathBuf::from("/test/game"),
            messages,
            // Create a mock TtsEngine - it won't be used in process_one_batch tests
            // but is required for struct construction
            tts_engine: TtsEngine::new_mock().expect("Failed to create mock TTS engine"),
            active_timers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    #[tokio::test]
    async fn test_deduplicates_identical_messages() {
        // Setup: 5 identical charm messages
        let messages = vec![MessageConfig::Simple {
            pattern: "charm spell has worn off".to_string(),
            announcement: "charm break".to_string(),
        }];

        let monitor = create_test_monitor(messages);

        let log_data = "Your charm spell has worn off.\n".repeat(5);
        let mut reader = BufReader::new(log_data.as_bytes());
        let mut line_buffer = String::new();

        // Act
        let result = monitor
            .process_one_batch(&mut reader, &mut line_buffer)
            .await
            .unwrap();

        // Assert: Should get Some(BatchResult) with only 1 unique immediate announcement
        assert!(result.is_some());
        let batch = result.unwrap();
        assert_eq!(batch.immediate.len(), 1);
        assert!(batch.immediate.contains(&"charm break".to_string()));
        assert_eq!(batch.timed_delay.len(), 0);
    }

    #[tokio::test]
    async fn test_preserves_different_message_types() {
        // Setup: Mix of charm and root messages
        let messages = vec![
            MessageConfig::Simple {
                pattern: "charm spell has worn off".to_string(),
                announcement: "charm break".to_string(),
            },
            MessageConfig::Simple {
                pattern: "Root spell has worn off".to_string(),
                announcement: "root break".to_string(),
            },
        ];

        let monitor = create_test_monitor(messages);

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
        let batch = result.unwrap();
        assert_eq!(batch.immediate.len(), 2);
        assert!(batch.immediate.contains(&"charm break".to_string()));
        assert!(batch.immediate.contains(&"root break".to_string()));
        assert_eq!(batch.timed_delay.len(), 0);
    }

    #[tokio::test]
    async fn test_single_line_announcement() {
        // Setup: Single matching line
        let messages = vec![MessageConfig::Simple {
            pattern: "charm spell has worn off".to_string(),
            announcement: "charm break".to_string(),
        }];

        let monitor = create_test_monitor(messages);

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
        let batch = result.unwrap();
        assert_eq!(batch.immediate.len(), 1);
        assert!(batch.immediate.contains(&"charm break".to_string()));
        assert_eq!(batch.timed_delay.len(), 0);
    }

    #[tokio::test]
    async fn test_no_matching_lines() {
        // Setup
        let messages = vec![MessageConfig::Simple {
            pattern: "charm spell has worn off".to_string(),
            announcement: "charm break".to_string(),
        }];

        let monitor = create_test_monitor(messages);

        let log_data = "Some random log message.\n\
                       Another unrelated message.\n";
        let mut reader = BufReader::new(log_data.as_bytes());
        let mut line_buffer = String::new();

        // Act
        let result = monitor
            .process_one_batch(&mut reader, &mut line_buffer)
            .await
            .unwrap();

        // Assert: Should get Some(empty BatchResult)
        assert!(result.is_some());
        let batch = result.unwrap();
        assert_eq!(batch.immediate.len(), 0);
        assert_eq!(batch.timed_delay.len(), 0);
    }

    #[tokio::test]
    async fn test_eof_immediately() {
        // Setup: Empty data (immediate EOF)
        let messages = vec![MessageConfig::Simple {
            pattern: "charm spell has worn off".to_string(),
            announcement: "charm break".to_string(),
        }];

        let monitor = create_test_monitor(messages);

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
        let messages = vec![
            MessageConfig::Simple {
                pattern: "charm spell has worn off".to_string(),
                announcement: "charm break".to_string(),
            },
            MessageConfig::Simple {
                pattern: "snare".to_string(),
                announcement: "snare faded".to_string(),
            },
        ];

        let monitor = create_test_monitor(messages);

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
        let batch = result.unwrap();
        assert_eq!(batch.immediate.len(), 2);
        assert!(batch.immediate.contains(&"charm break".to_string()));
        assert!(batch.immediate.contains(&"snare faded".to_string()));
        assert_eq!(batch.timed_delay.len(), 0);
    }

    #[test]
    fn test_match_message() {
        // Test the match_message helper
        let messages = vec![MessageConfig::Simple {
            pattern: "charm spell has worn off".to_string(),
            announcement: "charm break".to_string(),
        }];

        let monitor = create_test_monitor(messages);

        // Should match
        let result = monitor.match_message("Your charm spell has worn off.");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].announcement(), "charm break");

        // Should not match
        assert!(monitor.match_message("Some other message").is_empty());
    }

    #[test]
    fn test_find_most_recent_log_no_files() {
        // Create a temp directory with no eqlog files
        let temp_dir = std::env::temp_dir().join("test_no_logs");
        std::fs::create_dir_all(&temp_dir).unwrap();

        let result = find_most_recent_log(&temp_dir).unwrap();
        assert!(result.is_none());

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[tokio::test]
    async fn test_timed_delay_message_batching() {
        // Setup: TimedDelay message config
        let messages = vec![MessageConfig::TimedDelay {
            pattern: "Charm spell has taken hold".to_string(),
            announcement: "charm about to break".to_string(),
            timer_delay_in_seconds: 30,
        }];

        let monitor = create_test_monitor(messages);

        let log_data = "Your Charm spell has taken hold.\n";
        let mut reader = BufReader::new(log_data.as_bytes());
        let mut line_buffer = String::new();

        // Act
        let result = monitor
            .process_one_batch(&mut reader, &mut line_buffer)
            .await
            .unwrap();

        // Assert: Should get TimedDelay in batch result
        assert!(result.is_some());
        let batch = result.unwrap();
        assert_eq!(batch.immediate.len(), 0);
        assert_eq!(batch.timed_delay.len(), 1);

        let (announcement, delay) = batch.timed_delay.get("Charm spell has taken hold").unwrap();
        assert_eq!(announcement, "charm about to break");
        assert_eq!(*delay, 30);
    }

    #[tokio::test]
    async fn test_mixed_simple_and_timed_delay() {
        // Setup: Mix of Simple and TimedDelay messages
        let messages = vec![
            MessageConfig::Simple {
                pattern: "charm spell has worn off".to_string(),
                announcement: "charm break".to_string(),
            },
            MessageConfig::TimedDelay {
                pattern: "Charm spell has taken hold".to_string(),
                announcement: "charm about to break".to_string(),
                timer_delay_in_seconds: 30,
            },
        ];

        let monitor = create_test_monitor(messages);

        let log_data = "Your charm spell has worn off.\n\
                       Your Charm spell has taken hold.\n\
                       Your charm spell has worn off.\n";
        let mut reader = BufReader::new(log_data.as_bytes());
        let mut line_buffer = String::new();

        // Act
        let result = monitor
            .process_one_batch(&mut reader, &mut line_buffer)
            .await
            .unwrap();

        // Assert: Should get both immediate and timed_delay
        assert!(result.is_some());
        let batch = result.unwrap();

        // Should have 1 unique immediate (deduplicated charm break)
        assert_eq!(batch.immediate.len(), 1);
        assert!(batch.immediate.contains(&"charm break".to_string()));

        // Should have 1 timed_delay
        assert_eq!(batch.timed_delay.len(), 1);
        let (announcement, delay) = batch.timed_delay.get("Charm spell has taken hold").unwrap();
        assert_eq!(announcement, "charm about to break");
        assert_eq!(*delay, 30);
    }

    #[tokio::test]
    async fn test_multiple_timed_delay_same_pattern() {
        // Setup: TimedDelay message
        let messages = vec![MessageConfig::TimedDelay {
            pattern: "Charm spell has taken hold".to_string(),
            announcement: "charm about to break".to_string(),
            timer_delay_in_seconds: 30,
        }];

        let monitor = create_test_monitor(messages);

        // Multiple instances of the same timed delay message
        let log_data = "Your Charm spell has taken hold.\n".repeat(3);
        let mut reader = BufReader::new(log_data.as_bytes());
        let mut line_buffer = String::new();

        // Act
        let result = monitor
            .process_one_batch(&mut reader, &mut line_buffer)
            .await
            .unwrap();

        // Assert: Should get 1 timed_delay entry (deduplicated at batch level)
        assert!(result.is_some());
        let batch = result.unwrap();
        assert_eq!(batch.immediate.len(), 0);
        assert_eq!(batch.timed_delay.len(), 1);

        // Verify the content
        let (announcement, delay) = batch.timed_delay.get("Charm spell has taken hold").unwrap();
        assert_eq!(announcement, "charm about to break");
        assert_eq!(*delay, 30);
    }

    #[tokio::test]
    async fn test_same_pattern_simple_and_timed_delay() {
        // Setup: Same pattern with both Simple and TimedDelay types
        // This is the user's config scenario: immediate "go back in" + delayed "get out"
        let messages = vec![
            MessageConfig::Simple {
                pattern: "flesh begins to liquefy".to_string(),
                announcement: "go back in".to_string(),
            },
            MessageConfig::TimedDelay {
                pattern: "flesh begins to liquefy".to_string(),
                announcement: "get out".to_string(),
                timer_delay_in_seconds: 22,
            },
        ];

        let monitor = create_test_monitor(messages);

        // 3 identical log lines matching the same pattern
        let log_data = "Your flesh begins to liquefy.\n".repeat(3);
        let mut reader = BufReader::new(log_data.as_bytes());
        let mut line_buffer = String::new();

        // Act
        let result = monitor
            .process_one_batch(&mut reader, &mut line_buffer)
            .await
            .unwrap();

        // Assert: Should get both message types, each deduplicated
        assert!(result.is_some());
        let batch = result.unwrap();

        // 1 immediate announcement (deduplicated from 3 lines)
        assert_eq!(batch.immediate.len(), 1);
        assert!(batch.immediate.contains(&"go back in".to_string()));

        // 1 timed delay entry (deduplicated from 3 lines)
        assert_eq!(batch.timed_delay.len(), 1);
        let (announcement, delay) = batch
            .timed_delay
            .get("flesh begins to liquefy")
            .unwrap();
        assert_eq!(announcement, "get out");
        assert_eq!(*delay, 22);
    }
}
