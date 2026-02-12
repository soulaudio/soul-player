# Lock Reduction Roadmap

**Date**: 2026-02-11
**Current Lock Count**: ~55 Mutex instances
**Target**: <20 locks (industry leading)
**Status**: Phase 1-3 Complete, Phase 4 Ready for Implementation

---

## Executive Summary

This document provides a comprehensive strategy to reduce lock contention from ~55 mutexes to <20, moving Soul Player from "good" to "industry-leading" in audio performance. The analysis shows:

- **13 locks are in CRITICAL audio path** (must be lock-free)
- **8 locks are in device management** (high priority)
- **25+ locks are in configuration/getters** (medium priority)
- **9 locks are in metrics/logging** (low priority, acceptable)

**Quick wins** can eliminate ~30 locks with minimal effort. **Medium-term changes** will eliminate another ~15 locks. The remaining ~10 locks are either already optimized (using `try_lock`) or required by OS APIs.

---

## Phase 1: Lock Inventory

### 1.1 Audio Path Locks (CRITICAL - Real-Time Thread)

These locks are hit during audio callbacks and MUST be lock-free:

#### `libraries/soul-audio-desktop/src/playback.rs`

| Line | Lock | Current Usage | Criticality |
|------|------|---------------|-------------|
| 644 | `manager: Arc<Mutex<PlaybackManager>>` | Audio callback reads samples | **CRITICAL** |
| 661 | `resampling_settings: Arc<Mutex<ResamplingSettings>>` | Read-only during playback | HIGH |
| 641 | `stream: Arc<Mutex<Option<Stream>>>` | Only locked for device switching | MEDIUM |

**Analysis**:
- `manager` lock is hit on EVERY audio callback (lines 1482, 1687, 1876)
- Line 1482 already uses `try_lock` with DAC keepalive fallback (good pattern!)
- Lines 1687, 1876 use blocking `lock()` in f32/i16 callbacks
- `resampling_settings` is read-only during playback but uses Mutex

#### `libraries/soul-audio-desktop/src/sources/local.rs`

| Line | Lock | Current Usage | Criticality |
|------|------|---------------|-------------|
| 149 | `shared: Arc<Mutex<SharedState>>` | Decoder thread → audio thread | **CRITICAL** |

**Analysis**:
- Lines 1043, 1129, 1144, 1159 use `try_lock` (EXCELLENT!)
- On contention, outputs silence or returns cached values
- Already optimized, but still counts as a lock

#### `libraries/soul-audio-desktop/src/sources/streaming.rs`

| Line | Lock | Current Usage | Criticality |
|------|------|---------------|-------------|
| 46 | `buffer: Arc<Mutex<Vec<f32>>>` | Download thread → audio thread | **CRITICAL** |
| 61 | `error: Arc<Mutex<Option<String>>>` | Error signaling | MEDIUM |

**Analysis**:
- Used for network streaming (less common than local playback)
- Buffer lock could cause glitches on slow network/contention

#### `libraries/soul-audio-desktop/src/output.rs`

| Line | Lock | Current Usage | Criticality |
|------|------|---------------|-------------|
| 123 | `buffer: Mutex<Arc<Vec<f32>>>` | Shared mode audio callback | **CRITICAL** |

**Analysis**:
- Comment says "Mutex is acceptable here because we only swap the Arc pointer"
- This is a PERFECT candidate for `ArcSwap` (already in dependencies!)

#### `libraries/soul-audio-desktop/src/exclusive.rs`

| Line | Lock | Current Usage | Criticality |
|------|------|---------------|-------------|
| 237 | `buffer: Mutex<Arc<AudioData>>` | Exclusive mode audio callback | **CRITICAL** |

**Analysis**:
- Same pattern as `output.rs` - swap Arc pointer
- Perfect candidate for `ArcSwap`

**Total Critical Path Locks: 6 instances across 5 files**

---

### 1.2 Device Management Locks (HIGH Priority)

#### `libraries/soul-audio-desktop/src/device_monitor_macos.rs`

| Line | Lock | Current Usage | Criticality |
|------|------|---------------|-------------|
| 120 | `previous_devices: StdMutex<Vec<...>>` | CoreAudio callbacks | HIGH |
| 122 | `previous_default: StdMutex<Option<AudioDeviceID>>` | CoreAudio callbacks | HIGH |

**Analysis**:
- Locked from CoreAudio property listener callbacks (lines 262, 399)
- Holds during device list comparison (not in audio path but affects responsiveness)
- Could cause UI lag if device enumeration is slow

#### `libraries/soul-audio-desktop/src/device_monitor_linux.rs`

| Line | Lock | Current Usage | Criticality |
|------|------|---------------|-------------|
| 102 | `devices: Arc<Mutex<Vec<...>>>` | PipeWire registry callbacks | HIGH |
| 106 | `default_sink_name: Arc<Mutex<Option<String>>>` | PipeWire metadata callbacks | HIGH |

**Analysis**:
- PipeWire async callbacks (not audio thread, but device monitoring thread)
- Target: <100ms enumeration on Linux (currently meeting target)

#### `libraries/soul-audio-desktop/src/device_monitor_cpal_fallback.rs`

| Line | Lock | Current Usage | Criticality |
|------|------|---------------|-------------|
| 216 | `previous_devices: Arc<Mutex<Vec<...>>>` | Polling fallback | MEDIUM |
| 377 | `callback_invoked: Arc<Mutex<bool>>` | Test-only flag | LOW |

**Analysis**:
- Fallback device monitor for systems without native APIs
- Polling-based (less performance critical than event-based)
- Line 377 is test-only

#### `libraries/soul-audio-desktop/src/device_monitor_windows.rs`

| Line | Lock | Current Usage | Criticality |
|------|------|---------------|-------------|
| 389 | `watcher: Arc<Mutex<Option<DeviceWatcher>>>` | WinRT device watcher | MEDIUM |

**Analysis**:
- Only locked for stop/cleanup (not hot path)

**Total Device Management Locks: 8 instances (2 test-only)**

---

### 1.3 Configuration Locks (MEDIUM Priority)

These are getters/setters called from UI/command handlers (not audio thread):

#### `libraries/soul-audio-desktop/src/playback.rs` - Configuration API

| Lines | Lock | Operations | Frequency |
|-------|------|-----------|-----------|
| 2491-2542 | `manager.lock()` | State getters (14 methods) | UI polling |
| 3302-3695 | `manager.lock()` | Effect/volume config (30+ methods) | User changes |
| 3535-3629 | `resampling_settings.lock()` | Resampling config (8 methods) | User changes |

**Analysis**:
- ~50+ getter/setter methods that lock `manager` or `resampling_settings`
- Called from UI thread (React → Tauri → Rust)
- Not performance-critical but high lock count

**Examples**:
```rust
// Line 2491: State getter
pub fn get_state(&self) -> soul_playback::PlaybackState {
    self.manager.lock().unwrap().get_state()
}

// Line 2527: Volume getter
pub fn get_volume(&self) -> u8 {
    self.manager.lock().unwrap().get_volume()
}

// Line 3310: Volume leveling setter
pub fn set_volume_leveling_mode(&self, mode: NormalizationMode) {
    let mut manager = self.manager.lock().unwrap();
    manager.set_volume_leveling_mode(mode);
}
```

**Total Configuration Locks: 25+ individual lock acquisitions**

---

### 1.4 Metrics/Logging Locks (LOW Priority - Acceptable)

#### `libraries/soul-audio-desktop/src/sources/metrics.rs`

| Line | Lock | Current Usage | Criticality |
|------|------|---------------|-------------|
| 163 | `locks: Mutex<HashMap<String, Arc<LockMetrics>>>` | Metrics registry | LOW |

**Analysis**:
- Only locked for metrics report generation (every 60s)
- Recording metrics is lock-free (atomic operations)
- Comment says "this is acceptable" - agreed!

**Total Metrics Locks: 1 instance (KEEP)**

---

## Phase 2: Instrumentation Analysis

### 2.1 Existing Metrics Infrastructure

Soul Player already has excellent lock profiling at `libraries/soul-audio-desktop/src/sources/metrics.rs`:

**Current Instrumentation**:
- `LockMetrics`: Atomic counters for attempts, contentions, wait times
- `LockTimer`: Instant-based timing helper
- `GLOBAL_LOCK_METRICS`: Global registry (Lazy static)
- Detection thresholds:
  - Significant contention: >5% of attempts
  - High p99 latency: >10ms
  - Frame duration check: per sample rate/buffer size

**Current Usage**:
```rust
// sources/local.rs line 1041-1043
let timer = LockTimer::start();
let Ok(mut state) = self.shared.try_lock() else {
    let wait_ns = timer.elapsed_ns();
    self.lock_metrics.record_attempt(true, wait_ns);
    // ... output silence ...
};
```

**Observation**: `local.rs` is the ONLY place currently using lock metrics! Need to instrument other hot paths.

### 2.2 Recommended Instrumentation Points

Add lock timing to these critical paths:

1. **`playback.rs` line 1482** - `manager.try_lock()` (f64 callback)
   ```rust
   let timer = LockTimer::start();
   let Ok(mut mgr) = manager.try_lock() else {
       GLOBAL_LOCK_METRICS.record_lock("manager_audio_f64", timer.elapsed());
       // ... keepalive noise ...
   };
   ```

2. **`playback.rs` lines 1687, 1876** - Blocking `manager.lock()` (f32/i16 callbacks)
   ```rust
   let timer = LockTimer::start();
   let mut mgr = manager.lock().unwrap();
   GLOBAL_LOCK_METRICS.record_lock("manager_audio_f32", timer.elapsed());
   ```

3. **`output.rs` buffer lock** - Add metrics to understand real-world contention
4. **Device monitor locks** - Measure impact on UI responsiveness

### 2.3 Questions to Answer with Metrics

Before implementing Phase 4, run instrumentation for 1 week to answer:

1. **What is p99 wait time for `manager` lock in audio callbacks?**
   - Target: <100μs (no audible glitch)
   - Action threshold: >1ms (investigate)

2. **What is contention rate on `local.rs` shared state?**
   - Current: Unknown (metrics exist but need dashboard)
   - Expected: <1% (decoder thread is async)

3. **Which getters are called most frequently from UI?**
   - Candidates: `get_state()`, `get_position()`, `get_current_track()`
   - These should use lock-free reads if called >10Hz

4. **Do device monitor locks ever block for >100ms?**
   - macOS target: <50ms
   - Linux target: <100ms
   - Windows target: <150ms

### 2.4 Metric Collection Strategy

**Option A: Production Telemetry** (Recommended)
- Enable lock metrics in release builds (minimal overhead)
- Log report every 60s to user's log directory
- Aggregate in Sentry/PostHog/etc. (future: telemetry service)

**Option B: Development Profiling**
- Add `--profile-locks` CLI flag to desktop app
- Output real-time lock stats to terminal
- Use during dogfooding sessions

---

## Phase 3: Elimination Strategy

### 3.1 Quick Wins (30 locks → Lock-Free)

#### QW1: Replace `resampling_settings: Arc<Mutex<...>>` with `ArcSwap`

**File**: `libraries/soul-audio-desktop/src/playback.rs:661`

**Current Code**:
```rust
resampling_settings: Arc<Mutex<ResamplingSettings>>,
```

**Change**:
```rust
resampling_settings: ArcSwap<ResamplingSettings>,
```

**Impact**:
- Eliminates lock from audio source creation path
- Read-only during playback, only written by user commands
- **Effort**: Low (1 hour)
- **Files Changed**: 1
- **Locks Eliminated**: 1 (+ 8 lock acquisitions in getters/setters)

**Implementation**:
```rust
// Initialization (line 815)
let resampling_settings = ArcSwap::new(Arc::new(ResamplingSettings::default()));

// Write (line 3535)
pub fn set_resampling_quality(&self, quality: &str) {
    let mut settings = (*self.resampling_settings.load()).clone();
    settings.quality = quality.to_string();
    self.resampling_settings.store(Arc::new(settings));
}

// Read (line 3548)
pub fn get_resampling_quality(&self) -> String {
    self.resampling_settings.load().quality.clone()
}
```

---

#### QW2: Replace `output.rs` buffer Mutex with `ArcSwap`

**File**: `libraries/soul-audio-desktop/src/output.rs:123`

**Current Code**:
```rust
buffer: Mutex<Arc<Vec<f32>>>,
```

**Change**:
```rust
buffer: ArcSwap<Vec<f32>>,
```

**Impact**:
- **CRITICAL**: Removes lock from shared mode audio callback
- Comment already says "Mutex is acceptable because we only swap Arc" - this is the EXACT use case for ArcSwap!
- **Effort**: Low (30 minutes)
- **Files Changed**: 1
- **Locks Eliminated**: 1 (audio path!)

**Implementation**:
```rust
// Definition (line 123)
struct AudioState {
    buffer: ArcSwap<Vec<f32>>,  // Was: Mutex<Arc<Vec<f32>>>
    position: AtomicUsize,
    // ...
}

// Write (outside audio thread)
fn set_samples(&self, samples: Vec<f32>) {
    self.buffer.store(Arc::new(samples));
}

// Read (audio callback)
fn data_callback(&self, output: &mut [f32]) {
    let buffer = self.buffer.load();
    // ... copy samples from buffer ...
}
```

---

#### QW3: Replace `exclusive.rs` buffer Mutex with `ArcSwap`

**File**: `libraries/soul-audio-desktop/src/exclusive.rs:237`

**Current Code**:
```rust
buffer: Mutex<Arc<AudioData>>,
```

**Change**:
```rust
buffer: ArcSwap<AudioData>,
```

**Impact**:
- **CRITICAL**: Removes lock from exclusive mode audio callback
- Identical pattern to `output.rs`
- **Effort**: Low (30 minutes)
- **Files Changed**: 1
- **Locks Eliminated**: 1 (audio path!)

---

#### QW4: Replace `streaming.rs` error Mutex with `ArcSwap<Option<String>>`

**File**: `libraries/soul-audio-desktop/src/sources/streaming.rs:61`

**Current Code**:
```rust
error: Arc<Mutex<Option<String>>>,
```

**Change**:
```rust
error: ArcSwap<Option<String>>,
```

**Impact**:
- Read-only from audio thread, write-only from download thread
- Low frequency writes (only on error)
- **Effort**: Low (15 minutes)
- **Files Changed**: 1
- **Locks Eliminated**: 1

---

#### QW5: Cache frequently-read state in atomics

**File**: `libraries/soul-audio-desktop/src/playback.rs`

**Problem**: UI polls these getters at 10-60 Hz:
- `get_state()` (line 2491)
- `get_volume()` (line 2527)
- `get_position()` (line 2500)

**Solution**: Cache in atomic fields (read-only from UI, updated by manager):

```rust
// Add to DesktopPlayback struct
pub struct DesktopPlayback {
    // ... existing fields ...

    // Cached state (updated by manager, read lock-free by UI)
    cached_state: AtomicU8,  // PlaybackState as u8
    cached_volume: AtomicU8,
    cached_position_secs: AtomicU64,  // f64 as u64 bit pattern
}

impl DesktopPlayback {
    // Lock-free getter for UI
    pub fn get_state_fast(&self) -> PlaybackState {
        let state_u8 = self.cached_state.load(Ordering::Relaxed);
        PlaybackState::from_u8(state_u8)
    }

    // Keep original getter for backward compat (mark deprecated)
    #[deprecated(note = "Use get_state_fast() for lock-free reads")]
    pub fn get_state(&self) -> PlaybackState {
        self.manager.lock().unwrap().get_state()
    }
}
```

**Impact**:
- Eliminates 3 most frequent lock acquisitions
- UI queries become lock-free (instant)
- **Effort**: Medium (2 hours) - need to update cache on state changes
- **Locks Eliminated**: 3 high-frequency acquisitions

**Implementation Note**: Update cache whenever manager state changes:
- After `play()`, `pause()`, `stop()`, `skip_next()`, etc.
- In audio callback position updates (every ~250ms)
- On volume changes

---

#### QW6: Device Monitor - Use Lock-Free Data Structures

**Files**:
- `device_monitor_macos.rs:120-122`
- `device_monitor_linux.rs:102-106`
- `device_monitor_cpal_fallback.rs:216`

**Current Pattern**:
```rust
struct ListenerContext {
    event_sender: mpsc::Sender<DeviceEvent>,
    previous_devices: StdMutex<Vec<(String, AudioDeviceID)>>,
    previous_default: StdMutex<Option<AudioDeviceID>>,
}
```

**Change to**:
```rust
use arc_swap::ArcSwap;

struct ListenerContext {
    event_sender: mpsc::Sender<DeviceEvent>,
    previous_devices: ArcSwap<Vec<(String, AudioDeviceID)>>,
    previous_default: ArcSwap<Option<AudioDeviceID>>,
}
```

**Impact**:
- Device callbacks no longer block on comparison
- UI device list updates are lock-free reads
- **Effort**: Medium (3 hours across 3 files)
- **Files Changed**: 3
- **Locks Eliminated**: 6 (2 per platform)

**Caveat**: Requires `Clone` for device ID types (AudioDeviceID is `Copy`, so OK)

---

**Total Quick Wins**:
- **Locks eliminated**: 13 instances + 10+ high-frequency acquisitions
- **Effort**: 1 day (8 hours)
- **Risk**: Low (drop-in replacements)
- **Audio path locks removed**: 3 critical locks

---

### 3.2 Medium-Term Changes (15 locks → Message Passing)

#### MT1: Replace `manager: Arc<Mutex<PlaybackManager>>` with Lock-Free Queue

**Files**: `libraries/soul-audio-desktop/src/playback.rs:644`

**Problem**: Most contended lock in entire codebase
- Audio callback locks manager every frame (10ms @ 512 samples)
- UI commands lock manager (play, pause, skip, seek, volume)
- ~50 getter/setter methods lock manager
- **Current mitigation**: Line 1482 uses `try_lock` with keepalive fallback (good!)

**Root Cause Analysis**: `PlaybackManager` is a God Object with too many responsibilities:
1. Queue management (not needed in audio callback)
2. Audio sample generation (needed in audio callback)
3. State tracking (read by UI, written by audio thread)
4. Effect processing (needed in audio callback)
5. Volume control (needed in audio callback)

**Solution**: Split `PlaybackManager` into 3 components:

```rust
// Audio thread owns this (no lock needed)
struct AudioEngine {
    source: Option<Box<dyn AudioSource>>,
    effects: EffectChain,
    volume: VolumeController,
    fades: FadeController,
    sample_rate: u32,
}

// UI thread owns this
struct QueueController {
    queue: QueueManager,
    current_track: Option<QueueTrack>,
    shuffle: ShuffleMode,
    repeat: RepeatMode,
}

// Shared via lock-free primitives
struct SharedPlaybackState {
    state: AtomicU8,  // PlaybackState
    position_secs: AtomicU64,  // f64 bit pattern
    volume: AtomicU8,
    // ... other cached fields ...
}
```

**Communication**:
- **UI → Audio**: `crossbeam_channel::bounded` (already used for PlaybackCommand)
- **Audio → UI**: `crossbeam_channel::unbounded` (already used for PlaybackEvent)
- **Cached reads**: Atomic fields (zero-cost)

**Benefits**:
- Audio callback is lock-free (reads atomics, processes samples)
- UI getters are lock-free (read atomics)
- Commands use message passing (non-blocking send)
- Clear ownership boundaries

**Drawbacks**:
- **High effort**: Major refactor (5-10 days)
- **Risk**: Medium (changes core architecture)
- Breaking API change (semver major bump)

**Impact**:
- **Locks eliminated**: 1 mega-lock → 0 locks
- **Lock acquisitions eliminated**: 50+ per second (UI polling + audio)
- **Performance gain**: ~10-20μs per audio callback (no lock overhead)

**Implementation Plan** (future task):
1. Create `AudioEngine` struct with sample processing logic
2. Move queue logic to `QueueController`
3. Add atomic cached state fields
4. Update audio callbacks to read atomics instead of locking
5. Update getters to read atomics
6. Update command handlers to send messages

**Timeline**: Defer to v0.2.0 (major refactor)

---

#### MT2: Replace `streaming.rs` buffer Mutex with Lock-Free Queue

**File**: `libraries/soul-audio-desktop/src/sources/streaming.rs:46`

**Current Code**:
```rust
buffer: Arc<Mutex<Vec<f32>>>,
```

**Change to**:
```rust
use crossbeam_queue::ArrayQueue;

buffer: Arc<ArrayQueue<f32>>,
```

**Impact**:
- **CRITICAL**: Removes lock from streaming audio path
- Download thread pushes, audio thread pops (SPSC pattern)
- **Effort**: Medium (2 hours)
- **Files Changed**: 1
- **Locks Eliminated**: 1 (audio path!)

**Implementation**:
```rust
const BUFFER_SIZE: usize = CHUNK_SIZE * BUFFER_CHUNKS;

pub struct StreamingAudioSource {
    buffer: Arc<ArrayQueue<f32>>,  // Lock-free bounded queue
    // ...
}

impl StreamingAudioSource {
    pub fn new(...) -> Result<Self> {
        let buffer = Arc::new(ArrayQueue::new(BUFFER_SIZE));
        // ...

        // Download thread
        let buffer_clone = buffer.clone();
        thread::spawn(move || {
            for sample in decoded_samples {
                // Non-blocking push (drop old samples if full)
                let _ = buffer_clone.push(sample);
            }
        });

        Ok(Self { buffer, ... })
    }
}

impl AudioSource for StreamingAudioSource {
    fn read_samples(&mut self, output: &mut [f32]) -> Result<usize> {
        let mut samples_read = 0;
        while samples_read < output.len() {
            match self.buffer.pop() {
                Some(sample) => {
                    output[samples_read] = sample;
                    samples_read += 1;
                }
                None => break,  // Buffer empty (underrun)
            }
        }
        Ok(samples_read)
    }
}
```

**Note**: `crossbeam_queue::ArrayQueue` is already a dependency (line 55 of Cargo.toml)

---

#### MT3: Replace `local.rs` shared state Mutex with Lock-Free Queue

**File**: `libraries/soul-audio-desktop/src/sources/local.rs:149`

**Current Code**:
```rust
shared: Arc<Mutex<SharedState>>,
```

**Status**: Already well-optimized! Uses `try_lock` everywhere (lines 1043, 1129, 1144, 1159)

**Observation**: Could still be improved by using lock-free queue for samples:

```rust
struct LocalAudioSource {
    // Decoded samples (lock-free bounded queue)
    output_buffer: Arc<ArrayQueue<f32>>,

    // Metadata (atomic or ArcSwap)
    samples_read: Arc<AtomicUsize>,
    is_eof: Arc<AtomicBool>,
    total_frames: Arc<AtomicUsize>,

    // Decoder thread handle
    _decoder_thread: JoinHandle<()>,
}
```

**Impact**:
- Removes last `try_lock` from audio path
- Decoder thread pushes, audio thread pops (SPSC pattern)
- **Effort**: Medium-High (4 hours) - need to handle seek/pause commands
- **Files Changed**: 1
- **Locks Eliminated**: 1 (audio path!)

**Complexity**: Commands (seek, pause) need to be handled via separate channel to decoder thread

---

**Total Medium-Term Changes**:
- **Locks eliminated**: 3 major locks
- **Effort**: 2-10 days (depending on scope)
- **Risk**: Medium (architectural changes)
- **Audio path locks removed**: 3 remaining critical locks

---

### 3.3 Long-Term Optimizations (Architectural Changes)

#### LT1: Lock-Free PlaybackManager (The Big One)

See MT1 above - this is the ultimate goal but requires major refactor.

**Timeline**: v0.2.0 (Q2 2026)

---

#### LT2: Consider `parking_lot` for Remaining Locks

**Context**: `parking_lot::Mutex` is faster than `std::sync::Mutex`:
- Smaller size (1 byte vs 40 bytes on 64-bit)
- Faster locking (1-2x depending on contention)
- No poisoning overhead

**Recommendation**: Replace `std::sync::Mutex` with `parking_lot::Mutex` for remaining locks

**Files to Change**:
- Device monitor mutexes (not in audio path, but faster is better)
- Test-only mutexes

**Impact**:
- 10-30% faster lock acquisition for remaining locks
- **Effort**: Low (1 hour) - search/replace + add dependency
- **Risk**: Very low (drop-in replacement)

**Add to Cargo.toml**:
```toml
parking_lot = "0.12"
```

**Note**: Don't use `parking_lot` for audio path locks - eliminate them entirely instead!

---

### 3.4 Locks to KEEP (Acceptable)

#### KEEP1: `metrics.rs` lock registry (line 163)

**Reason**:
- Locked only during report generation (every 60s)
- Recording is lock-free (atomic operations)
- Not in hot path

#### KEEP2: Device monitor context mutexes (if not converted to ArcSwap)

**Reason**:
- Locked from OS callbacks (not audio thread)
- Low frequency (only on device add/remove)
- Contention is rare

#### KEEP3: Test-only mutexes

**Reason**: Performance not critical in tests

---

## Phase 4: Implementation Roadmap

### Sprint 1: Quick Wins (1-2 days)

**Goal**: Eliminate 13 locks with low-risk changes

**Tasks**:
1. ✅ Add `ArcSwap` usage to `playback.rs` for `resampling_settings` (1 hour)
   - Lines to change: 661 (definition), 815 (init), 3535-3629 (getters/setters)
   - Test: UI resampling config changes during playback

2. ✅ Replace `output.rs` buffer Mutex with `ArcSwap` (30 min)
   - Lines to change: 123 (definition), 137 (init), buffer access in callbacks
   - Test: Shared mode playback (default on most systems)

3. ✅ Replace `exclusive.rs` buffer Mutex with `ArcSwap` (30 min)
   - Lines to change: 237 (definition), 253 (init), buffer access in callbacks
   - Test: Exclusive mode playback (ASIO/WASAPI)

4. ✅ Replace `streaming.rs` error Mutex with `ArcSwap` (15 min)
   - Lines to change: 61 (definition), 81 (init), error check in audio source
   - Test: Network streaming error handling

5. ✅ Replace device monitor Mutexes with `ArcSwap` (3 hours)
   - Files: `device_monitor_macos.rs`, `device_monitor_linux.rs`, `device_monitor_cpal_fallback.rs`
   - Test: Device hotplug on all platforms

**Testing**:
- Run full audio E2E test suite
- Manual test: Play audio while changing resampling settings
- Manual test: Hotplug USB DAC during playback

**Success Metrics**:
- All tests pass
- No audible glitches
- Lock count: 55 → 42 (~24% reduction)

---

### Sprint 2: Cached State Atomics (2-3 days)

**Goal**: Eliminate high-frequency lock acquisitions from UI polling

**Tasks**:
1. ✅ Add cached atomic fields to `DesktopPlayback` (1 hour)
   - Fields: `cached_state`, `cached_volume`, `cached_position_secs`
   - Initialize in `new()` method

2. ✅ Add lock-free getter methods (1 hour)
   - `get_state_fast()`, `get_volume_fast()`, `get_position_fast()`
   - Mark old getters as `#[deprecated]`

3. ✅ Update cache on state changes (4 hours)
   - After command handlers (play, pause, stop, etc.)
   - In audio callbacks (position updates)
   - On volume/state changes from manager

4. ✅ Update UI code to use fast getters (2 hours)
   - `applications/desktop/src/providers/TauriBackendProvider.tsx`
   - Find all `invoke()` calls to `get_state`, `get_volume`, `get_position`
   - Replace with new commands that use fast getters

**Testing**:
- UI state updates during playback
- Position slider accuracy
- Verify cache coherency (no stale reads)

**Success Metrics**:
- UI remains responsive during heavy playback
- Position updates smooth (60 FPS)
- Lock acquisitions: ~50/sec → ~5/sec (90% reduction in high-freq locks)

---

### Sprint 3: Lock-Free Audio Buffers (3-5 days)

**Goal**: Remove remaining Mutex from audio sample paths

**Tasks**:
1. ✅ Replace `streaming.rs` buffer with `ArrayQueue` (2 hours)
   - Use `crossbeam_queue::ArrayQueue<f32>`
   - Update download thread (push) and audio source (pop)
   - Test network streaming

2. ✅ Replace `local.rs` shared state with lock-free structures (6 hours)
   - Output buffer: `ArrayQueue<f32>`
   - Metadata: Atomic types
   - Commands: Separate channel to decoder thread
   - Test local file playback, seek, pause/resume

3. ✅ Add comprehensive lock metrics (4 hours)
   - Instrument `manager` lock in audio callbacks
   - Add metrics to remaining locks
   - Create metrics dashboard/logging

**Testing**:
- Full E2E audio test suite
- Stress test: Rapid skip/seek during playback
- Memory test: Check for buffer overflows/underruns

**Success Metrics**:
- Zero `Mutex` in audio sample path
- All E2E tests pass
- p99 audio callback latency <100μs
- Lock count: 42 → 32 (~42% total reduction)

---

### Sprint 4: Instrumentation & Validation (1 week)

**Goal**: Validate improvements and identify remaining bottlenecks

**Tasks**:
1. ✅ Enable production lock metrics (2 hours)
   - Add metrics collection in release builds
   - Log reports to user log directory
   - Add `--profile-locks` dev mode

2. ✅ Dogfood for 1 week (1 week)
   - Use Soul Player as daily driver
   - Monitor logs for lock contention warnings
   - Collect p99 latency data

3. ✅ Analyze results and prioritize next phase (4 hours)
   - Identify remaining hot locks
   - Decide on MT1 (PlaybackManager refactor) timeline
   - Document findings

**Success Metrics**:
- p99 lock wait time <1ms for all locks
- No significant contention detected (>5%)
- Clear data for next optimization phase

---

### Future Sprints: PlaybackManager Refactor (v0.2.0)

**Goal**: Eliminate `manager: Arc<Mutex<PlaybackManager>>` (the big one!)

**Blocked On**: Sprint 4 metrics data

**Estimated Effort**: 2-3 weeks (major refactor)

**Approach**: See MT1 above (split into AudioEngine + QueueController + SharedState)

**Timeline**: Q2 2026 (after v0.1.x stabilization)

---

## Summary Tables

### Lock Elimination by Phase

| Phase | Locks Eliminated | Effort | Risk | Audio Path |
|-------|-----------------|--------|------|-----------|
| Sprint 1 (Quick Wins) | 13 | 1-2 days | Low | 3 critical |
| Sprint 2 (Cached State) | 3 high-freq | 2-3 days | Low | 0 |
| Sprint 3 (Lock-Free Buffers) | 2-3 | 3-5 days | Medium | 2-3 critical |
| **Total (v0.1.x)** | **18-19** | **2 weeks** | **Low-Med** | **5-6 critical** |
| Future (PlaybackManager) | 25+ | 2-3 weeks | High | All remaining |
| **Grand Total** | **43-44** | **4-5 weeks** | **Medium** | **All** |

### Final Lock Count Projection

| Category | Current | After Sprints 1-3 | After Manager Refactor | Target |
|----------|---------|------------------|----------------------|--------|
| Audio Path | 13 | 7-8 | 0 | 0 |
| Device Management | 8 | 2 | 2 | <5 |
| Configuration | 25+ | 5 | 0 | 0 |
| Metrics/Tests | 9 | 9 | 9 | <15 |
| **TOTAL** | **~55** | **~32** | **~11** | **<20** |

**Result**: After Sprints 1-3 (2 weeks), we'll have ~32 locks (42% reduction). After PlaybackManager refactor (v0.2.0), we'll reach ~11 locks (80% reduction, **EXCEEDING** the <20 goal).

---

## Risk Assessment

### Low Risk (Safe to implement now)
- ✅ Quick Wins (Sprint 1): All are drop-in replacements
- ✅ Cached State (Sprint 2): Additive changes, doesn't break existing API

### Medium Risk (Needs thorough testing)
- ⚠️ Lock-Free Buffers (Sprint 3): Changes audio sample flow
- ⚠️ Device monitor ArcSwap: OS callback timing assumptions

### High Risk (Defer to major version)
- 🔴 PlaybackManager refactor (MT1): Breaking API changes, core architecture

**Mitigation Strategy**:
1. Implement Quick Wins first (build confidence)
2. Extensive E2E testing after each sprint
3. Use feature flags for risky changes (`--features lock-free-buffers`)
4. Dogfood on developer machines before release
5. Monitor production metrics after release (Sentry alerts)

---

## Success Criteria

### Phase 1-3 Success (v0.1.x)
- ✅ Lock count reduced by >40% (55 → 32)
- ✅ Zero `Mutex` in audio sample read paths
- ✅ p99 lock wait time <1ms for all remaining locks
- ✅ All E2E tests pass
- ✅ No regressions in audio quality or UI responsiveness

### Phase 4 Success (v0.2.0)
- ✅ Lock count <20 (target achieved)
- ✅ Audio thread is 100% lock-free
- ✅ UI getters are 100% lock-free (atomic reads only)
- ✅ Production metrics show zero lock contention

---

## Appendix A: Lock-Free Alternatives Reference

| Current | Lock-Free Alternative | Use Case | Crate |
|---------|---------------------|----------|-------|
| `Mutex<T>` | `ArcSwap<T>` | Read-heavy, infrequent writes, full struct updates | `arc-swap` |
| `Mutex<Vec<T>>` | `ArrayQueue<T>` | SPSC producer/consumer (bounded) | `crossbeam-queue` |
| `Mutex<Vec<T>>` | `SegQueue<T>` | MPMC queue (unbounded) | `crossbeam-queue` |
| `Mutex<bool>` | `AtomicBool` | Single boolean flag | `std::sync::atomic` |
| `Mutex<u32/u64>` | `AtomicU32/U64` | Single integer counter/value | `std::sync::atomic` |
| `Mutex<f32/f64>` | `AtomicU32/U64` + bit pattern | Single float (store as bits) | `std::sync::atomic` |
| `Mutex<Option<T>>` | `ArcSwap<Option<T>>` | Optional value, infrequent updates | `arc-swap` |
| `RwLock<T>` | `ArcSwap<T>` | Read-heavy, write-rare, full updates | `arc-swap` |
| `std::sync::Mutex` | `parking_lot::Mutex` | Must keep lock, need faster impl | `parking_lot` |

---

## Appendix B: Code Audit - All Mutex Locations

**Generated**: 2026-02-11 (automated grep)

### Audio Path (CRITICAL)
1. `playback.rs:641` - `stream: Arc<Mutex<Option<Stream>>>`
2. `playback.rs:644` - `manager: Arc<Mutex<PlaybackManager>>`
3. `playback.rs:661` - `resampling_settings: Arc<Mutex<ResamplingSettings>>`
4. `sources/local.rs:149` - `shared: Arc<Mutex<SharedState>>`
5. `sources/streaming.rs:46` - `buffer: Arc<Mutex<Vec<f32>>>`
6. `sources/streaming.rs:61` - `error: Arc<Mutex<Option<String>>>`
7. `output.rs:123` - `buffer: Mutex<Arc<Vec<f32>>>`
8. `exclusive.rs:237` - `buffer: Mutex<Arc<AudioData>>`

### Device Management
9. `device_monitor_macos.rs:120` - `previous_devices: StdMutex<Vec<...>>`
10. `device_monitor_macos.rs:122` - `previous_default: StdMutex<Option<...>>`
11. `device_monitor_linux.rs:102` - `devices: Arc<Mutex<Vec<...>>>`
12. `device_monitor_linux.rs:106` - `default_sink_name: Arc<Mutex<Option<String>>>`
13. `device_monitor_cpal_fallback.rs:216` - `previous_devices: Arc<Mutex<Vec<...>>>`
14. `device_monitor_windows.rs:389` - `watcher: Arc<Mutex<Option<DeviceWatcher>>>`

### Test-Only
15. `device_monitor_macos.rs:1200` - `callback_invoked: Arc<Mutex<bool>>` (test)
16. `device_monitor_cpal_fallback.rs:377` - `callback_invoked: Arc<Mutex<bool>>` (test)

### Metrics (Keep)
17. `sources/metrics.rs:163` - `locks: Mutex<HashMap<...>>` (metrics registry)

### Backup File (Ignore)
18-28. `playback.rs.bak:*` - (backup file, ignore)

**Total**: 17 distinct lock instances (excluding backups and test-only)

**Note**: This count (17) differs from "~55" because:
- The 55 count includes EVERY `manager.lock()` call site (~50 getters/setters)
- This audit counts lock INSTANCES (Arc<Mutex<>> definitions)
- Both perspectives are valid: instances for refactoring, call sites for runtime impact

---

## Appendix C: References

### Internal Documentation
- `libraries/soul-audio-desktop/src/sources/metrics.rs` - Lock profiling infrastructure
- `libraries/soul-audio-desktop/src/sources/local.rs` - Example of `try_lock` pattern (lines 1043+)
- `CLAUDE.md` Section 4 - "Audio Safety: No Allocations" (applies to locks too)

### External References
- [ArcSwap Crate Docs](https://docs.rs/arc-swap) - Lock-free Arc swapping
- [Crossbeam Queue Docs](https://docs.rs/crossbeam-queue) - Lock-free queues
- [Real-Time Audio Programming 101](http://www.rossbencina.com/code/real-time-audio-programming-101-time-waits-for-nothing) - Classic article on lock-free audio
- [Lock-Free Programming](https://preshing.com/20120612/an-introduction-to-lock-free-programming/) - Excellent intro to lock-free techniques

---

**Document Status**: ✅ Complete - Ready for Implementation
**Next Step**: Create GitHub issue for Sprint 1 (Quick Wins)
**Owner**: TBD
**Target Release**: v0.1.11 (Sprints 1-3), v0.2.0 (PlaybackManager refactor)
