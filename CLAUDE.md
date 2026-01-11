# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Agent Instructions

Read AGENTS.md for VERY IMPORTANT instructions around task planning, tracking, and completion.

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

6. **Log Monitor (LogMonitor)**: Monitors EverQuest log files and triggers announcements
   - Tails log file using async `BufReader` with line-by-line reading
   - Implements batch collection with timeout to deduplicate rapid announcements
   - Uses `HashSet<String>` to ensure only one announcement per unique message type per batch
   - Batching timeout: 10ms (catches immediately available lines without delaying single-line announcements)
   - Idle retry delay: 50ms (when EOF is reached)

### Current Implementation State

**Code Organization:**
- `src/audio.rs`: Audio synthesis and playback module (TtsEngine, constants)
- `src/log_monitor.rs`: Log file monitoring with batching and deduplication (LogMonitor)
- `src/main.rs`: Application entry point, config loading, initialization

**Implemented features:**
- ✅ Async TTS engine with Tokio runtime
- ✅ Non-blocking audio synthesis and playback
- ✅ Pipelined synthesis (next announcement can synthesize while current one plays)
- ✅ Thread-safe synthesizer access via mutex
- ✅ Proper error handling with `anyhow::Result` and context
- ✅ Modular code structure with separate audio and log monitoring modules
- ✅ Config loading and log monitoring (fully integrated)
- ✅ Batch deduplication of announcements (prevents duplicate announcements in rapid succession)
- ✅ Comprehensive unit tests with generic AsyncBufRead pattern (14 tests covering batch processing, deduplication, and audio engine)

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

# Run tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_deduplicates_identical_messages
```

## Key Dependencies

- `tokio` (1.48): Async runtime with full features
- `piper-rs` (0.1.9): Neural TTS synthesis
- `ort` (2.0.0-rc.9): ONNX Runtime for ML inference
- `rodio` (0.21.1): Audio playback
- `anyhow` (1.0): Error handling with context
- `serde` / `serde_json` (1.0): JSON config parsing

## Project Structure

```
quarm_announce/
├── src/
│   ├── main.rs          # Application entry point, config loading, initialization
│   ├── log_monitor.rs   # Log file monitoring with batching and deduplication
│   └── audio.rs         # Audio synthesis and playback (TtsEngine, constants)
├── speakers/            # Piper ONNX models and configs
│   └── en_US-amy-medium.onnx.json
├── plans/               # Implementation plans and task breakdowns
│   └── 2025-11-07_async-audio.md
├── Cargo.toml          # Dependencies and metadata
└── config.json         # Message mappings (log patterns -> announcements)
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

### Batching and Deduplication

The log monitor implements intelligent batching to prevent announcement spam when multiple identical messages appear in rapid succession (common in EverQuest when multiple buffs/debuffs expire simultaneously).

**How it works:**
1. **First line read**: Immediately reads the first available line from the log file
2. **Batch collection window**: After finding data, enters a 10ms timeout loop to collect all immediately available lines
3. **Deduplication**: Uses `HashSet<String>` to track unique announcement messages
4. **Task spawning**: After batch collection completes, spawns ONE task per unique message type

**Example behavior:**
- 5 "charm spell has worn off" lines → 1 "charm break" announcement
- 5 "charm" + 1 "root spell has worn off" → 2 announcements (charm + root)

**Timeout constants** (in `log_monitor.rs`):
- `BATCH_READ_TIMEOUT` (10ms): Short enough to not delay single-line announcements, long enough to catch bursts
- `IDLE_RETRY_DELAY` (50ms): Wait time when no data is available (EOF reached)

**Key benefit**: Dramatically reduces audio spam during buff/debuff cascades while preserving distinct message types

### Performance Considerations

- **Semaphore limit**: Hardcoded to 1 in `audio.rs:54` to prevent audio overlap
- **Pipelining benefit**: Synthesis time (100-500ms) overlaps with previous playback, reducing wait time
- **Memory per task**: ~200KB audio samples, max 1 concurrent playback = ~200KB active memory
- **Mutex overhead**: Minimal, only held during synthesis (~100-500ms per message)

## Testing

The project has comprehensive unit tests for both audio and log monitoring components.

### Testing Strategy

**Generic AsyncBufRead Pattern:**
- `process_log_lines()` and `process_one_batch()` are generic over `AsyncBufReadExt + Unpin`
- This allows testing with in-memory data (`BufReader<&[u8]>`) instead of real files
- Tests are fast, deterministic, and don't require file I/O

**Mock TTS Engine:**
- `TtsEngine::new_mock()` provides a test-only constructor (requires model file to exist)
- Audio playback is mocked in test builds using `#[cfg(test)]` conditional compilation
- Tests run silently and in parallel without audio device contention

### Test Coverage

**Log Monitor Tests** (`src/log_monitor.rs`):
- ✅ Deduplication of identical messages (5 charm lines → 1 announcement)
- ✅ Preservation of different message types (charm + root → 2 announcements)
- ✅ Single line announcements (no batching overhead)
- ✅ Non-matching lines (empty result set)
- ✅ EOF handling (returns None)
- ✅ Mixed matches and non-matches with deduplication
- ✅ `match_message()` pattern matching logic

**Audio Engine Tests** (`src/audio.rs`):
- ✅ Engine initialization (valid and invalid model paths)
- ✅ Single and concurrent announcements
- ✅ Semaphore limiting behavior
- ✅ Engine cloning for multi-task usage
- ✅ Text handling (empty, special characters)

### Running Tests

```bash
# Run all tests (14 total)
cargo test

# Run log monitor tests only
cargo test log_monitor

# Run audio tests only
cargo test audio

# Run with output to see println! statements
cargo test -- --nocapture
```

**Note:** Tests require the Piper TTS model file to exist at `./speakers/en_US-amy-medium.onnx.json` for TtsEngine initialization tests and mock creation.
