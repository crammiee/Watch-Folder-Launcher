# Watch Folder Launcher

Watches a folder and automatically launches any app when a matching file is dropped in — without keeping the app open in the background.

Runs as a **system tray app** on Windows: green circle = watching, grey = paused. Right-click to toggle or quit. Idle memory: ~3–5 MB.

---

## Quickstart

1. Go to [Releases](../../releases/latest) and download `ame-watcher.exe` and `config.json`
2. Put both files in the same folder, e.g. `C:\Tools\WatchLauncher\`
3. Edit `config.json` in Notepad — set your watch folder and app path (see below)
4. Double-click `ame-watcher.exe` — it appears in the system tray (bottom-right)

---

## config.json

```json
{
  "watchFolder": "C:\\Watch",
  "debounceMs": 2000,
  "fileExtensions": [".mp4", ".mov", ".mxf", ".r3d", ".braw", ".ari", ".arri", ".avi", ".mkv", ".mts", ".m2ts", ".mpg", ".mpeg", ".wmv"],
  "appDisplayName": "Adobe Media Encoder",
  "launchCommand": "C:\\Program Files\\Adobe\\Adobe Media Encoder 2024\\Adobe Media Encoder.exe",
  "launchArgs": [],
  "processPattern": "Adobe Media Encoder.exe",
  "logFile": "C:\\Watch\\watcher.log"
}
```

| Field | Description |
|---|---|
| `watchFolder` | Folder to watch |
| `debounceMs` | How long to wait after the last file event before acting (default: 2000ms) |
| `fileExtensions` | Only these file types trigger a launch |
| `appDisplayName` | Name shown in the tray tooltip and log |
| `launchCommand` | Full path to the `.exe` to launch |
| `launchArgs` | Extra arguments passed to the exe — leave as `[]` if none |
| `processPattern` | `.exe` name used to check if the app is already running. Set to `""` to skip the check. |
| `logFile` | Path to a log file. Recommended — there's no console window in the default build. |

### Example: DaVinci Resolve

```json
{
  "appDisplayName": "DaVinci Resolve",
  "launchCommand": "C:\\Program Files\\Blackmagic Design\\DaVinci Resolve\\Resolve.exe",
  "processPattern": "Resolve.exe"
}
```

---

## Tray icon

| Icon | Meaning |
|---|---|
| Green circle | Watching — will launch the app when a file is detected |
| Grey circle | Paused — no launches until resumed |

Right-click menu:
- **Pause / Resume Watching**
- **Quit**

---

## Running multiple instances

Each instance is a separate exe process with its own tray icon. Use the `--config` flag to point each one at a different config file.

**Folder layout:**

```
C:\Tools\WatchLauncher\
  ame-watcher.exe
  config-ame.json
  config-resolve.json
```

**`config-ame.json`** — watches one folder, launches AME:

```json
{
  "watchFolder": "C:\\Watch\\AME",
  "appDisplayName": "Adobe Media Encoder",
  "launchCommand": "C:\\Program Files\\Adobe\\Adobe Media Encoder 2024\\Adobe Media Encoder.exe",
  "processPattern": "Adobe Media Encoder.exe",
  "logFile": "C:\\Watch\\AME\\watcher.log"
}
```

**`config-resolve.json`** — watches a different folder, launches Resolve:

```json
{
  "watchFolder": "C:\\Watch\\Resolve",
  "appDisplayName": "DaVinci Resolve",
  "launchCommand": "C:\\Program Files\\Blackmagic Design\\DaVinci Resolve\\Resolve.exe",
  "processPattern": "Resolve.exe",
  "logFile": "C:\\Watch\\Resolve\\watcher.log"
}
```

Run both at once:

```bat
ame-watcher.exe --config config-ame.json
ame-watcher.exe --config config-resolve.json
```

Each appears as its own tray icon. To auto-start both on login, create one Task Scheduler task per instance (see below), with different task names and `--config` arguments.

---

## Auto-start on login (Task Scheduler)

Repeat these steps once per instance, giving each task a unique name.

**Task 1 — AME watcher**

1. Open **Task Scheduler** → *Create Basic Task*
2. **Name:** `WatchLauncher - AME`
3. **Trigger:** At log on
4. **Action:** Start a program
   - Program/script: `C:\Tools\WatchLauncher\ame-watcher.exe`
   - Add arguments: `--config C:\Tools\WatchLauncher\config-ame.json`
   - Start in: `C:\Tools\WatchLauncher\`
5. **Settings tab:** Check *If the task is already running, do not start a new instance*
6. **General tab:** Check *Run only when user is logged on*

**Task 2 — Resolve watcher**

Repeat all the same steps with:
- **Name:** `WatchLauncher - Resolve`
- **Add arguments:** `--config C:\Tools\WatchLauncher\config-resolve.json`

Everything else is identical.

To test without rebooting: right-click each task → *Run*. You should see two tray icons appear.

---

## Build from source

Requires [Rust](https://rustup.rs).

```bat
cargo build --release
```

Binary: `target\release\ame-watcher.exe`

To keep a console window visible (useful while configuring):

```bat
cargo build --release --features console
```

### Cross-compile from WSL (produces a Windows .exe)

```bash
./build-windows.sh
```

Output is placed in `dist/` — ready to upload to GitHub Releases.

### Development/testing on WSL

```bash
cargo build
./target/debug/ame-watcher --dry-run
```
