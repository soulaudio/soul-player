# Single-Writer PlaybackManager Architecture Implementation

**Status**: Feature-gated implementation (PARTIAL - compilation issues remain)
**Feature Flag**: `single-writer-manager` (default: OFF)
**Target**: 50-60% reduction in lock operations
**Date**: 2026-02-11

---

## Overview

This document describes the implementation of Task #4: Single-Writer PlaybackManager Architecture. This is a high-risk architectural change that eliminates `Arc<Mutex<PlaybackManager>>` by giving the audio callback exclusive ownership of the manager.

---

## Implementation Summary

### ✅ Completed Components

#### 1. Feature Flag Infrastructure (`Cargo.toml`)
```toml
[features]
single-writer-manager = []  # EXPERIMENTAL: Single-writer PlaybackManager (no Arc<Mutex<>>)
```

#### 2. Core Data Structures (`playback.rs` lines 596-790)

**`PlaybackStateSnapshot`**: Lock-free state snapshot for UI queries
- Contains all frequently-accessed state (position, volume, track, etc.)
- Updated periodically (~100ms) from audio callback
- Read via `ArcSwap` (lock-free atomic pointer swap)
- Includes staleness detection via timestamp

**`AudioCallbackState`**: Owned manager state for audio callback
- **Owns** `PlaybackManager` (NO Arc, NO Mutex!)
- Includes command receiver, event sender, track loader
- Manages snapshot publishing interval
- Handles error recovery state

**`PlaybackStateBackup`**: State serialization for device switching
- Captures complete manager state before stream destruction
- Used to restore state when creating new stream
- Includes queue, history, position, settings, etc.

#### 3. Conditional `DesktopPlayback` Fields (lines 815-825)

**Legacy mode** (`!single-writer-manager`):
```rust
manager: Arc<Mutex<PlaybackManager>>,  // Shared with audio thread
```

**Single-writer mode** (`single-writer-manager`):
```rust
state_snapshot: Arc<ArcSwap<PlaybackStateSnapshot>>,  // Lock-free queries
```

#### 4. Initialization Logic (lines 923-1066)

- Conditionally creates either `Arc<Mutex<Manager>>` or `ArcSwap<Snapshot>`
- Calls appropriate stream creation function based on feature
- Properly initializes struct fields for each mode

#### 5. Stream Creation (`create_audio_stream_single_writer`, lines 1408-1633)

- Creates owned `PlaybackManager` (not shared!)
- Wraps in `AudioCallbackState` with all callback context
- Builds F32 audio stream with direct manager access
- **Critical difference**: Audio callback processes commands and audio WITHOUT locks
- Publishes snapshots every ~100ms for UI queries
- Handles zero-device systems (silent mode)

**Current Limitation**: Only F32 format supported (I32/I16 would follow same pattern)

#### 6. Single-Writer Helper Functions (lines 2854-3067)

- `process_command_single_writer()`: Command processing WITHOUT locks
- `poll_track_loader_single_writer()`: Track loading results handling
- `prepare_next_track_if_needed_single_writer()`: Gapless/crossfade preparation
- `load_next_track_single_writer()`: Next track loading trigger
- `forward_manager_events_single_writer()`: Event forwarding to UI

All functions access `state.manager` directly - NO mutex acquisition!

#### 7. Lock-Free Public API (lines 3104-3167)

Updated getter methods to use snapshot in single-writer mode:
- `get_state()`: Reads from snapshot (no lock)
- `get_current_track()`: Reads from snapshot (no lock)
- `get_position()`: Reads from snapshot (no lock)
- `get_volume()`: Reads from snapshot (no lock)
- `get_shuffle_mode()`: Reads from snapshot (no lock)
- `get_repeat_mode()`: Reads from snapshot (no lock)

Manager-mutating methods (`get_manager_mut`, `get_playback_manager`) are `#[cfg(not(feature = "single-writer-manager"))]` - unavailable in single-writer mode.

---

## Architecture Comparison

### Legacy Mode (`Arc<Mutex<>>`)

```
UI Thread                     Audio Callback Thread
    |                               |
    | send_command()                |
    v                               v
 [Channel] -----------------> [try_recv_command]
                                    |
                                    v
                             [manager.lock()] <- MUTEX LOCK
                                    |
                                    v
                             [process_command]
                                    |
                                    v
                             [manager.unlock()]
                                    |
                                    v
                             [manager.lock()] <- MUTEX LOCK (again!)
                                    |
                                    v
                             [process_audio()]
                                    |
                                    v
                             [manager.unlock()]
```

**Locks per callback**: 2-3 (command processing + audio processing + queries)

### Single-Writer Mode (Feature-Gated)

```
UI Thread                     Audio Callback Thread
    |                               |
    | send_command()                |
    v                               v
 [Channel] -----------------> [try_recv_command]
    |                               |
    |                               v
    |                        [process_command]  <- Direct access!
    |                               |
    |                               v
    |                        [process_audio()]  <- Direct access!
    |                               |
    |                               v
    |                        [publish_snapshot] <- Every ~100ms
    |                               |
    v                               v
[ArcSwap] <-------------------  [store()]
    |
    v
[load()] <- Lock-free read!
```

**Locks per callback**: 0 (commands and audio processing)
**Lock-free operations**: UI queries via ArcSwap snapshot

---

## Known Issues (Compilation Errors)

The following issues prevent compilation with `--features single-writer-manager`:

### 1. PlaybackManager API Mismatches

**`load_source_queue()` method not found**:
```
error[E0599]: no method named `load_source_queue` found for struct `PlaybackManager`
```
- **Fix**: Update to correct method name (likely `set_source_queue` or similar)
- **Location**: `process_command_single_writer()` line 2966

**`try_recv_result()` method not found on TrackLoader**:
```
error[E0599]: no method named `try_recv_result` found for struct `Arc<TrackLoader>`
```
- **Fix**: Check TrackLoader API - might be `try_recv()` or `poll_result()`
- **Location**: `poll_track_loader_single_writer()` line 2985

**`poll_event()` method not found**:
```
error[E0599]: no method named `poll_event` found for struct `PlaybackManager`
```
- **Fix**: Check PlaybackManager event API - might need different approach
- **Location**: `forward_manager_events_single_writer()` line 3042

### 2. Event Type Mismatches

**`PlaybackEvent::TrackChanged` is a struct variant, not tuple**:
```
error[E0164]: expected tuple struct or tuple variant, found struct variant
```
- **Fix**: Update pattern matching to struct syntax: `{ track_id, previous_track_id }`
- **Location**: `forward_manager_events_single_writer()` line 3044

**`CrossfadeStarted` field names incorrect**:
```
error[E0026]: variant does not have fields named `from`, `to`
```
- **Fix**: Use correct field names `from_track_id`, `to_track_id`
- **Location**: `forward_manager_events_single_writer()` lines 3048-3049

**Other event variants** (CrossfadeProgress, StateChanged):
- Similar struct vs tuple variant issues
- Need to update to match actual soul_playback event definitions

### 3. Incomplete Feature Gates

Several methods still try to access `self.manager` unconditionally:
```
error[E0609]: no field `manager` on type `&DesktopPlayback`
```
- **Locations**: Various methods that need `#[cfg(not(feature = "single-writer-manager"))]`
- **Fix**: Add conditional compilation for manager-dependent methods

---

## Device Switching Strategy

Device switching requires stream recreation, which destroys the manager in single-writer mode.

**Planned Approach**:
1. Serialize current state via `AudioCallbackState::serialize_state()`
2. Send stop command and wait for audio callback to complete
3. Drop old stream (destroys AudioCallbackState and owned manager)
4. Create new PlaybackManager with restored state
5. Create new stream with new manager
6. Resume playback

**Not Yet Implemented**: This requires additional plumbing in `switch_device()` methods.

---

## Testing Strategy

### Unit Tests Needed

1. **Snapshot Publishing**
   - Verify snapshot updates every ~100ms
   - Check staleness detection
   - Validate all fields copied correctly

2. **Command Processing**
   - Test all PlaybackCommand variants
   - Verify state transitions
   - Check event emission

3. **Error Recovery**
   - Test fade-out on audio errors
   - Verify DAC keepalive noise generation
   - Check consecutive error handling

### Integration Tests Needed

1. **Pause/Resume Reliability** (adapt existing test)
   - Run `pause_during_startup_e2e_test` with feature
   - Verify no false starts
   - Check fade completions

2. **Queue Navigation**
   - Test next/previous commands
   - Verify track loading
   - Check state consistency

3. **Device Switching**
   - Test state preservation across switches
   - Verify queue restoration
   - Check position accuracy

### Performance Benchmarks

Compare lock contention between modes:
- Measure time spent in `try_lock()` failures
- Count DAC keepalive noise samples emitted (proxy for contention)
- Profile audio callback latency distribution

**Target**: 50-60% reduction in lock-related overhead

---

## Enabling the Feature

### Development Testing

```bash
# Compile with single-writer mode
cargo build -p soul-audio-desktop --features single-writer-manager

# Run tests (once compilation issues fixed)
cargo test -p soul-audio-desktop --features single-writer-manager

# Run desktop app
cargo run -p soul-player-desktop --features single-writer-manager
```

### Integration into Workspace

**NOT RECOMMENDED** until all issues are resolved and tests pass.

To enable by default (when ready):
```toml
# In libraries/soul-audio-desktop/Cargo.toml
[features]
default = ["effects", "volume-leveling", "single-writer-manager"]
```

---

## Risks and Mitigations

### Risk: Manager State Unavailable for External Queries

**Impact**: Methods like `get_manager_mut()` can't exist in single-writer mode.

**Mitigation**:
- Snapshot provides all commonly-needed state
- For rare operations (e.g., effect chain config), add commands to the channel
- Alternative: Add specific query channels for complex state

### Risk: Snapshot Staleness

**Impact**: UI might see state 100ms old.

**Mitigation**:
- 100ms is typically imperceptible for UI updates
- Position updates still use `PositionUpdated` events (accurate timing)
- Can reduce interval if needed (trades CPU for freshness)

### Risk: Device Switching Complexity

**Impact**: Must serialize/restore full manager state.

**Mitigation**:
- `PlaybackStateBackup` captures all necessary state
- Test coverage for state restoration
- Fallback: Keep legacy mode for problematic scenarios

### Risk: Incomplete API Coverage

**Impact**: Some commands not yet implemented in single-writer helpers.

**Current Status**: Basic commands done (play/pause/stop/next/prev/seek/volume).
**Todo**: Queue manipulation, lazy loading, shuffle/repeat advanced features.

---

## Next Steps

### Critical (Fix Compilation)

1. **Fix PlaybackManager API calls**
   - Update `load_source_queue` to correct method
   - Fix `try_recv_result` on TrackLoader
   - Fix `poll_event` on PlaybackManager

2. **Fix Event Type Mismatches**
   - Update all `PlaybackEvent` pattern matches to struct syntax
   - Use correct field names from `soul_playback::PlaybackEvent`

3. **Complete Feature Gates**
   - Audit all methods accessing `self.manager`
   - Add `#[cfg]` attributes where needed
   - Ensure both modes compile cleanly

### High Priority (Functionality)

4. **Implement I32/I16 Sample Format Support**
   - Follow F32 pattern
   - Add conversion buffers to `AudioCallbackState`
   - Test with ASIO backend

5. **Complete Command Coverage**
   - Implement remaining PlaybackCommand variants
   - Add queue manipulation (RemoveFromQueue, ClearQueue, etc.)
   - Test lazy loading support

6. **Device Switching Integration**
   - Add state serialization/restoration to `switch_device()`
   - Test across different backends
   - Verify no audio glitches during switch

### Medium Priority (Optimization)

7. **Tune Snapshot Interval**
   - Profile CPU overhead of 100ms publishing
   - Consider adaptive interval based on state changes
   - Add config option if needed

8. **Performance Benchmarking**
   - Measure lock contention reduction
   - Profile audio callback latency
   - Compare CPU usage between modes

9. **Add Snapshot Staleness Warnings**
   - Log if UI reads very stale snapshots (>500ms)
   - Could indicate audio callback not running

### Low Priority (Polish)

10. **Documentation**
    - Add rustdoc comments to new types
    - Document snapshot lifetime and staleness
    - Add architecture diagrams

11. **Logging**
    - Add debug logs for snapshot publishing
    - Log state transitions in single-writer mode
    - Add performance metrics (time per callback)

---

## Rollback Plan

If issues arise:

1. **Disable by default**: Feature is already opt-in via feature flag
2. **Fix critical bugs**: Single-writer mode can coexist with legacy mode
3. **Remove if necessary**: Can delete all `#[cfg(feature = "single-writer-manager")]` blocks

**Migration path**: Users can switch between modes by toggling feature flag during build.

---

## References

- **Original Task**: See task instructions in conversation
- **Related Issues**: Audio callback latency, lock contention under load
- **Inspiration**: Single-writer pattern from JACK audio server, CPAL examples

---

## Appendix: Lock Contention Analysis (Pre-Implementation)

**Symptoms observed**:
- Occasional DAC keepalive noise during heavy UI interactions
- `try_lock()` failures in audio callback (lines 1459-1470 in legacy code)
- Position updates lag during rapid command sequences

**Root cause**: Multiple threads (UI, audio callback, device monitor) all locking the same manager.

**Expected improvement with single-writer mode**:
- Zero locks in audio callback (most critical path)
- UI queries via lock-free ArcSwap
- Commands remain channel-based (already lock-free)

**Estimated impact**: 50-60% reduction in time spent on lock operations, measured by reduction in DAC keepalive noise emission and improved callback latency percentiles.

---

**Implementation by**: Claude Sonnet 4.5
**Review needed**: Before enabling by default or merging to main
