# Quarm Announce

![Release Build](https://github.com/ardivoli/QuarmAnnounce/actions/workflows/release.yml/badge.svg)

This application reads a target EverQuest log file for configured messages and then announces (via text-to-speech) a mapped, corresponding message when a new message arrives that matches the configuration.

## Installation

Pre-built binaries are available for Windows and Linux on the [Releases page](https://github.com/ardivoli/QuarmAnnounce/releases).

### Windows

1. Download the latest `quarm_announce-v*-windows-x64.zip` from the [Releases page](https://github.com/ardivoli/QuarmAnnounce/releases)
2. Extract the archive to a location of your choice (e.g., `C:\Program Files\quarm_announce\`)
3. The extracted folder should contain:
   - `quarm_announce.exe` - The main application
   - `config.json` - Configuration file (see below for setup)
   - `speakers/` - TTS voice models directory
4. Edit `config.json` to configure your EverQuest log file path and message announcements
5. Run `quarm_announce.exe` to start the application

### Linux / SteamOS

1. Download the latest `quarm_announce-v*-linux-x64.tar.gz` from the [Releases page](https://github.com/ardivoli/QuarmAnnounce/releases)
2. Extract the archive:
   ```bash
   tar -xzf quarm_announce-v*-linux-x64.tar.gz -C ~/quarm_announce
   cd ~/quarm_announce
   ```
3. The extracted folder should contain:
   - `quarm_announce` - The main application (executable)
   - `config.json` - Configuration file (see below for setup)
   - `speakers/` - TTS voice models directory
4. Install required system dependencies (Ubuntu/Debian):
   ```bash
   sudo apt install libasound2
   ```
   For SteamOS, additional packages may be required. If you encounter issues, try:
   ```bash
   sudo steamos-readonly disable
   sudo pacman -S alsa-lib
   ```
5. Edit `config.json` to configure your EverQuest log file path and message announcements
6. Run the application:
   ```bash
   ./quarm_announce
   ```

## Example config

Default `config.json`:
```json
{
  "game_directory": "path/to/EverquestProjectQuarm",
  "message_announcements": {
    "Charm spell has worn off": "charm wore off",
    "Root spell has worn off": "root wore off",
    "Fetter spell has worn off": "fetter wore off"
  }
}
```

Next time a charm wears off, this application will output "charm worn off" audio.

## Development

1. This app is coded in Rust, so [install that](https://rust-lang.org/learn/get-started/).
2. Git hooks are controlled by [lefthook](https://github.com/evilmartians/lefthook) (you'll also need [homebrew/linux-brew](https://brew.sh/)): `brew install lefthook`.
3. We use [beads](https://github.com/steveyegge/beads) and [perles](https://github.com/zjrosen/perles) for AI issue tracking.

```console
git lfs pull
```

## Future: Named Pipes Support

See [docs/named-pipes-notes.md](docs/named-pipes-notes.md) for notes on potential Zeal named pipe integration for real-time EverQuest game state monitoring.
