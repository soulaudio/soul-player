# Audio Startup Delay - Root Cause Analysis

**Test Results:** E2E test reveals 5+ second delay from play() to audio start

## Timing Breakdown from Test

```
1. Audio engine init: ~680ms (acceptable)
2. Warm-up cycle:     ~50ms (fast)
3. Load tracks:       < 1ms (instant)
4. play() call:       < 1µs (instant)
5. Wait for audio:    5+ seconds (TIMEOUT!)
---
TOTAL: 5.7+ seconds
```

## Root Cause Identified

### The Problem Chain:

1. **User clicks play** → `play()` command returns instantly
2. **PlaybackManager** sets state to `Loading`, calls `play_next_in_queue()`
3. **TrackLoader** spawns background thread to load audio source
4. **LocalAudioSource::new()** spawns decoder thread
5. **Decoder thread tries to open file** → **FAILS** (file not found/wrong working directory)
6. **Decoder thread exits immediately** with error log
7. **Buffer never gets filled** (decoder dead)
8. **TrackLoader waits for `is_ready()`** → buffer never reaches MIN_BUFFER_SAMPLES (24000)
9. **Times out after 5 seconds** (max_wait)
10. **PlaybackManager never receives `set_audio_source()`**
11. **Playing event never emitted**

### Code Locations:

**Decoder thread failure:**
```rust
// libraries/soul-audio-desktop/src/sources/local.rs:444-449
let file = match File::open(&path) {
    Ok(f) => f,
    Err(e) => {
        tracing::error!("[DecoderThread] Failed to open file: {}", e);
        return; // ← Thread exits, buffer never fills
    }
};
```

**TrackLoader timeout:**
```rust
// libraries/soul-audio-desktop/src/track_loader.rs:224-228
let max_wait = std::time::Duration::from_secs(5);
while !source.is_ready() && wait_start.elapsed() < max_wait {
    std::thread::sleep(std::time::Duration::from_millis(10));
}
```

**is_ready() check:**
```rust
// libraries/soul-audio-desktop/src/sources/local.rs:1167-1175
fn is_ready(&self) -> bool {
    match self.shared.try_lock() {
        Ok(state) => {
            state.output_buffer.len() >= MIN_BUFFER_SAMPLES || state.is_eof
        }
        Err(_) => false,
    }
}
```

## Why Warm-Up Didn't Help

The warm-up cycle (play/stop with no tracks) correctly initializes the CPAL audio device (~50ms), BUT:
- Warm-up has no effect on file decoding
- Decoder thread is spawned per-track, not during warm-up
- File I/O and buffer filling happens lazily when loading actual tracks

## Solutions

### Option A: Fix Test File Paths (Immediate)
The test uses relative paths that don't resolve from the decoder thread's working directory.

**Current test:**
```rust
create_test_track("1", "libraries/soul-audio-desktop/test_data/track_1.wav")
```

**Should be:**
```rust
let base_dir = std::env::current_dir().unwrap();
let track_path = base_dir.join("libraries/soul-audio-desktop/test_data/track_1.wav");
create_test_track("1", track_path.to_str().unwrap())
```

### Option B: Better Error Handling (Production)
Add proper error propagation from decoder thread to main thread:

1. **Decoder thread communicates errors back**
   - Use a result channel instead of silently exiting
   - Emit `PlaybackEvent::Error` when file open fails

2. **TrackLoader detects decoder failure faster**
   - Don't wait 5 seconds if decoder already exited
   - Check if thread is still alive

3. **Emit error event immediately**
   - User sees "File not found" instead of hanging
   - UX: instant feedback instead of 5s timeout

### Option C: Pre-validate Files (Prevention)
Add file validation before spawning decoder:

```rust
// In LocalAudioSource::new()
if !path.exists() {
    return Err(PlaybackError::AudioSource(
        format!("File not found: {}", path.display())
    ));
}
```

## Why User Sees 5s Delay in Real App

If user experiences 5s delay in production:

1. **File doesn't exist** (moved/deleted/network drive)
2. **Permission denied** (locked by another process)
3. **Slow disk** (spinning HDD, network drive) + insufficient buffer
4. **Codec issue** (unsupported format, corrupt file)

All of these cause decoder thread to fail or stall, triggering the 5s timeout.

## Immediate Next Steps

1. **Fix test paths** - Use absolute paths or current_dir()
2. **Re-run test** - Verify buffer fills quickly with valid files
3. **Check real app** - If still delays, investigate file access logs
4. **Add instrumentation** - Log decoder thread lifecycle

## Expected Behavior After Fix

```
1. Audio engine init: ~680ms
2. Warm-up cycle:     ~50ms
3. Load tracks:       ~1ms
4. play() call:       <1µs
5. Decoder fills buffer: ~50-200ms
6. Wait for audio:    ~100ms (buffer ready check)
---
TOTAL: <1 second (acceptable UX)
```

## Test Commands

```bash
# Run the E2E test
cargo test --package soul-audio-desktop --test startup_immediate_play_test -- --nocapture --test-threads=1

# Watch for decoder errors in logs
grep "DecoderThread" ~/.local/state/soul-player/logs/latest.log

# Check if files exist
ls -lh libraries/soul-audio-desktop/test_data/*.wav
```

