# Seek Performance Logging - Quick Start

## TL;DR - Get Logging NOW

### 1. Rebuild
```bash
cargo xtask clean dev && cargo xtask dev desktop
```

### 2. Run with Backend Logging
```bash
# macOS/Linux
RUST_LOG=soul_playback=info cargo xtask dev desktop

# Windows PowerShell
$env:RUST_LOG="soul_playback=info"
cargo xtask dev desktop
```

### 3. Test Seek + Check Logs

**Frontend logs** (always visible):
- Open DevTools: `F12` or `Cmd+Option+I`
- Click **Console** tab
- Perform a seek action
- Look for messages starting with `[SEEK PERF]`

**Backend logs** (with RUST_LOG=info):
- Same console where you ran `cargo xtask dev desktop`
- Search for lines with `[SEEK PERF]`

## What You'll See

### Frontend (DevTools Console)
```
[SEEK PERF] ===== SEEK START ===== at 1234.56ms
[SEEK PERF] Store update: 0.50ms
[SEEK PERF] invoke('seek_to') completed in 3.45ms (total: 4.47ms)
[SEEK PERF] ===== SEEK END ===== (total time: ~130ms)
```

### Backend (Terminal/Console)
```
[SEEK PERF] === Rust seek_to() ENTRY === position=12.345
[SEEK PERF] === Manager.seek_to() ENTRY === position=12.345s
[SEEK PERF] Decoder.seek() completed in 5.32ms
[SEEK PERF] === Manager.seek_to() EXIT === completed in 5.47ms
[SEEK PERF] === Rust seek_to() EXIT === completed in 5.89ms
```

## Key Timings to Check

| What | Time | Good? |
|------|------|-------|
| Store update | 0-1ms | ✓ (React) |
| Invoke overhead | 0-5ms | ✓ (IPC) |
| Decoder seek | 1-50ms | ✓ (depends on format) |
| Total backend | 5-60ms | ✓ |
| Position update | ~100ms | ✓ (throttled) |

## Is 100ms Fix Working?

Look for this in logs:
```
[SEEK PERF] === Position Update EMIT === after 9823 samples (~100ms @ 48000Hz)
```

- Shows "~100ms" = Fix is compiled ✓
- Shows "~500ms" or other = Rebuild needed

## Enable TRACE Logging (More Details)

```bash
RUST_LOG=soul_playback=trace cargo xtask dev desktop
```

This also shows:
- Lock acquisition timing
- Position update timing
- Every helper function call

Good for deep debugging, creates more noise.

## Production Logging

Set environment variable once, then restart app:

**Windows (PowerShell as Admin)**:
```powershell
[Environment]::SetEnvironmentVariable("RUST_LOG", "soul_playback=info", [EnvironmentVariableTarget]::User)
```

Logs go to: `%APPDATA%\Soul Player\logs\`

**macOS/Linux**:
```bash
export RUST_LOG=soul_playback=info
# Then start Soul Player
```

Logs go to: `~/.config/soul-player/logs/`

## Interpret Results

### Good Seek (~130ms total)
```
Frontend: 4.47ms (click to backend call)
Backend: 5.89ms (Rust handling)
Decoder: 5.32ms (seek itself)
Ignore window: 120ms (hardcoded pause)
Total perceived: ~130ms ✓
```

### Slow Backend Seek
```
Decoder: 42.15ms  <-- MP3 VBR is slow
Total backend: 42.31ms
Ignore window: 120ms
Total perceived: ~165ms (expected for MP3)
```

### Lock Contention
```
[SEEK PERF] Lock acquired in 15.67ms  <-- SLOW!
This means audio thread is busy.
```

## Files Modified

All changes are in these 5 files:

1. `applications/shared/src/hooks/useSeekBar.ts` - Frontend timing
2. `applications/desktop/src/providers/TauriPlayerCommandsProvider.tsx` - IPC timing
3. `applications/desktop/src-tauri/src/main.rs` - Rust command handler
4. `applications/desktop/src-tauri/src/playback.rs` - PlaybackManager wrapper
5. `libraries/soul-playback/src/manager.rs` - Core seek logic (2 sections)

## Common Issues

### "I don't see [SEEK PERF] logs"
- [ ] Rebuild with `cargo xtask clean dev`
- [ ] Check DevTools console (F12)
- [ ] Set `RUST_LOG=soul_playback=info` for backend

### "Backend logs show ~500ms instead of ~100ms"
- [ ] Old code is running
- [ ] Rebuild: `cargo xtask clean dev && cargo xtask dev desktop`
- [ ] Check manager.rs line 1900: should have `/ 10`

### "Position bar jumps back after seek"
- [ ] Ignore window not working
- [ ] Check if `IGNORE_WINDOW_MS = 120` is logged
- [ ] Verify frontend logs show "Ignore window DISABLED"

## Next: Full Details

See `SEEK_PERFORMANCE_LOGGING_GUIDE.md` for comprehensive documentation.

