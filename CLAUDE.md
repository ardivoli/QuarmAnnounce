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
   - Only 1 concurrent playback allowed (controlled by semaphore)

4. **TtsEngine Struct**: Coordinates synthesis and playback
   - `synthesizer`: `Arc<Mutex<PiperSpeechSynthesizer>>` for thread-safe sharing
   - `audio_semaphore`: `Arc<Semaphore>` limits concurrent playback (not synthesis)
   - `announce()`: Async method that synthesizes and plays TTS with pipelined operations

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
- ✅ Pipelined synthesis (next announcement can synthesize while current one plays)
- ✅ Thread-safe synthesizer access via mutex
- ✅ Proper error handling with `anyhow::Result` and context
- ✅ Modular code structure with separate audio module
- ✅ Config loading and log monitoring (fully integrated)

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
- **Playback gating**: Semaphore (limit=1) ensures only one playback at a time to prevent audio overlap
- **Pipelined operations**: Semaphore is acquired AFTER synthesis, allowing next announcement to synthesize during current playback
- **Blocking operations**: Both synthesis and playback use `tokio::task::spawn_blocking`
- **Error handling**: All errors use `anyhow::Result` with `.context()` for clear error chains

### Threading Model

```
Announcement 1:  [Synth1 (mutex)] ────> [Play1 (semaphore)]
Announcement 2:                [Synth2 (mutex)] ────> [Play2 (semaphore)]
                                    ^
                                    └─ Starts during Play1
```

- Synthesis tasks queue at the mutex (serialized by espeak-ng thread-safety requirement)
- Playback is gated by semaphore (limit=1) to prevent audio overlap
- **Key optimization**: Semaphore acquired after synthesis completes, enabling pipelined execution
- Next announcement can synthesize while current one plays, reducing latency for queued messages

### Performance Considerations

- **Semaphore limit**: Hardcoded to 1 in `audio.rs:54` to prevent audio overlap
- **Pipelining benefit**: Synthesis time (100-500ms) overlaps with previous playback, reducing wait time
- **Memory per task**: ~200KB audio samples, max 1 concurrent playback = ~200KB active memory
- **Mutex overhead**: Minimal, only held during synthesis (~100-500ms per message)
