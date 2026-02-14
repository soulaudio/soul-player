# Seek Performance Logging Guide

This document explains how to enable comprehensive performance logging to measure actual seek latency at every step of the pipeline.

## Logging Points

Performance logs are added at 6 key checkpoints:

### 1. Frontend - useSeekBar.ts
- **What**: Frontend click timestamp, target position, optimistic UI update time
- **Log level**: `console.log()` (always visible)
- **Example**: `[SEEK PERF] ===== SEEK START ===== at 1234.56ms`

### 2. Frontend - TauriPlayerCommandsProvider.tsx
- **What**: Invoke timestamp, IGNORE_WINDOW_MS value, actual invoke() timing
- **Log level**: `console.log()` (always visible)
- **Example**: `[SEEK PERF] invoke('seek_to') completed in 1.23ms`

### 3. Backend - main.rs (seek_to command)
- **What**: Rust entry/exit timing for the Tauri command handler
- **Log level**: `tracing::info!()` (INFO level)
- **Example**: `[SEEK PERF] === Rust seek_to() ENTRY === position=12.345s`

### 3b. Backend - playback.rs (PlaybackManager wrapper)
- **What**: Lock acquisition timing and command send timing
- **Log level**: `tracing::trace!()` (TRACE level)
- **Example**: `[SEEK PERF] Lock acquired in 0.15ms`

### 4. Backend - manager.rs (actual seek logic)
- **What**: Manager entry, decoder seek call timing, manager exit
- **Log level**: `tracing::info!()` (INFO level)
- **Example**: `[SEEK PERF] === Manager.seek_to() EXIT === completed in 5.67ms`

### 6. Backend - Position Update Emission
- **What**: When position updates are emitted (every 100ms)
- **Log level**: `tracing::trace!()` (TRACE level)
- **Example**: `[SEEK PERF] === Position Update EMIT === after 9823 samples`

## Enabling Logging

### Frontend Console Logs
Frontend logs use `console.log()` which are **always visible** in the browser DevTools console. No configuration needed.

**To view**:
1. Open Soul Player desktop app
2. Right-click → Inspect Element (or press F12)
3. Go to Console tab
4. Perform a seek action
5. Look for `[SEEK PERF]` messages

### Backend Rust Logs

Backend logs use the `tracing` crate with levels: ERROR, INFO, DEBUG, TRACE

#### Enable TRACE logging (most detailed)

**Development (macOS/Linux)**:
```bash
RUST_LOG=soul_playback=trace cargo xtask dev desktop
```

**Development (Windows PowerShell)**:
```powershell
$env:RUST_LOG="soul_playback=trace"
cargo xtask dev desktop
```

**Production app (Windows)**:
1. Stop Soul Player
2. Set environment variable:
   ```powershell
   [Environment]::SetEnvironmentVariable("RUST_LOG", "soul_playback=trace", [EnvironmentVariableTarget]::User)
   ```
3. Restart Soul Player
4. Check logs at: `%APPDATA%\Soul Player\logs\`

#### Enable INFO logging (less noise, core timing only)

```bash
RUST_LOG=soul_playback=info cargo xtask dev desktop
```

This shows:
- Manager entry/exit
- Decoder seek timing
- Rust command entry/exit

But NOT lock timing or position update details.

## Reading Log Output

### Complete Seek Timeline

A successful seek should produce logs in this order:

**Frontend (Console)**:
```
[SEEK PERF] ===== SEEK START ===== at 1234.56ms
[SEEK PERF] Target position: 12.345s (clamped from 12.345s)
[SEEK PERF] Store update: 0.50ms (UI now shows 45.5%)
[SEEK PERF] Invoking backend seek_to at 1.02ms
[SEEK PERF] invoke('seek_to') completed in 3.45ms (total: 4.47ms)
[SEEK PERF] Finalizing seek state in 120ms (already elapsed: 5.01ms)
[SEEK PERF] ===== SEEK END ===== (total time: ~130ms)
```

**Backend (Rust logs with TRACE level)**:
```
[SEEK PERF] === Rust seek_to() ENTRY === position=12.345
[SEEK PERF] PlaybackManager.seek() ENTRY position=12.345
[SEEK PERF] Lock acquired in 0.15ms
[SEEK PERF] === Manager.seek_to() ENTRY === position=12.345s
[SEEK PERF] Decoder.seek() completed in 5.32ms
[SEEK PERF] === Manager.seek_to() EXIT === completed in 5.47ms
[SEEK PERF] PlaybackManager.seek() EXIT - command sent in 0.08ms
[SEEK PERF] === Rust seek_to() EXIT === completed in 5.89ms
```

**Position update (appears ~100ms after seek)**:
```
[SEEK PERF] === Position Update EMIT === after 9823 samples (~100ms @ 48000Hz)
```

### What Each Timing Means

| Metric | What It Measures | Acceptable Range | Issue If |
|--------|------------------|------------------|----------|
| Store update time | React store setState() call | <1ms | >5ms = React performance issue |
| Frontend → Invoke | Time between click and invoke() call | <2ms | >5ms = JavaScript lag |
| Invoke duration | Time spent in invoke('seek_to') | <5ms | >20ms = IPC overhead issue |
| Rust seek_to total | Total time in Tauri command handler | <10ms | >20ms = Backend bottleneck |
| Lock acquisition | Time to acquire PlaybackManager mutex | <1ms | >5ms = Lock contention |
| Manager.seek_to total | Time in core seek logic | <8ms | >20ms = Decoder slow |
| Decoder.seek | Actual decoder seek call | 5-50ms | >100ms = Decoder issue (MP3 VBR normal) |
| Frontend to position update | Time from seek click to first position update | 100-130ms | >200ms = Ignore window or update threshold wrong |

## Debugging Workflow

### Problem: "Seek feels laggy (>300ms)"

1. **Check frontend logs** - What is the "SEEK END" total time?
   - If <130ms: Problem is elsewhere (UI rendering, playback state display)
   - If >300ms: One of the steps is slow

2. **Identify slow step** - Compare timestamps:
   - Store update slow? → React rendering issue
   - Invoke slow? → IPC overhead (unusual)
   - Rust command slow? → Backend issue

3. **Check backend logs** - Enable TRACE logging and look for:
   - `Decoder.seek()` time > 50ms on FLAC = symphonia issue
   - Lock acquisition > 5ms = audio thread contention
   - Manager entry to exit > 20ms = crossfade/stop-fade cancellation slow

4. **Verify ignore window** - Check if:
   - `IGNORE_WINDOW_MS = 120` is logged from TauriPlayerCommandsProvider
   - Ignore window disabled in logs appears ~120ms after enabled

### Problem: "Position bar jumps back after seek"

This means the ignore window isn't working. Check:

1. **Frontend logs**: Is "Ignore window disabled" message appearing?
2. **Backend logs**: Are position updates being emitted during seek?
   - If yes: Frontend ignore window not activated
   - If no: Good (ignore window is working)

### Problem: "100ms fix not applied"

Check manager.rs logs:
```
[SEEK PERF] === Position Update EMIT === after 9823 samples (~100ms @ 48000Hz)
```

- If showing "~100ms": Fix is applied ✓
- If showing "~500ms" or larger: Old code still running ✗

Verify the division is present in the code:
```rust
let threshold = (self.sample_rate as usize * 2) / 10;  // This /10 gives 100ms
```

Should NOT be:
```rust
let threshold = (self.sample_rate as usize * 2) / 2;   // This /2 gives 500ms
```

## Viewing Log Files

Logs are saved to:

- **Windows**: `%APPDATA%\Soul Player\logs\`
- **macOS**: `~/Library/Application Support/soul-player/logs/`
- **Linux**: `~/.config/soul-player/logs/`

Open the latest log file with a text editor and search for `[SEEK PERF]`.

## Log Output Examples

### Good Seek (FLAC, instant seek)
```
[SEEK PERF] === Manager.seek_to() ENTRY === position=Duration { secs: 12, nanos: 345000000 }
[SEEK PERF] Decoder.seek() completed in 1.23ms
[SEEK PERF] === Manager.seek_to() EXIT === completed in 1.45ms
```

### Slow Seek (MP3 VBR, expected)
```
[SEEK PERF] === Manager.seek_to() ENTRY === position=Duration { secs: 45, nanos: 678000000 }
[SEEK PERF] Decoder.seek() completed in 42.15ms
[SEEK PERF] === Manager.seek_to() EXIT === completed in 42.31ms
```

### Lock Contention Issue
```
[SEEK PERF] PlaybackManager.seek() ENTRY position=12.345
[SEEK PERF] Lock acquired in 15.67ms  <-- SLOW! Audio thread is busy
[SEEK PERF] PlaybackManager.seek() EXIT
```

### Crossfade Cancellation (during crossfade)
```
[SEEK PERF] === Manager.seek_to() ENTRY === position=45.678
[SEEK PERF] Cancelling active crossfade due to seek (took 0.12ms so far)
[SEEK PERF] Decoder.seek() completed in 8.34ms
```

## Performance Targets

### Per-Step Targets

- Frontend click to store update: **<1ms**
- Store update to backend invoke: **<1ms**
- Backend invoke() call: **<5ms**
- Manager lock acquisition: **<1ms**
- Decoder seek (FLAC): **<5ms**
- Decoder seek (MP3 VBR): **<50ms**
- Total backend execution: **<50ms** (FLAC) or **<100ms** (MP3)

### End-to-End Targets

- Optimistic UI update (store → visual): **Immediate** (within same frame)
- Total to ignore window end: **~130ms** (optimistic + backend + ignore window)
- Perceived latency: **~120ms** (ignore window hides backend + crossfade)

## Checking If 100ms Fix Is Compiled

Look for this exact line in logs with TRACE level enabled:

```
[SEEK PERF] === Position Update EMIT === after 9823 samples (~100ms @ 48000Hz)
```

If you see "~500ms" instead, the old code is running. Rebuild with:

```bash
cargo xtask build desktop --release
```

## Advanced: Capturing Full Logs

For detailed analysis, save logs to file:

**Development**:
```bash
RUST_LOG=soul_playback=trace cargo xtask dev desktop 2>&1 | tee seek_debug.log
```

**Production logs are automatically saved** to:
- Windows: `%APPDATA%\Soul Player\logs\soul-player-YYYY-MM-DD.log`
- macOS: `~/Library/Application Support/soul-player/logs/soul-player-YYYY-MM-DD.log`
- Linux: `~/.config/soul-player/logs/soul-player-YYYY-MM-DD.log`

## Interpreting Timing Variance

Seek timings will vary based on:

1. **File format**:
   - FLAC (byte-exact seeking): 1-10ms
   - ALAC (byte-exact seeking): 1-10ms
   - MP3 (VBR frame search): 20-50ms
   - Ogg Vorbis (seeking required): 10-30ms

2. **Disk speed**:
   - SSD: <5ms overhead
   - HDD: 10-50ms overhead
   - Network share: 50-200ms overhead

3. **System load**:
   - Idle: Consistent timing
   - Under load: Timing increases 5-20%

4. **Position**:
   - Early in file: Faster (less to skip)
   - Late in file: Slower (more to skip)
   - MP3 especially affected by this

## Troubleshooting Checklist

- [ ] Frontend console shows `[SEEK PERF]` messages
- [ ] Backend logs show `[SEEK PERF]` messages (enable with `RUST_LOG=soul_playback=info`)
- [ ] Store update time is <1ms
- [ ] Invoke time is <5ms
- [ ] Decoder seek is <50ms
- [ ] Position update appears ~100ms after seek
- [ ] Ignore window disabled ~120ms after enabled
- [ ] No error messages in any logs
- [ ] File format is not MP3 VBR (if seeking is slow)

## Next Steps

After collecting logs:

1. **Share timings** in an issue with format:
   ```
   Frontend: 4.47ms (click to invoke)
   Invoke: 3.45ms (overhead)
   Backend: 5.89ms (manager entry to exit)
   Decoder: 5.32ms (actual seek)
   Total: ~135ms (with ignore window)
   ```

2. **Identify bottleneck** - Which step is slowest?

3. **Optimize** - Focus on slowest path first

4. **Re-test** with same file to verify improvement

