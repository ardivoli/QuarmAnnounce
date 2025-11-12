use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use piper_rs::synth::PiperSpeechSynthesizer;
use rodio::buffer::SamplesBuffer;
use tokio::sync::{Mutex, Semaphore};

// Audio-related constants
pub static CONFIG_PATH: &str = "./speakers/en_US-amy-medium.onnx.json";
pub static SPEAKER_ID: i64 = 4;
pub const MAX_CONCURRENT_ANNOUNCEMENTS: usize = 1;

/// TTS Engine for synthesizing and playing audio announcements
pub struct TtsEngine {
    synthesizer: Arc<Mutex<PiperSpeechSynthesizer>>,
    audio_semaphore: Arc<Semaphore>,
}

impl Clone for TtsEngine {
    fn clone(&self) -> Self {
        Self {
            synthesizer: Arc::clone(&self.synthesizer),
            audio_semaphore: Arc::clone(&self.audio_semaphore),
        }
    }
}

impl TtsEngine {
    /// Creates a new TtsEngine with async model loading
    pub async fn new(model_path: &str, max_concurrent: usize) -> Result<Self> {
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
        let audio_semaphore = Arc::new(Semaphore::new(max_concurrent));

        Ok(Self {
            synthesizer,
            audio_semaphore,
        })
    }

    /// Announces a message via TTS in a non-blocking way
    pub async fn announce(&self, text: &str) -> Result<()> {
        // Acquire semaphore permit to limit concurrent announcements
        let _permit = self
            .audio_semaphore
            .acquire()
            .await
            .context("Failed to acquire semaphore permit")?;

        // 1. Synthesize speech (CPU-bound, must serialize due to espeak-ng thread-safety)
        // Lock the mutex to get exclusive access to the synthesizer
        let synth = Arc::clone(&self.synthesizer);
        let text = text.to_string();
        let samples = tokio::task::spawn_blocking(move || {
            // Lock is acquired in the blocking thread to avoid holding it across await
            let synth_guard = synth.blocking_lock();
            synthesize_audio(&synth_guard, &text)
        })
        .await
        .context("Failed to spawn blocking task for synthesis")?
        .context("TTS synthesis failed")?;

        // 2. Play audio (blocking rodio operations, can run concurrently)
        tokio::task::spawn_blocking(move || play_audio(samples))
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
fn play_audio(samples: Vec<f32>) -> Result<()> {
    let stream_handle = rodio::OutputStreamBuilder::open_default_stream()
        .context("Failed to open default audio stream")?;
    let sink = rodio::Sink::connect_new(stream_handle.mixer());

    let buf = SamplesBuffer::new(1, 22050, samples);
    sink.append(buf);
    sink.sleep_until_end();

    Ok(())
}
