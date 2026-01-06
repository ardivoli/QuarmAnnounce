# Quarm Announce

![Release Build](https://github.com/kaiby/quarm_announce/actions/workflows/release.yml/badge.svg)

This application reads a target EverQuest log file for configured messages and then announces (via text-to-speech) a mapped, corresponding message when a new message arrives that matches the configuration.

## Installation

Pre-built binaries are available for Windows and Linux on the [Releases page](https://github.com/kaiby/quarm_announce/releases).

### Windows

1. Download the latest `quarm_announce-v*-windows-x64.zip` from the [Releases page](https://github.com/kaiby/quarm_announce/releases)
2. Extract the archive to a location of your choice (e.g., `C:\Program Files\quarm_announce\`)
3. The extracted folder should contain:
   - `quarm_announce.exe` - The main application
   - `config.json` - Configuration file (see below for setup)
   - `speakers/` - TTS voice models directory
4. Edit `config.json` to configure your EverQuest log file path and message announcements
5. Run `quarm_announce.exe` to start the application

### Linux / SteamOS

1. Download the latest `quarm_announce-v*-linux-x64.tar.gz` from the [Releases page](https://github.com/kaiby/quarm_announce/releases)
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
  "log_file_path": "../eq_log_yourcharacter.log",
  "message_announcements": {
    "Charm spell has worn off": "charm wore off",
    "Root spell has worn off": "root wore off",
    "Fetter spell has worn off": "fetter wore off"
  }
}
```

Next time a charm wears off, this application will output "charm worn off" audio.

## Development

```console
git lfs pull
```

## Extra notes about named pipes (for future functionality)

How the Piping Works

Technology: Windows Named Pipes - a standard OS-level inter-process communication (IPC) mechanism.

Architecture:
1. Zeal (the server) creates a named pipe for each EverQuest process:
  - Pipe name format: \\.\pipe\zeal_{processId} (e.g., \\.\pipe\zeal_12345)
  - Writes UTF-8 encoded JSON messages continuously
2. Client applications connect as readers:
  - Use NamedPipeClientStream in C# (see ZealPipes.Services/Helpers/ZealPipeReader.cs:51)
  - Read from pipe in chunks (32KB buffer by default)
  - Parse streaming JSON messages

Message Format:
{
  "type": 0,              // 0=LogText, 1=Label, 2=Gauge, 3=Player, 4=PipeCmd, 5=Raid
  "data_len": 123,
  "character": "PlayerName",
  "data": "{...}"         // Type-specific JSON data
}

Can Non-C# Applications Connect?

YES! Absolutely. Named Pipes are a Windows OS primitive, not C#-specific. Any language with file/pipe I/O can connect:

Quick Examples:

Python:
with open(r'\\.\pipe\zeal_12345', 'rb') as pipe:
    data = pipe.read(32768)
    message = json.loads(data.decode('utf-8'))

C++:
HANDLE hPipe = CreateFile("\\\\.\\pipe\\zeal_12345",
                          GENERIC_READ, 0, NULL,
                          OPEN_EXISTING, 0, NULL);
ReadFile(hPipe, buffer, 32768, &bytesRead, NULL);

Node.js, Go, Rust, Java - all can connect using their respective pipe/file APIs.

Caveat:

- Windows-only (Named Pipes are Windows-specific)
- Linux/macOS have equivalent mechanisms (FIFOs/Unix domain sockets) but would need Zeal to support them

The protocol is completely language-agnostic - you just need to read UTF-8 text from a Windows named pipe and parse JSON!

> How do Named Pipes work? Is it outputting data to a text file then?

⏺ No, Named Pipes are not text files - they're purely in-memory communication channels managed by the operating system. The name is just an identifier, not
  a file path.

How Named Pipes Actually Work

Think of it like a pipe in real life:
- One program pours data into one end (write)
- Another program drinks from the other end (read)
- Nothing is stored permanently - data flows through and disappears when read

The Mechanics:

1. In-Memory Buffer
  - The OS allocates a kernel buffer (typically 4-64KB)
  - Data written by the server goes into this buffer
  - Data read by clients comes from this buffer
  - No disk I/O involved at all
2. The "Name" is Just a Namespace
\\.\pipe\zeal_12345
└─┬─┘ └─┬─┘ └──┬───┘
  │     │       └─ Unique identifier
  │     └───────── Pipe namespace
  └─────────────── Local computer
  - Not a file path - it's more like a "phone number" in the OS
  - Windows creates a virtual namespace at \\.\pipe\
  - You can't browse it like folders or see it in Explorer
3. OS-Managed Communication
Zeal (Writer)           Windows Kernel           Your App (Reader)
      │                       │                          │
      ├─ Write "Hello" ──────>│                          │
      │                       │ [Buffer: "Hello"]        │
      │                       │<────── Read request ─────┤
      │                       ├─────── "Hello" ─────────>│
      │                       │ [Buffer: empty]          │

Key Differences from Files:

| Named Pipes                 | Regular Files            |
|-----------------------------|--------------------------|
| In-memory only              | Stored on disk           |
| Data disappears when read   | Data persists            |
| Blocking/streaming reads    | Random access            |
| Inter-process communication | Data storage             |
| No seeking/rewinding        | Can seek to any position |

Why Call Them "Pipes"?

The name comes from Unix shells:
cat file.txt | grep "pattern" | sort
            ↑                 ↑
        These are pipes - output flows into next program

Named Pipes are the same concept, but with a name so unrelated processes can connect (not just chained commands).

What Actually Happens with Zeal:

1. Zeal calls Windows API:
CreateNamedPipe("zeal_12345", PIPE_ACCESS_OUTBOUND, ...)
  - OS allocates kernel memory buffer
  - Registers name in pipe namespace
2. Zeal writes data:
WriteFile(pipe, jsonBytes, ...)
  - Bytes go into kernel buffer
  - If buffer is full, Zeal blocks until space available
3. Your app reads:
ReadFile(pipe, buffer, ...)
  - OS copies bytes from kernel buffer to your buffer
  - Bytes are removed from pipe (consumed)
  - If empty, your app blocks until data available

Nothing touches the disk. It's all RAM-based, making it very fast for real-time data streaming like EverQuest game state!

The ~/.wine/dosdevices/pipe/ directory in Wine is used to implement Windows named pipes by leveraging the underlying Linux named pipe mechanism.
In Windows, named pipes are a common method for inter-process communication (IPC). The pipe/ directory serves as the location where Wine creates the corresponding Unix domain sockets or named pipes that the Windows programs, running under Wine, can interact with.
When a Windows application running in Wine attempts to open or create a named pipe (e.g., \\.\pipe\MyPipe), Wine translates this request, and a corresponding communication channel is managed through files within the ~/.wine/dosdevices/pipe/ directory in the Linux environment. This allows different Windows processes within the same Wine prefix, or even Linux processes (with careful implementation), to communicate with each other.
In summary, the pipe/ directory is crucial for applications that rely on Windows named pipes for their internal operations or for communicating with other processes.

Example of wine APPS using pipes to talk to each other: https://gist.github.com/coderofsalvation/3053260
