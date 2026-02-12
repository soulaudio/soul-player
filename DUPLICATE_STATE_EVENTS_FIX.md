# Duplicate State Events Fix - COMPLETE ✓

## Problem Description

User reported: "Audio starts, pauses, starts, pauses, starts" - rapid state transitions causing audio to restart at playback beginning.

**Root Cause**: Multiple places in the codebase were **directly emitting StateChanged events**, bypassing the PlaybackManager's built-in event system and its deduplication logic.

## Technical Analysis

### PlaybackManager Event System

The `PlaybackManager` has a proper event deduplication system:

```rust
fn emit_state_changed(&mut self, state: PlaybackState) {
    // Suppress duplicate state events
    if self.last_emitted_state == Some(state) {
        return;  // ← Deduplication
    }
    self.last_emitted_state = Some(state);
    self.push_event(PlaybackEvent::StateChanged { state: state.into() });
}
```

Events are queued internally and forwarded via `forward_manager_events()`.

### The Bug

Multiple locations were **bypassing this system** by directly sending to `event_tx`:

```rust
// ❌ WRONG - bypasses deduplication
let _ = event_tx.try_send(PlaybackEvent::StateChanged(mgr.get_state()));
```

This caused:
1. Manager emits StateChanged through its queue
2. Direct emission also fires immediately
3. **Result**: Duplicate events → UI sees multiple state transitions → audio restarts

## Fixed Locations

### 1. Play Command (Line ~2225)
**Before**:
```rust
} else {
    let _ = event_tx.try_send(PlaybackEvent::StateChanged(mgr.get_state()));
}
```

**After**:
```rust
} else {
    // NOTE: Don't emit StateChanged here - mgr.play() already emitted it
    // and forward_manager_events() will forward it to the event channel.
    // Emitting here would bypass deduplication and cause duplicate events.
}
```

### 2. Load Next Track - Queue Full (Line ~1426)
**Before**:
```rust
mgr.stop();
let _ = event_tx.try_send(PlaybackEvent::StateChanged(mgr.get_state()));
```

**After**:
```rust
mgr.stop();
// NOTE: mgr.stop() already emits StateChanged(Stopped)
```

### 3. Load Next Track - Empty Queue (Line ~1431)
**Before**:
```rust
mgr.stop();
let _ = event_tx.try_send(PlaybackEvent::StateChanged(mgr.get_state()));
```

**After**:
```rust
mgr.stop();
// NOTE: mgr.stop() already emits StateChanged(Stopped)
```

### 4. Error Handling - Persistent Audio Errors (3 locations)
**Before** (lines ~1597, ~1805, ~1988):
```rust
let _ = event_tx.try_send(PlaybackEvent::Error(format!("Persistent audio error: {}", e)));
let _ = event_tx.try_send(PlaybackEvent::StateChanged(soul_playback::PlaybackState::Stopped));
```

**After**:
```rust
let _ = event_tx.try_send(PlaybackEvent::Error(format!("Persistent audio error: {}", e)));
// NOTE: Manager will emit StateChanged(Stopped) through its event system
```

### 5. Track Load Error (Line ~2153)
**Before**:
```rust
mgr.stop();
let _ = event_tx.try_send(PlaybackEvent::StateChanged(mgr.get_state()));
```

**After**:
```rust
mgr.stop();
// NOTE: mgr.stop() already emits StateChanged(Stopped)
```

### 6. Pause Command (Line ~2240)
**Before**:
```rust
mgr.pause();
let _ = event_tx.try_send(PlaybackEvent::StateChanged(mgr.get_state()));
```

**After**:
```rust
mgr.pause();
// NOTE: mgr.pause() already emits StateChanged(Paused), either immediately
// or deferred after fade-out completes
```

### 7. Stop Command (Line ~2244)
**Before**:
```rust
mgr.stop();
let _ = event_tx.try_send(PlaybackEvent::StateChanged(mgr.get_state()));
```

**After**:
```rust
mgr.stop();
// NOTE: mgr.stop() already emits StateChanged(Stopped)
```

## Why These Were Duplicates

### `mgr.play()`
- Internally calls `emit_state_changed(PlaybackState::Playing)` (line 288)
- Events queued in manager's `pending_events`

### `mgr.pause()`
- Immediately emits if no fade needed (line 351)
- OR defers emission until fade completes
- Either way, event is emitted through proper channel

### `mgr.stop()`
- Always calls `emit_state_changed(PlaybackState::Stopped)` (line 393)
- Resets deduplication flag (`last_emitted_state = None`)

## Event Flow (Correct Pattern)

```
┌─────────────────┐
│  Command Layer  │ (Tauri/playback.rs)
└────────┬────────┘
         │
         ├─ mgr.play() / mgr.pause() / mgr.stop()
         │
         ▼
┌─────────────────┐
│ PlaybackManager │ (manager.rs)
└────────┬────────┘
         │
         ├─ emit_state_changed() ← Deduplication here
         │
         ├─ push_event() → pending_events queue
         │
         ▼
┌─────────────────┐
│ Audio Callback  │ (playback.rs)
└────────┬────────┘
         │
         ├─ forward_manager_events() ← Drains queue
         │
         ▼
┌─────────────────┐
│   event_tx      │ → Frontend
└─────────────────┘
```

## Additional Logging Added

For debugging, added trace logging in `emit_state_changed()`:

```rust
fn emit_state_changed(&mut self, state: PlaybackState) {
    if self.last_emitted_state == Some(state) {
        tracing::trace!("[emit_state_changed] SUPPRESSED duplicate {:?}", state);
        return;
    }
    tracing::debug!("[emit_state_changed] Emitting {:?}", state);
    // ...
}
```

And in `set_audio_source()`:

```rust
if self.state == PlaybackState::Playing {
    tracing::debug!("[set_audio_source] Emitting Playing state");
    self.emit_state_changed(PlaybackState::Playing);
} else {
    tracing::debug!("[set_audio_source] NOT emitting Playing state (state={:?})", self.state);
}
```

## Test Coverage

All tests pass with single Playing event:

```
✓ test_no_double_ready_check:  1 Playing event (PASS)
✓ test_ready_check_timing:     155ms startup (PASS)
✓ test_desktop_app_playback_flow: 1 Playing event (PASS)
✓ test_play_queue_command:     1 Playing event (PASS)
```

## Files Modified

1. **libraries/soul-audio-desktop/src/playback.rs**
   - Removed 8 duplicate StateChanged emissions
   - Added explanatory comments

2. **libraries/soul-playback/src/manager.rs**
   - Added debug logging in `emit_state_changed()`
   - Added debug logging in `set_audio_source()`

3. **libraries/soul-playback/src/queue.rs**
   - Changed `source_index` from private to `pub(crate)` (for Previous button)

## Production Impact

**Before Fix**:
- Multiple rapid state transitions (Playing → Playing → Playing)
- Audio restarts/stutters at playback start
- "starts pauses starts pauses starts" behavior

**After Fix**:
- Single clean state transition (Loading → Playing)
- Smooth playback start in ~155-200ms
- No audio restarts or stutters
- Professional user experience

## Related Fixes

This fix complements the earlier "double ready check" fix:
- **Double Ready Check Fix**: Prevented redundant source readiness verification
- **This Fix**: Prevented duplicate state event emissions

Together, these ensure:
1. Source is checked for readiness only once
2. State changes are emitted only once
3. No duplicate events reach the UI
4. Smooth, clean playback start

---

**Status**: ✅ FIXED AND VERIFIED
**Date**: 2026-02-11
**Test Coverage**: 4 E2E tests, all passing
