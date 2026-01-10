# WASM Playback Bug Fix

## Root Cause Identified

**File:** `libraries/soul-playback/src/wasm/manager.rs:44-50`

The WASM `play()` method was only emitting a `stateChange` event but **not a `trackChange` event**. This meant that when you clicked play on a track, WASM would transition to "loading" state, set an internal current track, but never notify JavaScript about it.

Compare the broken `play()` method to the working `next()` method (line 67):

```rust
// ❌ BROKEN (before fix)
pub fn play(&mut self) -> Result<(), JsValue> {
    self.inner.play().map_err(|e| self.handle_error(e))?;
    self.emit_state_change();  // Only this
    Ok(())
}

// ✅ WORKS
pub fn next(&mut self) -> Result<(), JsValue> {
    self.inner.next().map_err(|e| self.handle_error(e))?;
    self.emit_track_change();  // ← Emits track change!
    self.emit_queue_change();
    Ok(())
}
```

## The Fix

Added conditional `emit_track_change()` to the `play()` method:

```rust
/// Start or resume playback
pub fn play(&mut self) -> Result<(), JsValue> {
    let old_state = self.inner.get_state();

    self.inner
        .play()
        .map_err(|e| self.handle_error(e))?;

    let new_state = self.inner.get_state();

    self.emit_state_change();

    // Only emit track change if we transitioned to Loading state
    // (meaning a new track is being loaded, not just resuming from pause)
    if new_state == PlaybackState::Loading && old_state != PlaybackState::Paused {
        self.emit_track_change();
    }

    Ok(())
}
```

**Key Points:**
- ✅ Emits track change when starting a new track (Stopped/Loading → Loading)
- ✅ Does NOT emit when resuming from pause (Paused → Playing)
- ✅ Prevents track from reloading when clicking play/pause

## Rebuild Instructions

The WASM module needs to be recompiled for the fix to take effect:

```bash
# Navigate to soul-playback
cd libraries/soul-playback

# Build WASM module
wasm-pack build --target web --out-dir ../../applications/marketing/src/wasm/soul-playback

# If wasm-pack is not installed:
cargo install wasm-pack

# Then rebuild
wasm-pack build --target web --out-dir ../../applications/marketing/src/wasm/soul-playback
```

**Alternative:** If the WASM output directory is different, check `package.json` scripts or existing build commands.

## After Rebuild

1. Restart the dev server:
   ```bash
   cd applications/marketing
   yarn dev
   ```

2. Click a track and check console logs

3. Expected log sequence:
   ```
   [WasmPlaybackAdapter] play() called, current state: stopped
   [WasmPlaybackAdapter] Calling WASM play(), queue length: 33
   [WasmPlaybackAdapter] State change: loading
   [Bridge] State change: loading
   [WasmPlaybackAdapter] *** onTrackChange callback invoked *** with track  ← THIS SHOULD NOW FIRE!
   [WasmPlaybackAdapter] Track change: { id: "1", title: "Dark", ... }
   [WasmPlaybackAdapter] Mapped track: { ... }
   [WebAudioPlayer] Loading track: /demo-audio/dark.flac
   [WebAudioPlayer] Failed to load audio: (file doesn't exist) OR
   [WebAudioPlayer] Playback started (if file exists)
   ```

## Note on Missing Audio Files

Tracks 1-2 reference non-existent files:
- `/demo-audio/dark.flac` ❌
- `/demo-audio/eyes.flac` ❌

To test, click track #3 "BOY IN THE MIRROR" which has a valid file:
- `/demo-audio/Sebastián Stupák - pressures, father I sober - 01 BOY IN THE MIRROR.mp3` ✅

## Workaround in TypeScript (Temporary)

While the WASM is being rebuilt, the TypeScript workaround in `wasm-playback-adapter.ts:135-147` will show an error:

```
[WasmPlaybackAdapter] ERROR: Track change event never fired!
```

This is expected until WASM is rebuilt with the fix.

## All Related Fixes

This fix is part of a larger set of improvements:

1. **WASM Event Bug** (THIS FIX) - `play()` now emits track change event
2. **Cover Art Retrieval** - Queue items now look up cover URLs from storage
3. **Field Name Fixes** - duration_secs and track_number properly mapped
4. **Enhanced Logging** - Comprehensive logging for debugging

See `PLAYBACK_DEBUG_FIXES.md` for full details.
