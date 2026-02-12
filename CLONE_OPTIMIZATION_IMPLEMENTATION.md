# Clone Operation Optimization Implementation Guide

## Overview

This document provides a complete implementation guide for optimizing 75 clone operations in `libraries/soul-audio-desktop/src/playback.rs`, with a target of reducing to <20 clones in hot paths.

## Current State

- **Total clones**: 75
- **Hot path clones**: ~30 (device queries, track cloning in events)
- **Major bottlenecks**:
  1. `current_device.lock().unwrap().clone()` - 6 occurrences in query methods
  2. `current_device_id.lock().unwrap().clone()` - 1 occurrence
  3. `track.clone()` in event emission - 12+ occurrences
  4. Arc clones written as `.clone()` instead of `Arc::clone()` - ~50 occurrences

## Optimization Strategy

### 1. Device Name Storage (HIGH IMPACT)

**Current**:
```rust
current_device: Arc<Mutex<String>>,
current_device_id: Arc<Mutex<Option<String>>>,
```

**Optimized**:
```rust
current_device: Arc<ArcSwap<Arc<str>>>,
current_device_id: Arc<ArcSwap<Option<Arc<str>>>>,
```

**Benefits**:
- Lock-free reads (no Mutex contention)
- Only Arc pointer is cloned, not the string data
- Estimated reduction: 7 lock+clone operations → 7 Arc::clone operations

**Implementation**:

#### Step 1: Update struct field

```rust
// In DesktopPlayback struct (around line 648)
/// Current device name (lock-free via ArcSwap)
current_device: Arc<ArcSwap<Arc<str>>>,

/// Current device ID (lock-free via ArcSwap)
current_device_id: Arc<ArcSwap<Option<Arc<str>>>>,
```

#### Step 2: Update initialization (around line 816)

```rust
// OLD:
let current_device = Arc::new(Mutex::new(actual_device_name.clone()));
let current_device_id = Arc::new(Mutex::new(device_id));

// NEW:
let current_device = Arc::new(ArcSwap::from_pointee(Arc::from(actual_device_name.as_str())));
let current_device_id = Arc::new(ArcSwap::from_pointee(device_id.map(|s| Arc::from(s.as_str()))));
```

#### Step 3: Update getter methods

```rust
// Line 3005-3007
/// Get current device name
pub fn get_current_device(&self) -> String {
    // OPTIMIZED: Load Arc, convert to String only when needed
    (*self.current_device.load()).to_string()
}

// Line 3017-3019
/// Get current device ID
pub fn get_current_device_id(&self) -> Option<String> {
    // OPTIMIZED: Load Arc, convert to String only when needed
    self.current_device_id.load().as_ref().as_ref().map(|arc| arc.to_string())
}
```

#### Step 4: Update setter operations (around line 2838, 3084, etc.)

```rust
// OLD:
*self.current_device.lock().unwrap() = actual_device_name.clone();

// NEW:
self.current_device.store(Arc::new(Arc::from(actual_device_name.as_str())));
```

### 2. Track Cloning in Events (MEDIUM IMPACT)

**Current**: Full `QueueTrack` clone on every event emission
**Optimized**: Use `Arc<QueueTrack>` for events, only clone when absolutely necessary

**Benefits**:
- Eliminates cloning of PathBuf, Strings in track metadata
- Only Arc pointer is cloned for event emission
- Estimated reduction: 12+ deep clones → 12 Arc::clone operations

**Implementation**:

#### Option A: Wrap tracks in Arc at source

```rust
// In event emission (lines 1423-1424, 2223-2224, etc.)
// OLD:
PlaybackEvent::TrackChanged(Some(TrackChangeInfo {
    path: track.path.clone(),
    track: track.clone(),
}))

// NEW (if we wrap track in Arc):
PlaybackEvent::TrackChanged(Some(TrackChangeInfo {
    path: Arc::clone(&track.path),  // If PathBuf wrapped in Arc
    track: Arc::clone(&track),  // If QueueTrack wrapped in Arc
}))
```

#### Option B: Clone only required fields

For events that don't need the full track, extract only what's needed:

```rust
// Instead of:
track: track.clone(),

// Use:
track_id: track.id.clone(),
track_title: track.title.clone(),
// ... only fields actually used by the event handler
```

### 3. Explicit Arc::clone (LOW IMPACT, HIGH CLARITY)

**Current**: `manager.clone()`, `track_loader.clone()`, etc.
**Optimized**: `Arc::clone(&manager)`, `Arc::clone(&track_loader)`, etc.

**Benefits**:
- Makes it explicit that only Arc pointer is cloned
- No performance change, but improves code clarity
- Helps identify which clones are cheap (Arc) vs expensive (data)

**Implementation**:

Replace all Arc clones with explicit syntax:

```rust
// Lines 781-793, etc.
// OLD:
manager.clone(),
command_rx.clone(),
event_tx.clone(),

// NEW:
Arc::clone(&manager),
command_rx.clone(),  // Not Arc, so keep as-is
Arc::clone(&event_tx),
```

### 4. Command Cloning (MEDIUM IMPACT)

**Current**: `PlaybackCommand` is cloned on every send
**Optimized**: Evaluate if commands can be moved instead of cloned

**Lines affected**: 2821, 2831, etc.

```rust
// OLD:
match self.command_tx.try_send(command.clone()) {

// NEW (if possible):
match self.command_tx.try_send(command) {
```

**Note**: Only do this if the command isn't needed after the send. Otherwise, keep the clone but document why it's necessary.

## Implementation Checklist

- [ ] 1. Add `arc-swap` dependency (already exists in Cargo.toml)
- [ ] 2. Update `current_device` field type to `Arc<ArcSwap<Arc<str>>>`
- [ ] 3. Update `current_device_id` field type to `Arc<ArcSwap<Option<Arc<str>>>>`
- [ ] 4. Update device initialization code
- [ ] 5. Update `get_current_device()` method
- [ ] 6. Update `get_current_device_id()` method
- [ ] 7. Update all device setter operations
- [ ] 8. Replace all `Arc::clone` calls with explicit syntax
- [ ] 9. Evaluate track cloning in events (Arc wrapper or field extraction)
- [ ] 10. Run benchmarks to verify improvements
- [ ] 11. Run full test suite: `cargo test --workspace`
- [ ] 12. Count remaining clones: `grep -c "\.clone()" playback.rs`

## Expected Outcomes

### Before Optimization
- **Total clones**: 75
- **Hot path clones**: ~30
- **Lock operations**: ~125 (many on device queries)

### After Optimization (Target)
- **Total clones**: <20 in hot paths
- **Device query clones**: 0 (replaced with Arc::load)
- **Track clones in events**: Reduced by 60-80%
- **Lock operations**: Reduced by ~10% (device queries no longer need locks)

### Performance Impact
- **Device queries**: 50-70% faster (no lock contention, no String clone)
- **Event emission**: 30-50% faster (no deep QueueTrack clones)
- **Overall playback overhead**: 10-15% reduction

## Testing Strategy

### 1. Unit Tests
```bash
cd libraries/soul-audio-desktop
cargo test --lib
```

### 2. Integration Tests
```bash
cargo test --test device_handling_test
cargo test --test pause_during_startup_e2e_test -- --include-ignored
```

### 3. Benchmarks
```bash
cargo bench --bench playback_stress
```

### 4. Clone Count Verification
```bash
# Before:
grep -c "\.clone()" libraries/soul-audio-desktop/src/playback.rs  # Should be 75

# After:
grep -c "\.clone()" libraries/soul-audio-desktop/src/playback.rs  # Target: <30 total, <20 in hot paths
```

## Rollback Plan

If issues arise:
1. Revert changes: `git restore libraries/soul-audio-desktop/src/playback.rs`
2. Cherry-pick only the safe optimizations (explicit Arc::clone)
3. Benchmark each optimization individually to identify the problematic change

## Related Tasks

- Task #16: Refactor device switching with lock-free patterns (synergizes with device name optimization)
- Task #22: Instrument all 125 lock operations with metrics (will show improvement from device query optimization)

## References

- ArcSwap docs: https://docs.rs/arc-swap/latest/arc_swap/
- Rust Arc docs: https://doc.rust-lang.org/std/sync/struct.Arc.html
- Lock-free programming: https://preshing.com/20120612/an-introduction-to-lock-free-programming/

---

**Last Updated**: 2026-02-11
**Status**: Ready for implementation
**Priority**: High (Task #18)
