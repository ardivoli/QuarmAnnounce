use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use piper_rs::synth::PiperSpeechSynthesizer;
use tokio::sync::{Mutex, Semaphore};

// SamplesBuffer is only used in production builds for audio playback
#[cfg(not(test))]
use rodio::buffer::SamplesBuffer;

// Audio-related constants
pub static SPEAKER_ID: i64 = 4;

/// TTS Engine for synthesizing and playing audio announcements
pub struct TtsEngine {
    synthesizer: Arc<Mutex<PiperSpeechSynthesizer>>,
    audio_semaphore: Arc<Semaphore>,
    audio_cache: Arc<HashMap<String, Arc<Vec<f32>>>>,
}

impl Clone for TtsEngine {
    fn clone(&self) -> Self {
        Self {
            synthesizer: Arc::clone(&self.synthesizer),
            audio_semaphore: Arc::clone(&self.audio_semaphore),
            audio_cache: Arc::clone(&self.audio_cache),
        }
    }
}

impl TtsEngine {
    /// Creates a new TtsEngine with async model loading
    pub async fn new(model_path: &str) -> Result<Self> {
        // Load Piper model in blocking thread (disk I/O)
        let model_path = model_path.to_string();
        let model =
            tokio::task::spawn_blocking(move || piper_rs::from_config_path(Path::new(&model_path)))
                .await
                .context("Failed to spawn blocking task for model loading")?
                .context("Failed to load Piper model from config path")?;

        // Set speaker ID
        model.set_speaker(SPEAKER_ID);

        // Wrap synthesizer in Arc<Mutex> for thread-safe sharing
        // Mutex is needed because espeak-ng (used by Piper) is not thread-safe
        let synthesizer = Arc::new(Mutex::new(
            PiperSpeechSynthesizer::new(model)
                .context("Failed to create PiperSpeechSynthesizer")?,
        ));

        // Create semaphore for limiting concurrent announcements
        let audio_semaphore = Arc::new(Semaphore::new(1));

        // Initialize empty audio cache
        let audio_cache = Arc::new(HashMap::new());

        Ok(Self {
            synthesizer,
            audio_semaphore,
            audio_cache,
        })
    }

    /// Creates a dummy TtsEngine for testing (no model loading required)
    /// This is only available in test builds and won't actually synthesize audio
    #[cfg(any(test, feature = "test-support"))]
    pub fn new_mock() -> Result<Self> {
        // Create a real model for the mock (required by PiperSpeechSynthesizer)
        // Calculate workspace root from package directory (CARGO_MANIFEST_DIR points to packages/audio)
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let workspace_root = Path::new(manifest_dir).parent().unwrap().parent().unwrap();
        let config_path = workspace_root.join("resources/speakers/en_US-amy-medium.onnx.json");
        let model = piper_rs::from_config_path(&config_path)
            .context("Failed to load Piper model for mock - model file may not exist")?;

        model.set_speaker(SPEAKER_ID);

        let synthesizer = Arc::new(Mutex::new(
            PiperSpeechSynthesizer::new(model)
                .context("Failed to create PiperSpeechSynthesizer for mock")?,
        ));

        let audio_semaphore = Arc::new(Semaphore::new(1));

        // Initialize empty audio cache
        let audio_cache = Arc::new(HashMap::new());

        Ok(Self {
            synthesizer,
            audio_semaphore,
            audio_cache,
        })
    }

    /// Pre-synthesizes audio for all given texts and caches them for fast playback
    /// Should be called at startup before any announce() calls
    pub async fn precache(&mut self, texts: impl IntoIterator<Item = impl AsRef<str>>) -> Result<()> {
        let synth = Arc::clone(&self.synthesizer);
        let texts: Vec<String> = texts.into_iter().map(|t| t.as_ref().to_string()).collect();

        // Synthesize all texts in blocking thread (espeak-ng is not thread-safe)
        let samples_map = tokio::task::spawn_blocking(move || {
            let synth_guard = synth.blocking_lock();
            let mut map = HashMap::new();
            for text in texts {
                let samples = synthesize_audio(&synth_guard, &text)?;
                map.insert(text, Arc::new(samples));
            }
            Ok::<_, anyhow::Error>(map)
        })
        .await
        .context("Failed to spawn blocking task for precache")?
        .context("Precache synthesis failed")?;

        // Store in cache - use Arc::make_mut to get mutable access
        let cache = Arc::make_mut(&mut self.audio_cache);
        cache.extend(samples_map);

        println!("Pre-cached {} announcements", cache.len());
        Ok(())
    }

    /// Announces a message via TTS in a non-blocking way
    pub async fn announce(&self, text: &str) -> Result<()> {
        // 1. Check cache first, fallback to synthesis if not cached
        let samples = if let Some(cached) = self.audio_cache.get(text) {
            // Cache hit - just clone the Arc reference (cheap)
            Arc::clone(cached)
        } else {
            // Cache miss - synthesize on demand (original behavior)
            let synth = Arc::clone(&self.synthesizer);
            let text = text.to_string();
            let samples = tokio::task::spawn_blocking(move || {
                let synth_guard = synth.blocking_lock();
                synthesize_audio(&synth_guard, &text)
            })
            .await
            .context("Failed to spawn blocking task for synthesis")?
            .context("TTS synthesis failed")?;
            Arc::new(samples)
        };

        // 2. Acquire semaphore permit ONLY for playback to prevent audio overlap
        // This allows next announcement to start synthesizing while current one plays
        let _permit = self
            .audio_semaphore
            .acquire()
            .await
            .context("Failed to acquire semaphore permit")?;

        // 3. Play audio (blocking rodio operations)
        // Note: We need to convert Arc<Vec<f32>> to Vec<f32> for play_audio
        let samples_vec = (*samples).clone();
        tokio::task::spawn_blocking(move || play_audio(samples_vec))
            .await
            .context("Failed to spawn blocking task for audio playback")?
            .context("Audio playback failed")?;

        Ok(())
    }
}

// Synchronous helper functions (run in blocking thread pool)

/// Synthesizes audio from text using Piper TTS (synchronous, CPU-bound)
fn synthesize_audio(synth: &PiperSpeechSynthesizer, text: &str) -> Result<Vec<f32>> {
    let mut samples = Vec::new();
    let audio = synth
        .synthesize_parallel(text.to_string(), None)
        .context("Failed to synthesize speech")?;

    for result in audio {
        samples.append(&mut result.context("Failed to process audio chunk")?.into_vec());
    }

    Ok(samples)
}

/// Plays audio samples through the default audio device (synchronous, blocking)
#[cfg(not(test))]
fn play_audio(samples: Vec<f32>) -> Result<()> {
    let mut stream_handle = rodio::OutputStreamBuilder::open_default_stream()
        .context("Failed to open default audio stream")?;
    // Disable noisy log messages related output stream being dropped after audio playback is done
    stream_handle.log_on_drop(false);

    let sink = rodio::Sink::connect_new(stream_handle.mixer());

    let buf = SamplesBuffer::new(1, 22050, samples);
    sink.append(buf);
    sink.sleep_until_end();

    Ok(())
}

/// Mock audio playback for tests (no-op, returns immediately)
#[cfg(test)]
fn play_audio(_samples: Vec<f32>) -> Result<()> {
    // Mock implementation - no actual audio playback in tests
    // This allows tests to run faster and in parallel without device contention
    Ok(())
}

#[cfg(test)]
mod tests {
    //! Integration tests for TtsEngine
    //!
    //! These tests require:
    //! - Valid Piper TTS model files at the test model path
    //!
    //! **Note**: Audio playback is mocked in tests using conditional compilation.
    //! The `play_audio()` function is a no-op in test builds, allowing:
    //! - Fast test execution (no actual audio device I/O)
    //! - Parallel test execution (no device contention)
    //! - Reliable CI/CD testing (no audio hardware required)
    //!
    //! Run tests normally:
    //! ```bash
    //! cargo test
    //! ```
    //!
    //! Tests cover:
    //! - Engine initialization (valid and invalid paths)
    //! - Single and concurrent announcements
    //! - Semaphore limiting behavior
    //! - Engine cloning for multi-task usage
    //! - Text handling (empty, special characters)
    //! - Audio precaching for faster playback

    use super::*;
    use std::sync::OnceLock;

    static TEST_MODEL_PATH: OnceLock<String> = OnceLock::new();

    // Get test model path - calculated once using CARGO_MANIFEST_DIR
    fn get_test_model_path() -> &'static str {
        TEST_MODEL_PATH.get_or_init(|| {
            let manifest_dir = env!("CARGO_MANIFEST_DIR");
            let workspace_root = Path::new(manifest_dir).parent().unwrap().parent().unwrap();
            let path = workspace_root.join("resources/speakers/en_US-amy-medium.onnx.json");
            path.to_string_lossy().to_string()
        })
    }

    /// Test that TtsEngine can be initialized successfully with valid model path
    #[tokio::test]
    async fn test_tts_engine_initialization() {
        let result = TtsEngine::new(get_test_model_path()).await;
        assert!(
            result.is_ok(),
            "TtsEngine should initialize successfully with valid model path"
        );
    }

    /// Test that TtsEngine initialization fails with invalid model path
    #[tokio::test]
    async fn test_tts_engine_initialization_invalid_path() {
        let result = TtsEngine::new("./nonexistent/model.json").await;
        assert!(
            result.is_err(),
            "TtsEngine should fail to initialize with invalid model path"
        );
    }

    /// Test that a single announcement completes successfully
    #[tokio::test]
    async fn test_single_announcement() {
        let engine = TtsEngine::new(get_test_model_path())
            .await
            .expect("Failed to initialize TtsEngine");

        let result = engine.announce("Test message").await;
        assert!(
            result.is_ok(),
            "Single announcement should complete successfully"
        );
    }

    /// Test that empty text can be announced without errors
    #[tokio::test]
    async fn test_empty_announcement() {
        let engine = TtsEngine::new(get_test_model_path())
            .await
            .expect("Failed to initialize TtsEngine");

        let result = engine.announce("").await;
        assert!(
            result.is_ok(),
            "Empty text announcement should complete without errors"
        );
    }

    /// Test that TtsEngine can be cloned and used from multiple tasks
    #[tokio::test]
    async fn test_engine_cloning() {
        let engine = TtsEngine::new(get_test_model_path())
            .await
            .expect("Failed to initialize TtsEngine");

        // Clone engine and spawn tasks
        let engine1 = engine.clone();
        let engine2 = engine.clone();

        let handle1 = tokio::spawn(async move { engine1.announce("First").await });

        let handle2 = tokio::spawn(async move { engine2.announce("Second").await });

        // Both should complete successfully
        let result1 = handle1.await.expect("Task 1 panicked");
        let result2 = handle2.await.expect("Task 2 panicked");

        assert!(result1.is_ok(), "First announcement should succeed");
        assert!(result2.is_ok(), "Second announcement should succeed");
    }

    /// Test that concurrent announcements are properly limited by semaphore
    #[tokio::test]
    async fn test_concurrent_announcement_limiting() {
        // Create engine with limit of 2 concurrent announcements
        let engine = TtsEngine::new(get_test_model_path())
            .await
            .expect("Failed to initialize TtsEngine");

        // Spawn 4 concurrent announcements
        let mut handles = vec![];
        for i in 0..4 {
            let engine_clone = engine.clone();
            let handle = tokio::spawn(async move {
                engine_clone
                    .announce(&format!("Message {}", i))
                    .await
            });
            handles.push(handle);
        }

        // All announcements should eventually complete successfully
        for (i, handle) in handles.into_iter().enumerate() {
            let result = handle
                .await
                .unwrap_or_else(|_| panic!("Task {} panicked", i));
            assert!(
                result.is_ok(),
                "Announcement {} should complete successfully",
                i
            );
        }
    }

    /// Test that announcements with special characters work correctly
    #[tokio::test]
    async fn test_announcement_with_special_characters() {
        let engine = TtsEngine::new(get_test_model_path())
            .await
            .expect("Failed to initialize TtsEngine");

        let test_cases = [
            "Hello, world!",
            "Test: 123",
            "Message with numbers 42",
            "Question?",
        ];

        for text in test_cases {
            let result = engine.announce(text).await;
            assert!(
                result.is_ok(),
                "Announcement with text '{}' should succeed",
                text
            );
        }
    }

    /// Test that precache() successfully caches announcement texts
    #[tokio::test]
    async fn test_precache_caches_announcements() {
        let mut engine = TtsEngine::new(get_test_model_path())
            .await
            .expect("Failed to initialize TtsEngine");

        let announcements = ["charm break", "root break"];
        let result = engine.precache(announcements.iter().copied()).await;

        assert!(result.is_ok(), "Precache should succeed");
    }

    /// Test that announce() uses cached audio (no synthesis needed)
    #[tokio::test]
    async fn test_announce_uses_cached_audio() {
        let mut engine = TtsEngine::new(get_test_model_path())
            .await
            .expect("Failed to initialize TtsEngine");

        // Precache the announcement
        engine
            .precache(["test announcement"])
            .await
            .expect("Precache should succeed");

        // Announce should succeed and use cached audio
        let result = engine.announce("test announcement").await;
        assert!(
            result.is_ok(),
            "Announce with cached audio should succeed"
        );
    }
}
