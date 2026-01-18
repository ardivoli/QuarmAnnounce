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

5. **Configuration Module** (`src/config.rs`): Type-safe configuration system
   - `MessageConfig` enum with two variants:
     - `Simple`: Immediate announcements when pattern matches
     - `TimedDelay`: Delayed announcements with configurable timer
   - `Config` struct with `game_directory` and `messages` Vec
   - `Config::load()` async method for JSON deserialization
   - Serde-based type-tagged JSON format

6. **Log Monitor (LogMonitor)**: Monitors EverQuest log files and triggers announcements
   - Tails log file using async `BufReader` with line-by-line reading
   - Implements batch collection with timeout to deduplicate rapid announcements
   - Uses `HashSet<String>` to ensure only one announcement per unique message type per batch
   - Batching timeout: 10ms (catches immediately available lines without delaying single-line announcements)
   - Idle retry delay: 50ms (when EOF is reached)
   - **Timer Management**: Tracks active timers with `Arc<Mutex<HashMap<String, JoinHandle<()>>>>` for timed delay announcements
   - **Debounce Behavior**: Re-triggering the same pattern resets the timer (cancels existing, starts new)

### Current Implementation State

**Code Organization:**
- `src/audio.rs`: Audio synthesis and playback module (TtsEngine, constants)
- `src/config.rs`: Configuration types and loading (Config, MessageConfig enum)
- `src/log_monitor.rs`: Log file monitoring with batching, deduplication, and timer management (LogMonitor)
- `src/main.rs`: Application entry point, initialization

**Implemented features:**
- ✅ Async TTS engine with Tokio runtime
- ✅ Non-blocking audio synthesis and playback
- ✅ Pipelined synthesis (next announcement can synthesize while current one plays)
- ✅ Thread-safe synthesizer access via mutex
- ✅ Proper error handling with `anyhow::Result` and context
- ✅ Modular code structure with separate audio, config, and log monitoring modules
- ✅ Type-safe configuration with Simple and TimedDelay message types
- ✅ Timed delay announcements with configurable delays (e.g., "charm about to break" 28s after "Charm spell has taken hold")
- ✅ Timer debounce behavior (re-triggering resets timer)
- ✅ Batch deduplication of announcements (prevents duplicate announcements in rapid succession)
- ✅ Comprehensive unit tests with generic AsyncBufRead pattern covering batch processing, deduplication, timers, and audio engine

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

See `Cargo.toml` for current versions. Core dependencies:

- `tokio`: Async runtime with full features
- `piper-rs`: Neural TTS synthesis
- `ort` / `ort-sys`: ONNX Runtime for ML inference
- `rodio`: Audio playback
- `anyhow`: Error handling with context
- `serde` / `serde_json`: JSON config parsing

## Project Structure

```
quarm_announce/
├── src/
│   ├── main.rs          # Application entry point, initialization
│   ├── config.rs        # Configuration types (Config, MessageConfig enum) and loading
│   ├── log_monitor.rs   # Log file monitoring with batching, deduplication, and timers
│   └── audio.rs         # Audio synthesis and playback (TtsEngine, constants)
├── speakers/            # Piper ONNX models and configs
│   └── en_US-amy-medium.onnx.json
├── Cargo.toml          # Dependencies and metadata
└── config.json         # Message configurations (Simple and TimedDelay types)
```

## Implementation Notes

### Async Architecture

- **Synthesis serialization**: Mutex ensures only one synthesis at a time (espeak-ng thread-safety)
- **Playback gating**: Semaphore (limit=1, hardcoded in `audio.rs:54`) ensures only one playback at a time to prevent audio overlap
- **Pipelined operations**: Semaphore is acquired AFTER synthesis, allowing next announcement to synthesize during current playback (synthesis time ~100-500ms overlaps with previous playback)
- **Blocking operations**: Both synthesis and playback use `tokio::task::spawn_blocking`
- **Error handling**: All errors use `anyhow::Result` with `.context()` for clear error chains

### Threading Model

```
Announcement 1:  [Synth1 (mutex)] ────> [Play1 (semaphore)]
Announcement 2:                [Synth2 (mutex)] ────> [Play2 (semaphore)]
                                    ^
                                    └─ Starts during Play1
```

Synthesis is serialized (mutex), playback is gated (semaphore, limit=1). Key optimization: semaphore acquired after synthesis, enabling pipelined execution.

### Batching and Deduplication

The log monitor uses intelligent batching to prevent announcement spam when multiple identical messages appear rapidly (common in EverQuest when buffs/debuffs expire simultaneously). It collects lines in a 10ms window and uses a `HashSet<String>` to spawn ONE task per unique message. For example: 5 "charm spell has worn off" lines → 1 announcement, but 5 charm + 1 root → 2 distinct announcements. See `log_monitor.rs` for timeout constants.

### Configuration Format

The `config.json` file uses a type-tagged format with two message types:

**Simple Messages** - Immediate announcements:
```json
{
  "type": "simple",
  "pattern": "charm spell has worn off",
  "announcement": "charm break"
}
```

**TimedDelay Messages** - Delayed announcements with debounce:
```json
{
  "type": "timed_delay",
  "pattern": "Charm spell has taken hold",
  "announcement": "charm about to break",
  "timer_delay_in_seconds": 28
}
```

### Timer Behavior

- **Debounce**: If the same pattern is detected again before the timer fires, the existing timer is cancelled and a new one starts
- **Example**: "Charm spell has taken hold" detected at T+0s and T+20s → only one announcement at T+48s (28s after the second trigger)
- **Implementation**: Timers tracked in `Arc<Mutex<HashMap<String, JoinHandle<()>>>>` keyed by pattern
- **Use case**: Warn player before charm breaks (EQ charm lasts ~30s, announce at 28s to give time to react)

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
- Deduplication, batch processing, pattern matching, EOF handling
- TimedDelay message batching and collection
- Mixed Simple and TimedDelay message handling

**Audio Engine Tests** (`src/audio.rs`):
- Engine initialization, concurrent announcements, semaphore behavior, text handling

### Running Tests

```bash
# Run all tests
cargo test

# Run log monitor tests only
cargo test log_monitor

# Run audio tests only
cargo test audio

# Run with output to see println! statements
cargo test -- --nocapture
```

**Note:** Tests require the Piper TTS model file to exist at `./speakers/en_US-amy-medium.onnx.json` for TtsEngine initialization tests and mock creation.
