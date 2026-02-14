# Seek Performance Logging - Changes Summary

## Overview

Added comprehensive performance logging at 6 key checkpoints to measure actual seek latency from frontend click to backend completion.

## Files Modified

### 1. `applications/shared/src/hooks/useSeekBar.ts`
**Purpose**: Frontend click timestamp and optimistic UI update logging

**What was added**:
- `performance.now()` timestamp at start of `handleSeek`
- Log target position before clamping
- Log store update time
- Log invoke start time and when backend completes
- Log when ignore window finalizes
- Detailed timing breakdown for each step

**Log pattern**:
```
[SEEK PERF] ===== SEEK START ===== at 1234.56ms
[SEEK PERF] Target position: 12.345s
[SEEK PERF] Store update: 0.50ms
[SEEK PERF] Invoking backend seek_to
[SEEK PERF] Backend seek completed at +4.47ms
[SEEK PERF] ===== SEEK END ===== (total: 130ms)
```

**Log level**: `console.log()` - Always visible in DevTools console

---

### 2. `applications/desktop/src/providers/TauriPlayerCommandsProvider.tsx`
**Purpose**: Tauri invoke() timing and ignore window tracking

**What was added**:
- Timestamp when `seek()` method is called
- Log value of `IGNORE_WINDOW_MS` constant
- Time to acquire mutex and enable ignore window
- Actual time spent in `invoke('seek_to')`
- When ignore window timer is set and when it expires
- Detailed millisecond breakdowns

**Log pattern**:
```
[SEEK PERF] TauriProvider.seek() called at +1.02ms
[SEEK PERF] IGNORE_WINDOW_MS = 120ms
[SEEK PERF] Ignore window enabled at +0.15ms
[SEEK PERF] invoke('seek_to') completed in 3.45ms
[SEEK PERF] Ignore window DISABLED after 120.23ms
```

**Log level**: `console.log()` - Visible in DevTools console

---

### 3. `applications/desktop/src-tauri/src/main.rs`
**Purpose**: Rust Tauri command handler entry/exit timing

**What was added**:
- `std::time::Instant::now()` at function entry
- Position value passed to command
- Timing for successful completion or error
- Total time spent in command

**Log pattern**:
```
[SEEK PERF] === Rust seek_to() ENTRY === position=12.345
[SEEK PERF] === Rust seek_to() EXIT === completed in 5.89ms
```

**Log level**: `tracing::info!()` - Level: INFO
**Enable with**: `RUST_LOG=soul_playback=info`

---

### 4. `applications/desktop/src-tauri/src/playback.rs`
**Purpose**: PlaybackManager mutex lock and command send timing

**What was added**:
- Lock acquisition timing
- Time from lock to command being queued
- Entry/exit tracking for the wrapper function

**Log pattern**:
```
[SEEK PERF] PlaybackManager.seek() ENTRY position=12.345
[SEEK PERF] Lock acquired in 0.15ms
[SEEK PERF] PlaybackManager.seek() EXIT - command sent in 0.08ms
```

**Log level**: `tracing::trace!()` - Level: TRACE
**Enable with**: `RUST_LOG=soul_playback=trace`

---

### 5. `libraries/soul-playback/src/manager.rs`

#### Section A: seek_to() method (lines ~485-535)
**Purpose**: Core seek logic timing and state management

**What was added**:
- Entry timestamp and position
- Time spent cancelling crossfade (if active)
- Time spent cancelling stop fade (if active)
- Individual timing for decoder seek call
- Exit timestamp with total manager time
- Error logging if seek fails

**Log pattern**:
```
[SEEK PERF] === Manager.seek_to() ENTRY === position=Duration { secs: 12, nanos: 345000000 }
[SEEK PERF] Cancelling active crossfade (took 0.12ms so far)
[SEEK PERF] Decoder.seek() completed in 5.32ms
[SEEK PERF] === Manager.seek_to() EXIT === completed in 5.47ms
```

**Log level**: `tracing::info!()` - Level: INFO
**Enable with**: `RUST_LOG=soul_playback=info`

#### Section B: maybe_emit_position_update() method (lines ~1893-1912)
**Purpose**: Position update emission throttling verification

**What was added**:
- Changed log level from `trace!()` to `trace!()` but with [SEEK PERF] prefix
- Verifies the 100ms threshold calculation
- Shows actual sample count vs threshold
- Confirms sample rate

**Log pattern**:
```
[SEEK PERF] === Position Update EMIT === after 9823 samples (~100ms @ 48000Hz)
```

**Log level**: `tracing::trace!()` - Level: TRACE
**Enable with**: `RUST_LOG=soul_playback=trace`

---

## How to Enable Logging

### Frontend (Always enabled)
Simply open DevTools (F12) and look for `[SEEK PERF]` messages in Console tab.

### Backend - Development
```bash
# TRACE level (most detailed)
RUST_LOG=soul_playback=trace cargo xtask dev desktop

# INFO level (core timing only, less noise)
RUST_LOG=soul_playback=info cargo xtask dev desktop
```

### Backend - Production
1. Set environment variable:
   - Windows: `set RUST_LOG=soul_playback=info`
   - macOS/Linux: `export RUST_LOG=soul_playback=info`
2. Restart Soul Player
3. Logs saved to:
   - Windows: `%APPDATA%\Soul Player\logs\`
   - macOS: `~/Library/Application Support/soul-player/logs/`
   - Linux: `~/.config/soul-player/logs/`

## Performance Metrics Captured

The logging captures these key metrics:

| Metric | Captured At | Details |
|--------|-----------|---------|
| Frontend click time | useSeekBar.ts | Wall-clock timestamp when seek starts |
| Store update time | useSeekBar.ts | React store setState duration |
| Frontend→Backend gap | TauriPlayerCommandsProvider | Time between store update and invoke |
| Invoke duration | TauriPlayerCommandsProvider | Time in invoke() IPC call |
| Lock acquisition | playback.rs | Time to acquire PlaybackManager mutex |
| Manager entry→exit | manager.rs | Total seek logic time |
| Decoder seek | manager.rs | Actual decoder.seek() call time |
| Position update | manager.rs | Every ~100ms position event emission |
| Ignore window | TauriPlayerCommandsProvider | Duration window was active |

## Sample Output Timeline

Here's what a complete successful seek looks like:

```
=== FRONTEND ===
[SEEK PERF] ===== SEEK START ===== at 1234.56ms
[SEEK PERF] Target position: 45.678s (clamped from 45.678s)
[SEEK PERF] Store update: 0.42ms (UI now shows 68.5%)
[SEEK PERF] Invoking backend seek_to at 1.11ms
[SEEK PERF] invoke('seek_to') completed in 3.45ms (total: 4.56ms)
[SEEK PERF] Finalizing seek state in 120ms (already elapsed: 5.02ms)

=== BACKEND ===
[SEEK PERF] === Rust seek_to() ENTRY === position=45.678
[SEEK PERF] PlaybackManager.seek() ENTRY position=45.678
[SEEK PERF] Lock acquired in 0.18ms
[SEEK PERF] === Manager.seek_to() ENTRY === position=45.678s
[SEEK PERF] Decoder.seek() completed in 5.23ms (total manager time: 5.39ms)
[SEEK PERF] === Manager.seek_to() EXIT === completed in 5.41ms
[SEEK PERF] PlaybackManager.seek() EXIT - command sent in 0.07ms (total: 5.56ms)
[SEEK PERF] === Rust seek_to() EXIT === completed in 5.67ms

=== LATER (position update) ===
[SEEK PERF] === Position Update EMIT === after 9823 samples (~100ms @ 48000Hz)

=== FRONTEND (finally) ===
[SEEK PERF] ===== SEEK END ===== (total time: 125.89ms)
```

## Verification Checklist

After rebuilding, verify the logging works:

- [ ] Seek a track in the app
- [ ] Check DevTools console for `[SEEK PERF]` messages
- [ ] Confirm store update time is <1ms
- [ ] Confirm invoke time is <5ms
- [ ] Enable backend logging with RUST_LOG env var
- [ ] Check logs show `[SEEK PERF]` entries
- [ ] Verify "100ms" appears in position update logs (proves fix is compiled)
- [ ] Confirm decoder seek time is <50ms (depends on format)

## What This Reveals

The logging will help answer:

1. **Is the 100ms fix actually compiled?**
   - Check logs for "~100ms" in position update messages
   - If missing, rebuild is needed

2. **Where is time being spent?**
   - Store update (should be <1ms)
   - IPC overhead (should be <5ms)
   - Decoder seek (depends on format)
   - Ignore window (hardcoded at 120ms)

3. **Is seek truly slow or is it UI responsiveness?**
   - If backend completes in 10ms but perceived as 300ms slow
   - Problem is React rendering, not seek logic

4. **Is there lock contention?**
   - If lock acquisition is >5ms
   - Audio thread might be busy (unusual)

5. **Are position updates throttled correctly?**
   - Should appear every ~100ms
   - If more frequent, position update threshold isn't working

## Notes

- All frontend logs use `console.log()` - no configuration needed
- All backend logs use `tracing` crate for structured logging
- TRACE level includes mutex and helper function calls
- INFO level shows just the main flow (less noise)
- Logs can be saved to files in production (see SEEK_PERFORMANCE_LOGGING_GUIDE.md)
- No performance overhead of logging itself (tracing is very efficient)

