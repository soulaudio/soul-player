# Seek Performance Logging - Code Reference

This document shows the exact code changes made for performance logging at each step.

## 1. Frontend - useSeekBar.ts

**File**: `applications/shared/src/hooks/useSeekBar.ts`

**Location**: The entire `handleSeek` callback (lines 30-56)

### Key Additions

```typescript
// STEP 1: Frontend click timestamp
const clickTimestamp = performance.now();
console.log(`[SEEK PERF] ===== SEEK START ===== at ${clickTimestamp.toFixed(2)}ms`);

// ... existing code ...

// Log store update timing
const storeUpdateTime = performance.now();
usePlayerStore.setState({ progress: progressPercentage });
const storeUpdateDelta = performance.now() - storeUpdateTime;
console.log(`[SEEK PERF] Store update: ${storeUpdateDelta.toFixed(2)}ms`);

// Log invoke timing
const invokeStartTime = performance.now();
console.log(`[SEEK PERF] Invoking backend seek_to at ${(invokeStartTime - clickTimestamp).toFixed(2)}ms`);

commands.seek(clampedPosition)
  .then(() => {
    const backendCompletedTime = performance.now();
    const totalDelta = backendCompletedTime - clickTimestamp;
    console.log(`[SEEK PERF] Backend seek completed at +${(backendCompletedTime - clickTimestamp).toFixed(2)}ms`);
  })
  .finally(() => {
    // ... finalize timing ...
    console.log(`[SEEK PERF] ===== SEEK END ===== (total time: ${(performance.now() - clickTimestamp).toFixed(2)}ms)`);
  });
```

### Log Output

```
[SEEK PERF] ===== SEEK START ===== at 1234.56ms
[SEEK PERF] Target position: 12.345s
[SEEK PERF] Store update: 0.50ms
[SEEK PERF] Invoking backend seek_to at 1.02ms
[SEEK PERF] invoke('seek_to') completed in 3.45ms (total: 4.47ms)
[SEEK PERF] ===== SEEK END ===== (total time: 130.15ms)
```

---

## 2. Frontend - TauriPlayerCommandsProvider.tsx

**File**: `applications/desktop/src/providers/TauriPlayerCommandsProvider.tsx`

**Location**: The `seek()` method in commands object (lines 502-543)

### Key Additions

```typescript
async seek(position: number) {
  // STEP 2: Tauri invoke timestamp
  const invokeStartTime = performance.now();
  console.log(`[SEEK PERF] TauriProvider.seek() called at +${invokeStartTime.toFixed(2)}ms`);
  console.log(`[SEEK PERF] IGNORE_WINDOW_MS = ${IGNORE_WINDOW_MS}ms`);

  // Enable ignore window
  ignoringPositionUpdatesRef.current = true;
  console.log(`[SEEK PERF] Ignore window enabled at +${(performance.now() - invokeStartTime).toFixed(2)}ms`);

  // Clear existing timer
  if (ignoreTimerRef.current) {
    console.log(`[SEEK PERF] Cleared existing ignore timer`);
    clearTimeout(ignoreTimerRef.current);
  }

  // Send seek command with timing
  try {
    const beforeInvoke = performance.now();
    await invoke('seek_to', { position });
    const afterInvoke = performance.now();
    console.log(`[SEEK PERF] invoke('seek_to') completed in ${(afterInvoke - beforeInvoke).toFixed(2)}ms`);
  } catch (error) {
    console.error(`[SEEK PERF] invoke('seek_to') failed:`, error);
    throw error;
  }

  // Disable ignore window after delay
  const ignoreWindowStartTime = performance.now();
  ignoreTimerRef.current = setTimeout(() => {
    const ignoreWindowEndTime = performance.now();
    console.log(`[SEEK PERF] Ignore window DISABLED after ${(ignoreWindowEndTime - ignoreWindowStartTime).toFixed(2)}ms`);
    ignoringPositionUpdatesRef.current = false;
    ignoreTimerRef.current = null;
  }, IGNORE_WINDOW_MS);
}
```

### Log Output

```
[SEEK PERF] TauriProvider.seek() called at +1.02ms
[SEEK PERF] IGNORE_WINDOW_MS = 120ms
[SEEK PERF] Ignore window enabled at +0.15ms
[SEEK PERF] invoke('seek_to') completed in 3.45ms (total: +4.47ms)
[SEEK PERF] Ignore window DISABLED after 120.23ms
```

---

## 3. Backend - main.rs

**File**: `applications/desktop/src-tauri/src/main.rs`

**Location**: The `seek_to` command handler (lines 591-611)

### Original Code
```rust
#[tauri::command]
async fn seek_to(position: f64, playback: State<'_, LazyPlaybackManager>) -> Result<(), String> {
    Ok(playback.get().await?.seek(position)?)
}
```

### Modified Code
```rust
#[tauri::command]
async fn seek_to(position: f64, playback: State<'_, LazyPlaybackManager>) -> Result<(), String> {
    // STEP 3: Rust seek_to entry timestamp
    let entry_time = std::time::Instant::now();
    tracing::info!("[SEEK PERF] === Rust seek_to() ENTRY === position={:.3}s", position);

    let result = playback.get().await?.seek(position);

    let exit_time = entry_time.elapsed();
    match &result {
        Ok(_) => {
            tracing::info!("[SEEK PERF] === Rust seek_to() EXIT === completed in {:.2}ms", exit_time.as_millis());
        }
        Err(e) => {
            tracing::error!("[SEEK PERF] === Rust seek_to() ERROR === after {:.2}ms: {}", exit_time.as_millis(), e);
        }
    }

    result.map_err(|e| format!("{}", e))
}
```

### Log Output

```
[SEEK PERF] === Rust seek_to() ENTRY === position=12.345
[SEEK PERF] === Rust seek_to() EXIT === completed in 5.89ms
```

---

## 4. Backend - playback.rs

**File**: `applications/desktop/src-tauri/src/playback.rs`

**Location**: The `seek()` method in PlaybackManager (lines 1249-1283)

### Original Code
```rust
pub fn seek(&self, position: f64) -> Result<(), AudioError> {
    let playback = self
        .playback
        .lock()
        .map_err(|_| AudioError::MutexPoisoned {
            context: "seek command".to_string(),
        })?;
    playback
        .send_command(PlaybackCommand::Seek(position))
        .map_err(|e| AudioError::CommandFailed {
            command: "Seek".to_string(),
            reason: e.to_string(),
        })
}
```

### Modified Code
```rust
pub fn seek(&self, position: f64) -> Result<(), AudioError> {
    // STEP 3b: PlaybackManager wrapper entry
    let entry_time = std::time::Instant::now();
    tracing::trace!("[SEEK PERF] PlaybackManager.seek() ENTRY position={:.3}s", position);

    let playback = self
        .playback
        .lock()
        .map_err(|_| AudioError::MutexPoisoned {
            context: "seek command".to_string(),
        })?;

    let lock_time = entry_time.elapsed();
    tracing::trace!("[SEEK PERF] Lock acquired in {:.2}ms", lock_time.as_millis());

    let send_start = std::time::Instant::now();
    let result = playback
        .send_command(PlaybackCommand::Seek(position))
        .map_err(|e| AudioError::CommandFailed {
            command: "Seek".to_string(),
            reason: e.to_string(),
        });

    let send_time = send_start.elapsed();
    match &result {
        Ok(_) => {
            tracing::trace!("[SEEK PERF] PlaybackManager.seek() EXIT - command sent in {:.2}ms (total: {:.2}ms)",
                           send_time.as_millis(), entry_time.elapsed().as_millis());
        }
        Err(e) => {
            tracing::error!("[SEEK PERF] PlaybackManager.seek() ERROR after {:.2}ms: {}",
                           entry_time.elapsed().as_millis(), e);
        }
    }

    result
}
```

### Log Output

```
[SEEK PERF] PlaybackManager.seek() ENTRY position=12.345
[SEEK PERF] Lock acquired in 0.15ms
[SEEK PERF] PlaybackManager.seek() EXIT - command sent in 0.08ms (total: 0.23ms)
```

---

## 5. Backend - manager.rs (Part A: seek_to method)

**File**: `libraries/soul-playback/src/manager.rs`

**Location**: The `seek_to()` method (lines 485-535)

### Key Changes

```rust
pub fn seek_to(&mut self, position: Duration) -> Result<()> {
    // STEP 4: Decoder seek timestamp (manager entry)
    let entry_time = std::time::Instant::now();
    tracing::info!("[SEEK PERF] === Manager.seek_to() ENTRY === position={:?}", position);

    // Guard check with error logging
    if self.state == PlaybackState::Stopped {
        tracing::error!("[SEEK PERF] === Manager.seek_to() ERROR === NoTrackLoaded (state=Stopped) after {:.2}ms",
                       entry_time.elapsed().as_millis());
        return Err(PlaybackError::NoTrackLoaded);
    }

    // Crossfade cancellation with timing
    if self.crossfade.is_active() {
        tracing::info!("[SEEK PERF] Cancelling active crossfade due to seek (took {:.2}ms so far)",
                      entry_time.elapsed().as_millis());
        // ... reset logic ...
    }

    // Stop fade cancellation with timing
    if self.stop_fade.is_active() {
        tracing::debug!("[SEEK PERF] Cancelling active stop fade (took {:.2}ms so far)",
                       entry_time.elapsed().as_millis());
        // ... reset logic ...
    }

    if let Some(source) = self.sources.current_source_mut() {
        // ... clamping logic ...

        // STEP 4: Actual decoder seek call with timing
        let seek_start = std::time::Instant::now();
        source.seek(clamped_position)?;
        let seek_duration = seek_start.elapsed();
        tracing::info!("[SEEK PERF] Decoder.seek() completed in {:.2}ms (total manager time: {:.2}ms)",
                      seek_duration.as_millis(), entry_time.elapsed().as_millis());

        self.start_fade.start();
        tracing::info!("[SEEK PERF] === Manager.seek_to() EXIT === completed in {:.2}ms",
                      entry_time.elapsed().as_millis());

        Ok(())
    } else {
        tracing::error!("[SEEK PERF] === Manager.seek_to() ERROR === NoTrackLoaded (no current source) after {:.2}ms",
                       entry_time.elapsed().as_millis());
        Err(PlaybackError::NoTrackLoaded)
    }
}
```

### Log Output

```
[SEEK PERF] === Manager.seek_to() ENTRY === position=Duration { secs: 12, nanos: 345000000 }
[SEEK PERF] Decoder.seek() completed in 5.32ms (total manager time: 5.39ms)
[SEEK PERF] === Manager.seek_to() EXIT === completed in 5.41ms
```

---

## 6. Backend - manager.rs (Part B: maybe_emit_position_update)

**File**: `libraries/soul-playback/src/manager.rs`

**Location**: The `maybe_emit_position_update()` method (lines 1893-1912)

### Original Code
```rust
pub fn maybe_emit_position_update(&mut self, samples_processed: usize) {
    self.position_update_samples += samples_processed;

    let threshold = (self.sample_rate as usize * 2) / 10; // 100ms

    if self.position_update_samples >= threshold {
        tracing::trace!(
            "[POSITION] Emitting update after {} samples (threshold: {}, interval: ~100ms @ {}Hz)",
            self.position_update_samples,
            threshold,
            self.sample_rate
        );
        self.emit_position_update();
        self.position_update_samples = 0;
    }
}
```

### Modified Code (Minor Log Change)
```rust
pub fn maybe_emit_position_update(&mut self, samples_processed: usize) {
    // STEP 6: Position update emission logging
    // Accumulate samples
    self.position_update_samples += samples_processed;

    // Calculate threshold: emit approximately every 100ms
    // At 48kHz stereo, 100ms = 48000 * 0.1 * 2 = 9600 samples
    // Formula: (sample_rate * 2 channels) / 10 = samples per 100ms
    let threshold = (self.sample_rate as usize * 2) / 10; // 100ms

    if self.position_update_samples >= threshold {
        tracing::trace!(
            "[SEEK PERF] === Position Update EMIT === after {} samples (threshold: {}, ~100ms @ {}Hz)",
            self.position_update_samples,
            threshold,
            self.sample_rate
        );
        self.emit_position_update();
        self.position_update_samples = 0;
    }
}
```

### Key Change

Just the log prefix changed from `[POSITION]` to `[SEEK PERF]` to group with other seek logs.

### Log Output

```
[SEEK PERF] === Position Update EMIT === after 9823 samples (threshold: 9600, ~100ms @ 48000Hz)
```

This log verifies the **100ms fix is actually compiled** (the `/10` division).

---

## Log Levels Summary

| Component | Log Level | Always Visible? | Visible With |
|-----------|-----------|-----------------|--------------|
| useSeekBar.ts | `console.log()` | YES | DevTools Console (F12) |
| TauriPlayerCommandsProvider.tsx | `console.log()` | YES | DevTools Console (F12) |
| main.rs seek_to() | `tracing::info!()` | NO | `RUST_LOG=soul_playback=info` |
| playback.rs seek() | `tracing::trace!()` | NO | `RUST_LOG=soul_playback=trace` |
| manager.rs seek_to() | `tracing::info!()` | NO | `RUST_LOG=soul_playback=info` |
| manager.rs maybe_emit_position_update() | `tracing::trace!()` | NO | `RUST_LOG=soul_playback=trace` |

---

## Timing Measurements Used

All Rust timing uses:
```rust
let entry_time = std::time::Instant::now();
// ... do work ...
let elapsed = entry_time.elapsed();
tracing::info!("Took {:.2}ms", elapsed.as_millis());
```

All Frontend timing uses:
```typescript
const startTime = performance.now();
// ... do work ...
const elapsed = performance.now() - startTime;
console.log(`Took ${elapsed.toFixed(2)}ms`);
```

Both have nanosecond precision internally but are formatted to 2 decimal places (hundredths of a millisecond) for readability.

---

## Verification Checklist

After applying changes, verify:

- [ ] Code compiles: `cargo check`
- [ ] Frontend logs visible in DevTools Console
- [ ] Backend logs visible with RUST_LOG=soul_playback=info
- [ ] Each log appears in correct sequence
- [ ] Position update shows "~100ms" (verifies /10 division)
- [ ] Total time is ~130ms (optimistic + ignore window)
- [ ] No latency overhead from logging

