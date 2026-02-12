# API Simplification Plan - PlaybackManager

**Target**: Reduce public API from 111 methods to <30 methods
**Status**: Analysis Complete
**Date**: 2026-02-11

---

## Executive Summary

The `PlaybackManager` API has grown to **111 public methods**, far exceeding typical coordinator patterns. Analysis shows:

- **Only 25 methods** are actually used by applications
- **83% of methods** are component internals that leaked into public API
- **Heavy duplication** exists between manager and components
- **Clear consolidation paths** exist through builder/command patterns

**Recommendation**: Target **28 essential methods** (75% reduction) through strategic refactoring.

---

## Phase 1: Current State Audit

### Method Count by Module

| Module | Public Methods | Should Be Public |
|--------|----------------|------------------|
| `manager.rs` | 111 | 28 |
| `audio_pipeline.rs` | 83 | 8 |
| `queue_manager.rs` | 49 | 12 |
| `state_manager.rs` | 24 | 6 |
| `fade_controller.rs` | 22 | 4 |
| `volume_controller.rs` | 8 | 5 |
| **Total** | **297** | **63** |

### Actual Usage Analysis

From `applications/desktop/src-tauri/src/main.rs` (real usage):

```
Used Methods (25 total):
  - play, pause, stop (3)
  - next, previous, seek (3)
  - set_volume, mute, unmute (3)
  - set_shuffle, get_shuffle, cycle_shuffle (3)
  - set_repeat, get_repeat, cycle_repeat (3)
  - load_playlist, add_to_queue_end, clear_queue (3)
  - clear_play_next, clear_add_to_queue (2)
  - skip_to_queue_index, get_queue (2)
  - has_next, has_previous (2)
  - get_state (1)
```

**Unused in Applications**: 86 methods (77%)

---

## Phase 2: Redundancy Analysis

### 1. Component Leakage (Biggest Issue)

**Problem**: Component methods are duplicated in `PlaybackManager`

#### Audio Pipeline Leakage (35 methods)

```rust
// Loudness normalization (14 methods) - Should be AudioSettings
pub fn set_volume_leveling_mode(&mut self, mode: NormalizationMode) { /* ... */ }
pub fn get_volume_leveling_mode(&self) -> NormalizationMode { /* ... */ }
pub fn set_track_gain(&mut self, gain_db: f64, peak_dbfs: f64) { /* ... */ }
pub fn set_album_gain(&mut self, gain_db: f64, peak_dbfs: f64) { /* ... */ }
pub fn clear_loudness_gains(&mut self) { /* ... */ }
pub fn set_loudness_preamp(&mut self, preamp_db: f64) { /* ... */ }
pub fn get_loudness_preamp(&self) -> f64 { /* ... */ }
pub fn set_prevent_clipping(&mut self, prevent: bool) { /* ... */ }
pub fn get_effective_gain_db(&mut self) -> f64 { /* ... */ }
pub fn reset_loudness_normalizer(&mut self) { /* ... */ }

// Output limiter (7 methods) - Should be AudioSettings
pub fn set_output_limiter_lookahead(&mut self, preset: LookaheadPreset) { /* ... */ }
pub fn get_output_limiter_lookahead(&self) -> LookaheadPreset { /* ... */ }
pub fn set_output_limiter_lookahead_ms(&mut self, lookahead_ms: f32) { /* ... */ }
pub fn set_output_limiter_threshold_db(&mut self, threshold_db: f32) { /* ... */ }
pub fn get_output_limiter_gain_reduction_db(&self) -> f32 { /* ... */ }
pub fn get_output_limiter_latency(&self) -> usize { /* ... */ }
pub fn reset_output_limiter(&mut self) { /* ... */ }

// Headroom (9 methods) - Should be AudioSettings
pub fn set_headroom_mode(&mut self, mode: HeadroomMode) { /* ... */ }
pub fn get_headroom_mode(&self) -> HeadroomMode { /* ... */ }
pub fn set_headroom_replaygain_db(&mut self, gain_db: f64) { /* ... */ }
pub fn set_headroom_preamp_db(&mut self, preamp_db: f64) { /* ... */ }
pub fn set_headroom_eq_boost_db(&mut self, boost_db: f64) { /* ... */ }
pub fn set_headroom_additional_gain_db(&mut self, gain_db: f64) { /* ... */ }
pub fn get_headroom_total_gain_db(&self) -> f64 { /* ... */ }
pub fn get_headroom_attenuation_db(&mut self) -> f64 { /* ... */ }
pub fn set_headroom_enabled(&mut self, enabled: bool) { /* ... */ }
pub fn is_headroom_enabled(&self) -> bool { /* ... */ }
pub fn reset_headroom(&mut self) { /* ... */ }
pub fn clear_headroom_track_gains(&mut self) { /* ... */ }

// Crossfade details (5 methods) - Low-level
pub fn set_crossfade_curve(&mut self, curve: FadeCurve) { /* ... */ }
pub fn get_crossfade_curve(&self) -> FadeCurve { /* ... */ }
pub fn get_crossfade_state(&self) -> CrossfadeState { /* ... */ }
pub fn get_crossfade_progress(&self) -> f32 { /* ... */ }
pub fn is_crossfading(&self) -> bool { /* ... */ }
```

**Action**: Move to `AudioSettings` builder/config struct

#### Queue Manager Duplication (8 methods)

```rust
// Redundant accessors
pub fn get_queue(&self) -> Vec<&QueueTrack> { /* ... */ }
pub fn queue_len(&self) -> usize { /* ... */ }
pub fn get_queue_length(&self) -> usize { /* ... */ }  // Same as queue_len!

// Redundant shuffle
pub fn set_shuffle(&mut self, mode: ShuffleMode) { /* ... */ }
pub fn get_shuffle(&self) -> ShuffleMode { /* ... */ }
pub fn get_shuffle_mode(&self) -> ShuffleMode { /* ... */ }  // Duplicate!
pub fn cycle_shuffle(&mut self) -> ShuffleMode { /* ... */ }
```

**Action**: Consolidate getters, keep only `get_queue()` and `queue_len()`

#### Internal Implementation Details (12 methods)

```rust
// Circuit breaker - internal error handling
pub fn record_track_load_failure(&mut self) -> bool { /* ... */ }
pub fn should_retry_after_circuit_open(&mut self) -> bool { /* ... */ }
pub fn reset_circuit_breaker(&mut self) { /* ... */ }

// Low-level audio configuration
pub fn set_sample_rate(&mut self, sample_rate: u32) { /* ... */ }
pub fn get_sample_rate(&self) -> u32 { /* ... */ }
pub fn set_output_channels(&mut self, channels: u16) { /* ... */ }

// Internal pipeline details
pub fn set_audio_source(&mut self, source: Box<dyn AudioSource>) { /* ... */ }
pub fn set_next_source(&mut self, source: Box<dyn AudioSource>, track: QueueTrack) { /* ... */ }
pub fn is_source_compatible(&self, source: &dyn AudioSource) -> bool { /* ... */ }
pub fn has_next_source(&self) -> bool { /* ... */ }
pub fn time_until_crossfade(&self) -> Option<Duration> { /* ... */ }
pub fn should_prepare_next_track(&self) -> bool { /* ... */ }
```

**Action**: Make `pub(crate)` or move to platform layer

### 2. Naming Inconsistencies

```rust
// Inconsistent prefixes
pub fn get_volume(&self) -> u8           // get_*
pub fn volume(&self) -> u8               // no prefix (better)
pub fn is_muted(&self) -> bool           // is_*

// Inconsistent queue methods
pub fn get_queue(&self)                  // get_*
pub fn queue_len(&self)                  // no prefix
pub fn get_queue_length(&self)           // get_* + different name

// Inconsistent clear methods
pub fn clear_queue(&mut self)
pub fn clear_play_next(&mut self)
pub fn clear_add_to_queue(&mut self)     // Should be clear_queued_later
```

### 3. Lazy Queue API Confusion

```rust
// Exposed internal loading mechanism
pub fn set_lazy_context(&mut self, context: QueueContext, fetcher: F) { /* ... */ }
pub fn clear_lazy_context(&mut self) { /* ... */ }
pub fn get_lazy_state(&self) -> Option<&LazyQueueState> { /* ... */ }
pub fn check_batch_loading(&mut self) -> Option<(usize, usize)> { /* ... */ }
pub fn check_jump_loading(&mut self, target_index: usize) -> Option<(usize, usize)> { /* ... */ }
```

**Action**: Should be `pub(crate)` - platform layer concern

---

## Phase 3: Proposed Simplified API

### Target: 28 Essential Methods

#### A. Core Playback (6 methods)
```rust
pub fn play(&mut self) -> Result<()>
pub fn pause(&mut self)
pub fn stop(&mut self)
pub fn next(&mut self) -> Result<()>
pub fn previous(&mut self) -> Result<()>
pub fn seek_to(&mut self, position: Duration) -> Result<()>
```

#### B. Volume Control (4 methods)
```rust
pub fn set_volume(&mut self, level: u8)
pub fn volume(&self) -> u8
pub fn set_muted(&mut self, muted: bool)
pub fn is_muted(&self) -> bool
```
*Remove: `mute()`, `unmute()`, `toggle_mute()` - use `set_muted()` instead*

#### C. Queue Management (8 methods)
```rust
pub fn load_playlist(&mut self, tracks: Vec<QueueTrack>, start_index: usize)
pub fn add_to_queue_next(&mut self, track: QueueTrack)
pub fn add_to_queue_end(&mut self, track: QueueTrack)
pub fn skip_to_index(&mut self, index: usize) -> Result<()>
pub fn remove_from_queue(&mut self, index: usize) -> Result<QueueTrack>
pub fn clear_queue(&mut self)
pub fn queue(&self) -> Vec<&QueueTrack>
pub fn queue_length(&self) -> usize
```
*Remove: `clear_play_next()`, `clear_add_to_queue()`, `get_queue_length()` duplicates*

#### D. Playback Modes (4 methods)
```rust
pub fn set_shuffle(&mut self, mode: ShuffleMode)
pub fn shuffle(&self) -> ShuffleMode
pub fn set_repeat(&mut self, mode: RepeatMode)
pub fn repeat(&self) -> RepeatMode
```
*Remove: `cycle_shuffle()`, `cycle_repeat()` - UI concern, not core API*

#### E. State Queries (6 methods)
```rust
pub fn state(&self) -> PlaybackState
pub fn current_track(&self) -> Option<&QueueTrack>
pub fn position(&self) -> Duration
pub fn duration(&self) -> Option<Duration>
pub fn has_next(&self) -> bool
pub fn has_previous(&self) -> bool
```
*Remove: `get_history()`, `peek_next_queue_track()` - rarely used*

---

## Phase 4: New API Patterns

### Pattern 1: Settings Builder (Replaces 35 methods)

```rust
// Current: 35 individual methods
manager.set_volume_leveling_mode(NormalizationMode::Album);
manager.set_loudness_preamp(-6.0);
manager.set_output_limiter_threshold_db(-1.0);
manager.set_headroom_mode(HeadroomMode::Auto);
manager.set_crossfade_duration(3000);
manager.set_crossfade_curve(FadeCurve::EqualPower);
// ... 29 more methods

// Proposed: Single settings struct
let settings = AudioSettings::builder()
    .loudness(LoudnessSettings {
        mode: NormalizationMode::Album,
        preamp_db: -6.0,
        prevent_clipping: true,
    })
    .limiter(LimiterSettings {
        threshold_db: -1.0,
        lookahead: LookaheadPreset::Balanced,
    })
    .headroom(HeadroomSettings {
        mode: HeadroomMode::Auto,
    })
    .crossfade(CrossfadeSettings {
        enabled: true,
        duration_ms: 3000,
        curve: FadeCurve::EqualPower,
        on_skip: true,
    })
    .build();

manager.apply_settings(settings);  // Single method
```

**Benefit**: 35 methods → 1 method + typed config

### Pattern 2: Command Pattern (Alternative)

```rust
pub enum PlaybackCommand {
    // Core control
    Play,
    Pause,
    Stop,
    Next,
    Previous,
    SeekTo(Duration),

    // Queue operations
    LoadPlaylist { tracks: Vec<QueueTrack>, start_index: usize },
    AddToQueueNext(QueueTrack),
    AddToQueueEnd(QueueTrack),
    RemoveFromQueue(usize),
    ClearQueue,

    // Configuration
    SetVolume(u8),
    SetMuted(bool),
    SetShuffle(ShuffleMode),
    SetRepeat(RepeatMode),
}

pub fn execute(&mut self, command: PlaybackCommand) -> Result<()> {
    // Single entry point, easier to log/test
}
```

**Benefit**: Single entry point, easier middleware/interceptors

### Pattern 3: Facade for Advanced Features

```rust
// Main API stays simple (28 methods)
impl PlaybackManager {
    pub fn play(&mut self) -> Result<()> { /* ... */ }
    // ... 27 other essential methods

    // Advanced features through facade
    pub fn audio_settings(&mut self) -> AudioSettingsFacade<'_> {
        AudioSettingsFacade { manager: self }
    }

    pub fn lazy_queue(&mut self) -> LazyQueueFacade<'_> {
        LazyQueueFacade { queue: &mut self.queue }
    }
}

// Advanced APIs grouped logically
impl<'a> AudioSettingsFacade<'a> {
    pub fn loudness(&mut self) -> LoudnessFacade<'_> { /* ... */ }
    pub fn limiter(&mut self) -> LimiterFacade<'_> { /* ... */ }
    pub fn headroom(&mut self) -> HeadroomFacade<'_> { /* ... */ }
}
```

**Benefit**: Clear separation, discoverability, backward compatibility

---

## Phase 5: Migration Strategy

### Step 1: Deprecate Redundant Methods (Week 1)

```rust
#[deprecated(since = "0.9.0", note = "Use `set_muted(true)` instead")]
pub fn mute(&mut self) { self.set_muted(true); }

#[deprecated(since = "0.9.0", note = "Use `set_muted(false)` instead")]
pub fn unmute(&mut self) { self.set_muted(false); }

#[deprecated(since = "0.9.0", note = "Use `queue_length()` instead")]
pub fn get_queue_length(&self) -> usize { self.queue_length() }

#[deprecated(since = "0.9.0", note = "Use `shuffle()` instead")]
pub fn get_shuffle_mode(&self) -> ShuffleMode { self.shuffle() }
```

**Removes**: 15 duplicate methods

### Step 2: Make Internal APIs Private (Week 2)

```rust
// Change from pub to pub(crate)
pub(crate) fn set_audio_source(&mut self, source: Box<dyn AudioSource>) { /* ... */ }
pub(crate) fn set_next_source(&mut self, source: Box<dyn AudioSource>, track: QueueTrack) { /* ... */ }
pub(crate) fn record_track_load_failure(&mut self) -> bool { /* ... */ }
pub(crate) fn should_retry_after_circuit_open(&mut self) -> bool { /* ... */ }
pub(crate) fn reset_circuit_breaker(&mut self) { /* ... */ }
pub(crate) fn set_sample_rate(&mut self, sample_rate: u32) { /* ... */ }
pub(crate) fn set_output_channels(&mut self, channels: u16) { /* ... */ }
pub(crate) fn process_audio(&mut self, output: &mut [f32]) -> Result<usize> { /* ... */ }
pub(crate) fn set_lazy_context(&mut self, ...) { /* ... */ }
pub(crate) fn check_batch_loading(&mut self) -> Option<(usize, usize)> { /* ... */ }
pub(crate) fn check_jump_loading(&mut self, target_index: usize) -> Option<(usize, usize)> { /* ... */ }
```

**Removes**: 25 methods from public API (still available to platform layer)

### Step 3: Introduce AudioSettings (Week 3)

```rust
// New module: src/audio_settings.rs
#[derive(Debug, Clone)]
pub struct AudioSettings {
    pub loudness: Option<LoudnessSettings>,
    pub limiter: Option<LimiterSettings>,
    pub headroom: Option<HeadroomSettings>,
    pub crossfade: Option<CrossfadeSettings>,
}

impl AudioSettings {
    pub fn builder() -> AudioSettingsBuilder { /* ... */ }
}

// Update PlaybackManager
impl PlaybackManager {
    pub fn apply_audio_settings(&mut self, settings: AudioSettings) {
        if let Some(loudness) = settings.loudness {
            self.audio.set_volume_leveling_mode(loudness.mode);
            self.audio.set_loudness_preamp(loudness.preamp_db);
            self.audio.set_prevent_clipping(loudness.prevent_clipping);
        }
        // ... apply other settings
    }

    pub fn audio_settings(&self) -> AudioSettings {
        AudioSettings {
            loudness: Some(LoudnessSettings {
                mode: self.audio.get_volume_leveling_mode(),
                preamp_db: self.audio.get_loudness_preamp(),
                prevent_clipping: true, // stored in audio pipeline
            }),
            // ... other settings
        }
    }
}
```

**Removes**: 35 granular audio settings methods

### Step 4: Deprecate Old Audio Settings (Week 4)

```rust
#[deprecated(since = "0.9.0", note = "Use `apply_audio_settings()` with AudioSettings::builder()")]
pub fn set_volume_leveling_mode(&mut self, mode: NormalizationMode) { /* ... */ }

#[deprecated(since = "0.9.0", note = "Use `apply_audio_settings()` with AudioSettings::builder()")]
pub fn set_loudness_preamp(&mut self, preamp_db: f64) { /* ... */ }
// ... 33 more deprecations
```

### Step 5: Remove Deprecated APIs (v1.0.0)

- Remove all `#[deprecated]` methods
- Final public API: **28 methods**
- Advanced features: `AudioSettings`, `LazyQueue` facades

---

## Phase 6: Documentation

### Module-Level Docs (Updated)

```rust
//! # Soul Playback - Simple, Powerful Music Playback
//!
//! PlaybackManager provides a clean, focused API for music playback:
//!
//! ## Core Operations (6 methods)
//! - `play()`, `pause()`, `stop()`
//! - `next()`, `previous()`, `seek_to()`
//!
//! ## Volume Control (4 methods)
//! - `set_volume()`, `volume()`
//! - `set_muted()`, `is_muted()`
//!
//! ## Queue Management (8 methods)
//! - `load_playlist()`, `add_to_queue_next()`, `add_to_queue_end()`
//! - `skip_to_index()`, `remove_from_queue()`, `clear_queue()`
//! - `queue()`, `queue_length()`
//!
//! ## Playback Modes (4 methods)
//! - `set_shuffle()`, `shuffle()`
//! - `set_repeat()`, `repeat()`
//!
//! ## State Queries (6 methods)
//! - `state()`, `current_track()`, `position()`, `duration()`
//! - `has_next()`, `has_previous()`
//!
//! ## Advanced Configuration
//! - Audio settings: `apply_audio_settings(AudioSettings::builder()...)`
//! - Lazy queue: Platform-specific (not exposed in public API)
//!
//! # Example
//! ```rust
//! use soul_playback::{PlaybackManager, PlaybackConfig, QueueTrack, ShuffleMode};
//!
//! let mut manager = PlaybackManager::new(PlaybackConfig::default());
//!
//! // Load playlist
//! manager.load_playlist(tracks, 0);
//!
//! // Configure playback
//! manager.set_volume(80);
//! manager.set_shuffle(ShuffleMode::Smart);
//!
//! // Control playback
//! manager.play()?;
//! manager.next()?;
//!
//! // Query state
//! println!("Now playing: {:?}", manager.current_track());
//! println!("Position: {:?} / {:?}", manager.position(), manager.duration());
//! ```
```

### Migration Guide

```markdown
# Migration Guide: v0.8 → v0.9

## Deprecated Methods

### Volume Control
- `mute()` → `set_muted(true)`
- `unmute()` → `set_muted(false)`
- `toggle_mute()` → `set_muted(!manager.is_muted())`
- `get_volume()` → `volume()`

### Queue Management
- `get_queue_length()` → `queue_length()`
- `get_queue()` → `queue()`

### Playback Modes
- `get_shuffle_mode()` → `shuffle()`
- `get_shuffle()` → `shuffle()`
- `cycle_shuffle()` → UI should cycle and call `set_shuffle()`

### State Queries
- `get_state()` → `state()`
- `get_current_track()` → `current_track()`
- `get_position()` → `position()`
- `get_duration()` → `duration()`

## Audio Settings Migration

### Before (v0.8)
```rust
manager.set_volume_leveling_mode(NormalizationMode::Album);
manager.set_loudness_preamp(-6.0);
manager.set_prevent_clipping(true);
manager.set_output_limiter_threshold_db(-1.0);
manager.set_output_limiter_lookahead(LookaheadPreset::Balanced);
manager.set_headroom_mode(HeadroomMode::Auto);
manager.set_crossfade_enabled(true);
manager.set_crossfade_duration(3000);
manager.set_crossfade_curve(FadeCurve::EqualPower);
```

### After (v0.9)
```rust
use soul_playback::{AudioSettings, LoudnessSettings, LimiterSettings};

let settings = AudioSettings::builder()
    .loudness(LoudnessSettings {
        mode: NormalizationMode::Album,
        preamp_db: -6.0,
        prevent_clipping: true,
    })
    .limiter(LimiterSettings {
        threshold_db: -1.0,
        lookahead: LookaheadPreset::Balanced,
    })
    .headroom(HeadroomSettings {
        mode: HeadroomMode::Auto,
    })
    .crossfade(CrossfadeSettings {
        enabled: true,
        duration_ms: 3000,
        curve: FadeCurve::EqualPower,
        on_skip: true,
    })
    .build();

manager.apply_audio_settings(settings);
```

## Removed Internal APIs

These methods are now `pub(crate)` (internal to soul-playback):
- `set_audio_source()` - Use platform layer
- `set_next_source()` - Use platform layer
- `record_track_load_failure()` - Internal
- `should_retry_after_circuit_open()` - Internal
- `reset_circuit_breaker()` - Internal
- `set_sample_rate()` - Configured via PlaybackConfig
- `set_output_channels()` - Configured via PlaybackConfig
- `process_audio()` - Use platform layer
- `set_lazy_context()` - Use platform layer
- `check_batch_loading()` - Internal
- `check_jump_loading()` - Internal
```

---

## Summary

### Metrics

| Metric | Before | After | Change |
|--------|--------|-------|--------|
| Public methods (manager.rs) | 111 | 28 | -75% |
| Public methods (components) | 186 | 35 | -81% |
| **Total public API** | **297** | **63** | **-79%** |
| Actually used by apps | 25 | 28 | +3 (added) |
| Internal (now pub(crate)) | 0 | 25 | Cleaned up |
| Deprecated duplicates | 0 | 15 | Will remove |
| Moved to AudioSettings | 0 | 35 | Consolidated |

### Benefits

1. **Simplicity**: 28 core methods (down from 111)
2. **Discoverability**: Clear grouping (control, volume, queue, modes, state)
3. **Maintainability**: Less surface area to test/document
4. **Flexibility**: AudioSettings builder allows future expansion without API bloat
5. **Type Safety**: Strongly-typed settings structs prevent invalid configurations
6. **Backward Compatibility**: Deprecation path, not breaking changes

### Timeline

- **Week 1**: Deprecate duplicates (15 methods)
- **Week 2**: Make internals pub(crate) (25 methods)
- **Week 3**: Introduce AudioSettings builder
- **Week 4**: Deprecate old audio settings (35 methods)
- **v1.0.0**: Remove deprecated APIs (final: 28 methods)

### Next Steps

1. Create `src/audio_settings.rs` module
2. Implement `AudioSettingsBuilder`
3. Add deprecation warnings
4. Update documentation
5. Update examples in `/applications`
6. Run full test suite
7. Update CLAUDE.md with new patterns

---

**Approval Required**: This is a major API change. Please review before implementation.
