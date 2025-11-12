# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Quarm Announce is a Rust application that monitors EverQuest log files for configured messages and announces them via text-to-speech (TTS). When a matching message is detected in the log file, the application speaks a corresponding audio announcement using the Piper TTS engine.

Example: When "Charm spell has worn off" appears in the log, the app speaks "charm wore off".

## Architecture

### Core Components

1. **Async Runtime (Tokio)**: Provides async/await infrastructure
   - Enables non-blocking I/O and concurrent task execution
   - Spawns blocking tasks for CPU-bound and synchronous operations

2. **TTS Engine (Piper)**: Uses `piper-rs` with ONNX models for neural text-to-speech
   - Model config: `./speakers/en_US-amy-medium.onnx.json`
   - Speaker ID: 4 (hardcoded)
   - Output: 22050 Hz mono audio
   - **Thread Safety**: Wrapped in `Arc<Mutex>` because espeak-ng (used internally) is not thread-safe
   - Synthesis operations are serialized via mutex lock

3. **Audio Playback (Rodio)**: Handles audio output through system default stream
   - Runs in `spawn_blocking` to avoid blocking async runtime
   - Up to 3 concurrent playbacks allowed (controlled by semaphore)

4. **TtsEngine Struct**: Coordinates synthesis and playback
   - `synthesizer`: `Arc<Mutex<PiperSpeechSynthesizer>>` for thread-safe sharing
   - `audio_semaphore`: `Arc<Semaphore>` limits concurrent announcements
   - `announce()`: Async method that synthesizes and plays TTS non-blocking

5. **Message Mapping**: Configuration system via `config.json`
   - Format: `{"<log message>": "<spoken message>"}`
   - Async loading with `load_message_config()` function (ready to use)

### Current Implementation State

**Code Organization:**
- `src/audio.rs`: Audio synthesis and playback module (TtsEngine, constants)
- `src/main.rs`: Application entry point, message config, main loop

**Implemented features:**
- ✅ Async TTS engine with Tokio runtime
- ✅ Non-blocking audio synthesis and playback
- ✅ Concurrent announcement support (up to 3 simultaneous)
- ✅ Thread-safe synthesizer access via mutex
- ✅ Proper error handling with `anyhow::Result` and context
- ✅ Modular code structure with separate audio module
- ✅ Config loading infrastructure (function implemented, not yet used)

**Demo**: Currently spawns 3 concurrent announcements ("Charm break", "Root break", "Fetter break")

**Missing features to implement:**
- Log file monitoring/tailing
- Integration of config.json message mapping with log monitoring
- Real-time log parsing and pattern matching

## Development Commands

```bash
# Build the project
cargo build

# Run the application
cargo run

# Check for compilation errors without building
cargo check

# Run with release optimizations
cargo build --release
cargo run --release

# Run tests (when added)
cargo test
```

## Key Dependencies

- `tokio` (1.48): Async runtime with full features
- `piper-rs` (0.1.9): Neural TTS synthesis
- `ort` (2.0.0-rc.9): ONNX Runtime for ML inference
- `rodio` (0.21.1): Audio playback
- `anyhow` (1.0): Error handling with context
- `serde` / `serde_json` (1.0): JSON config parsing

## Future Functionality: Named Pipes

The README documents a future feature to read from Zeal's Windows named pipes (`\\.\pipe\zeal_{processId}`) for real-time EverQuest event streaming. This would replace log file tailing.

**Key notes for named pipe implementation:**
- Windows-only IPC mechanism (Linux/Wine uses `~/.wine/dosdevices/pipe/`)
- UTF-8 encoded JSON messages with structure: `{type, data_len, character, data}`
- Type 0 (LogText) is the relevant message type for log monitoring
- Named pipes are in-memory only, no disk I/O
- Rust can connect using `std::fs::File` or Windows-specific APIs

## Project Structure

```
quarm_announce/
├── src/
│   ├── main.rs          # Application entry point, message config, main loop
│   └── audio.rs         # Audio synthesis and playback (TtsEngine, constants)
├── speakers/            # Piper ONNX models and configs
│   └── en_US-amy-medium.onnx.json
├── plans/               # Implementation plans and task breakdowns
│   └── 2025-11-07_async-audio.md
├── Cargo.toml          # Dependencies and metadata
└── config.json         # Message mappings (used for future log monitoring)
```

## Implementation Notes

### Async Architecture

- **Synthesis serialization**: Mutex ensures only one synthesis at a time (espeak-ng thread-safety)
- **Concurrent playback**: Up to 3 audio streams can play simultaneously (semaphore limit)
- **Blocking operations**: Both synthesis and playback use `tokio::task::spawn_blocking`
- **Error handling**: All errors use `anyhow::Result` with `.context()` for clear error chains

### Threading Model

```
Main Async Task (Tokio)
  ├─> spawn_blocking (Synthesis) ─> Mutex lock ─> Piper TTS ─> espeak-ng
  └─> spawn_blocking (Playback)  ─> Rodio sink ─> Audio device
```

- Synthesis tasks queue at the mutex (serialized)
- Playback tasks run concurrently (up to 1 at a time)
- Main async task continues immediately after spawning

### Performance Considerations

- **Semaphore limit**: `MAX_CONCURRENT_ANNOUNCEMENTS = 1` prevents audio overlap chaos
- **Memory per task**: ~200KB audio samples, max 3 concurrent = ~600KB
- **Mutex overhead**: Minimal, only held during synthesis (~100-500ms per message)
