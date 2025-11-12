use std::collections::HashMap;
use std::io::SeekFrom;
use std::path::PathBuf;

use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, BufReader};

use crate::Config;
use crate::audio::TtsEngine;

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

        let mut line = String::new();

        loop {
            // println!("Reading line from log file...");
            line.clear();

            // Try to read a line
            let bytes_read = reader
                .read_line(&mut line)
                .await
                .context("Failed to read line from log file")?;

            // println!("line data: {:?}", &line);

            if bytes_read == 0 {
                // EOF reached - wait briefly and retry
                // println!("EOF reached - waiting 1 second and retrying...");
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                continue;
            }

            // Check if line matches any configured messages
            if let Some(announcement) = self.match_message(&line) {
                println!(
                    "Match found! Log: '{}' -> Announcing: '{}'",
                    line.trim(),
                    announcement
                );

                // Spawn announcement task
                let engine = self.tts_engine.clone();
                let announcement = announcement.to_string();
                tokio::spawn(async move {
                    if let Err(e) = engine.announce(&announcement).await {
                        eprintln!("Failed to announce message: {}", e);
                    }
                });
            }
        }
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
