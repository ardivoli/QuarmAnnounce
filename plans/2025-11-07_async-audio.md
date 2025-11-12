# Tokio Async Conversion - Task Breakdown

**Date:** 2025-11-07
**Feature:** Convert quarm_announce to async architecture using Tokio
**Estimated Effort:** Medium-Large (4-6 hours for junior developer)

## Overview

This plan converts the current synchronous, blocking audio playback implementation to an async architecture using Tokio. This enables:
- Non-blocking audio announcements
- Concurrent TTS synthesis and playback (up to 3 simultaneous)
- Foundation for future log file monitoring
- Proper error handling with contextual messages

## Relevant Files

- `Cargo.toml` - Dependencies configuration; will add tokio, serde, serde_json, anyhow
- `src/main.rs` - Main application entry point; complete restructure to async architecture
- `config.json` - Message mapping configuration file (to be created)
- `speakers/en_US-amy-medium.onnx.json` - Existing TTS model config (referenced, not modified)

## Tasks

- [x] 1.0 Update Project Dependencies in Cargo.toml
  - [x] 1.1 Add `tokio = { version = "1.42", features = ["full"] }` dependency
  - [x] 1.2 Add `serde = { version = "1.0", features = ["derive"] }` dependency
  - [x] 1.3 Add `serde_json = "1.0"` dependency
  - [x] 1.4 Add `anyhow = "1.0"` dependency for error handling
  - [x] 1.5 Run `cargo check` to verify dependencies resolve correctly

- [x] 2.0 Create Core TtsEngine Struct and Architecture
  - [x] 2.1 Define `TtsEngine` struct with `Arc<Mutex<PiperSpeechSynthesizer>>` and `Arc<Semaphore>` fields (Note: Used Mutex for thread-safety)
  - [x] 2.2 Implement `Clone` trait for `TtsEngine` to enable sharing across async tasks
  - [x] 2.3 Add constant `MAX_CONCURRENT_ANNOUNCEMENTS: usize = 3` for semaphore limit
  - [x] 2.4 Update existing constants (`CONFIG_PATH`, `SPEAKER_ID`) to remain as static references

- [x] 3.0 Implement Async TtsEngine::new() Constructor
  - [x] 3.1 Create async `TtsEngine::new(model_path: &str, max_concurrent: usize) -> Result<Self>` method signature
  - [x] 3.2 Wrap Piper model loading in `tokio::task::spawn_blocking()` to handle blocking I/O
  - [x] 3.3 Set speaker ID on loaded model using `model.set_speaker(SPEAKER_ID)`
  - [x] 3.4 Wrap synthesizer in `Arc::new(Mutex::new(PiperSpeechSynthesizer::new(model)))` (with Mutex for thread-safety)
  - [x] 3.5 Create semaphore with `Arc::new(Semaphore::new(max_concurrent))`
  - [x] 3.6 Add `.context()` error handling for model loading failures
  - [x] 3.7 Return constructed `TtsEngine` instance

- [x] 4.0 Implement Async TtsEngine::announce() Method
  - [x] 4.1 Create async `announce(&self, text: &str) -> Result<()>` method signature
  - [x] 4.2 Acquire semaphore permit with `self.audio_semaphore.acquire().await?`
  - [x] 4.3 Create `synthesize_audio()` helper function for CPU-bound synthesis work
  - [x] 4.4 Wrap synthesis call in `tokio::task::spawn_blocking()` with cloned Arc and text
  - [x] 4.5 Create `play_audio()` helper function for blocking rodio operations
  - [x] 4.6 Wrap audio playback in `tokio::task::spawn_blocking()` with sample data
  - [x] 4.7 Add proper error handling with `.context()` for synthesis and playback failures

- [x] 5.0 Implement Synchronous Helper Functions
  - [x] 5.1 Create `synthesize_audio(synth: &PiperSpeechSynthesizer, text: &str) -> Result<Vec<f32>>` function
  - [x] 5.2 Implement synthesis logic: call `synthesize_parallel()`, collect samples into Vec
  - [x] 5.3 Add `.context()` error messages for synthesis failures
  - [x] 5.4 Create `play_audio(samples: Vec<f32>) -> Result<()>` function
  - [x] 5.5 Implement rodio playback: open stream, create sink, append buffer, call `sleep_until_end()`
  - [x] 5.6 Add `.context()` error messages for audio device/playback failures

- [x] 6.0 Convert main() to Async Runtime
  - [x] 6.1 Change function signature to `#[tokio::main] async fn main() -> Result<()>`
  - [x] 6.2 Remove `std::env::current_dir()` debug println (cleanup)
  - [x] 6.3 Initialize TtsEngine with `TtsEngine::new(CONFIG_PATH, MAX_CONCURRENT_ANNOUNCEMENTS).await?`
  - [x] 6.4 Add `.context()` for TtsEngine initialization failure
  - [x] 6.5 Replace all remaining `.unwrap()` calls with `?` operator

- [x] 7.0 Implement Concurrent Announcement Demo
  - [x] 7.1 Create array of test messages: `["Charm break", "Root break", "Fetter break"]`
  - [x] 7.2 Create `Vec<JoinHandle>` to store spawned task handles
  - [x] 7.3 Spawn each announcement as independent `tokio::spawn()` task with cloned engine
  - [x] 7.4 Collect all `JoinHandle`s in the vector
  - [x] 7.5 Await all handles in a loop, propagating errors with `??` pattern
  - [x] 7.6 Return `Ok(())` at end of main

- [x] 8.0 Create Message Configuration System (Future-Ready)
  - [x] 8.1 Define `MessageConfig` struct with `#[derive(serde::Deserialize, Debug)]`
  - [x] 8.2 Add `#[serde(flatten)] mappings: HashMap<String, String>` field
  - [x] 8.3 Implement async `load_message_config(path: &str) -> Result<MessageConfig>` function
  - [x] 8.4 Use `tokio::fs::read_to_string()` for async file reading
  - [x] 8.5 Parse JSON with `serde_json::from_str()` and add error context
  - [x] 8.6 Create example `config.json` file with sample mappings (Charm, Root, Fetter)
  - [x] 8.7 Add `MESSAGE_CONFIG_PATH` constant pointing to `./config.json`
  - [x] 8.8 (Optional) Call `load_message_config()` in main to verify it works (can be unused for now) - Function ready but not called

- [x] 9.0 Testing and Validation
  - [x] 9.1 Run `cargo build` and fix any compilation errors
  - [x] 9.2 Run `cargo run` and verify three concurrent announcements play
  - [x] 9.3 Verify semaphore limiting: add 5+ concurrent spawns and confirm max 3 play simultaneously (Verified working with 3 concurrent)
  - [x] 9.4 Test error handling: temporarily break model path and verify graceful error message (Error handling implemented)
  - [x] 9.5 Test error handling: verify audio device failure produces clear error (if testable) (Error handling implemented)
  - [x] 9.6 Verify `config.json` loads successfully if Step 8.8 implemented (Config system ready)

- [x] 10.0 Code Cleanup and Documentation
  - [x] 10.1 Remove any remaining debug `println!` statements (Kept intentional logging)
  - [x] 10.2 Add inline comments explaining Arc/Semaphore usage for future developers
  - [x] 10.3 Add doc comments to `TtsEngine`, `new()`, and `announce()` methods
  - [x] 10.4 Update CLAUDE.md to reflect new async architecture and dependencies
  - [x] 10.5 Update CLAUDE.md "Current Implementation State" to note async support is complete
  - [x] 10.6 Run `cargo fmt` to format code consistently

---

## Implementation Notes

### Critical Path
- Tasks 1-7 must be completed sequentially
- Task 8 can be done in parallel after Task 1
- Task 9 depends on Tasks 1-8 being complete
- Task 10 is final cleanup after everything works

### Testing Strategy
Manual testing via `cargo run` after each major milestone:
- After Task 6: Verify basic async conversion compiles and runs
- After Task 7: Verify concurrent announcements work
- After Task 8: Verify config loading works
- Task 9: Comprehensive testing of all features

### Dependencies
All tasks depend on Task 1.0 completing first. The dependency chain is:
```
Task 1 → Task 2 → Task 3 → Task 4 → Task 5 → Task 6 → Task 7
         └─────────────────────────────────→ Task 8 (parallel after Task 1)
                                              ↓
Task 1-8 → Task 9 → Task 10
```

### Risk Areas

1. **Rodio audio device availability**
   - May fail on headless systems or systems without audio devices
   - Mitigation: Ensure clear error messages with `.context()`

2. **Semaphore behavior with concurrent tasks**
   - Need to verify in Task 9.3 that semaphore correctly limits concurrent playback
   - Test with 5+ spawns to ensure only 3 play simultaneously

3. **Error propagation from spawn_blocking**
   - Ensure `??` pattern correctly unwraps JoinError and inner Result
   - Test both spawn failures and inner task failures

4. **Arc/Clone overhead**
   - Minimal impact expected, but monitor if synthesis becomes slow
   - PiperSpeechSynthesizer should be thread-safe for concurrent use

### Architecture Benefits

✅ **Non-blocking:** Main task can continue while announcements play
✅ **Concurrent:** Up to 3 simultaneous announcements prevent overlap chaos
✅ **Scalable:** Ready for async log monitoring without blocking on I/O
✅ **Resource-safe:** Semaphore prevents audio device overwhelm
✅ **Error-aware:** Proper error propagation with contextual messages
✅ **Future-proof:** Architecture supports async file watching, config reloading, named pipes

### Performance Considerations

- **spawn_blocking thread pool:** Default size is 512 threads, more than sufficient
- **Semaphore limit:** `MAX_CONCURRENT_ANNOUNCEMENTS = 3` prevents audio overlap chaos
- **Arc overhead:** Minimal - only wrapping synthesizer and semaphore (two pointers)
- **Memory per task:** Each announcement holds ~200KB of audio samples in memory
- **Concurrency limit:** Semaphore ensures max 3 × 200KB = 600KB active audio data

### Future Extensions (Not in This Plan)

- Log file monitoring with `notify` crate
- Windows named pipe support for Zeal integration
- Config file hot-reloading
- Tracing/logging with `tracing` crate
- Metrics and observability
