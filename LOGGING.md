# Soul Player Logging

## Overview

Soul Player supports optional file logging for debugging and troubleshooting. Logs are saved in the same directory as the database.

## Log Location

**Windows:** `%APPDATA%\Soul Player\logs\`
**macOS:** `~/Library/Application Support/soul-player/logs/`
**Linux:** `~/.config/soul-player/logs/`

## Log Files

Logs are automatically rotated daily with the format:
- `soul-player.log.YYYY-MM-DD` (e.g., `soul-player.log.2026-01-12`)

Old log files are kept, so you may want to periodically clean up the logs directory.

## Enabling File Logging

### From Project Root

```bash
# Development with file logging
yarn dev:desktop:logs

# Regular development (console only)
yarn dev:desktop
```

### From Desktop App Directory

```bash
cd applications/desktop

# Development with file logging
yarn tauri:dev:logs

# Regular development (console only)
yarn tauri:dev
```

### Direct Binary Execution

```bash
# Pass --logs flag to any Soul Player binary
./soul-player-desktop --logs

# Or with cargo run (development)
cargo run -- --logs
```

### How the Flag Works

The `--logs` flag is passed through multiple layers:
- `yarn dev:desktop:logs` → `tauri dev` → `cargo run` → `soul-player-desktop binary`
- Each layer uses `--` to pass arguments through to the next
- Final command: `cargo run -- --logs` (the `--` tells cargo to pass `--logs` to the binary)

## What Gets Logged

- **Library scanning progress** - `[SCAN]` prefix
- **Metadata extraction** - File paths being processed
- **Hash calculation** - File hashing for duplicate detection
- **Import operations** - Track imports, artist/album matching
- **Errors and warnings** - Any issues encountered
- **Audio device initialization** - Audio backend setup
- **Database operations** - Query execution (in debug mode)

## Log Levels

By default:
- `info` level for most components
- `debug` level for `soul_importer` (scanning/import details)

### Custom Log Levels

Set the `RUST_LOG` environment variable:

```bash
# Windows PowerShell
$env:RUST_LOG="debug"
yarn dev:desktop:logs

# macOS/Linux
RUST_LOG=debug yarn dev:desktop:logs

# More granular control
RUST_LOG="info,soul_importer=trace,soul_storage=debug" yarn dev:desktop:logs
```

## Common Use Cases

### Debugging Scan Hangs

```bash
yarn dev:desktop:logs
```

Look for the last `[SCAN] Processing:` line to identify which file caused the hang.

### Tracking Import Issues

```bash
RUST_LOG="soul_importer=debug" yarn dev:desktop:logs
```

Shows detailed metadata extraction and fuzzy matching for artists/albums.

### Finding Corrupted Files

Search log files for:
- `TIMEOUT on file:` - Files that took too long to process
- `Corrupted encoding detected` - Files with bad ID3 tags
- `Failed to process file` - General processing errors

## Log Format

File logs use plain text format (no ANSI colors):

```
2026-01-12T15:30:45.123Z INFO soul_importer: Starting library scan
2026-01-12T15:30:45.234Z DEBUG soul_importer: Processing file: D:\music\track.mp3
2026-01-12T15:30:45.567Z WARN soul_importer: Corrupted encoding detected (contains '?')
```

Console logs include colors and timestamps for better readability during development.

## Performance Impact

File logging has minimal performance impact:
- Writes are non-blocking (buffered)
- Logs are written to a background thread
- No impact on audio playback
- Negligible disk I/O overhead

## Troubleshooting

### Logs Directory Not Created

If the logs directory doesn't exist:
1. Check that the app has write permissions to the data directory
2. Logs will fall back to console-only mode
3. Check console output for `[LOGGING] File logging enabled:` message

### Large Log Files

Daily rotation means each day gets a new file. To clean up:

```bash
# Windows PowerShell
Remove-Item "$env:APPDATA\Soul Player\logs\*.log.*" -Force

# macOS/Linux
rm ~/Library/Application\ Support/soul-player/logs/*.log.*
```

### Missing Logs

If logs aren't appearing:
1. Ensure you're using `--logs` flag or `dev:logs` script
2. Check console for `[LOGGING] File logging enabled:` confirmation
3. Verify the logs directory exists and is writable

## Development Tips

**Filter scan logs only:**
```bash
cd applications/desktop
yarn tauri:dev:logs 2>&1 | grep "\[SCAN\]"
```

**Follow logs in real-time:**
```bash
# Windows PowerShell
Get-Content "$env:APPDATA\Soul Player\logs\soul-player.log.*" -Wait -Tail 50

# macOS/Linux
tail -f ~/Library/Application\ Support/soul-player/logs/soul-player.log.*
```

**Search for errors:**
```bash
# Windows PowerShell
Select-String -Path "$env:APPDATA\Soul Player\logs\*.log.*" -Pattern "ERROR|WARN"

# macOS/Linux
grep -r "ERROR\|WARN" ~/Library/Application\ Support/soul-player/logs/
```
