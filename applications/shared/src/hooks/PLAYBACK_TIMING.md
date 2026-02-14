# Playback Timing Synchronization

This document explains how frontend and backend timing is synchronized to prevent race conditions and ensure smooth playback UX.

## Overview

The playback system has several timing-sensitive operations:

1. **Position updates** - Backend emits position every N milliseconds during playback
2. **Seek operations** - User jumps to a new position in the track
3. **Device events** - Platform APIs emit device add/remove/change events

These operations can race with each other, causing UI glitches like:
- Progress bar jumping back after seek
- Duplicate device switches
- Stale position updates overwriting fresh data

## Solution: Shared Timing Configuration

All timing values are defined in **one place** and synchronized between frontend and backend:

### Backend (Rust)
- **Source**: `applications/desktop/src-tauri/src/playback_constants.rs`
- **Command**: `get_playback_timing_config` - Returns current timing config
- **Default values**:
  - `position_update_interval_ms`: 500ms (2 updates/sec)
  - `ignore_window_ms`: 600ms (500ms × 1.2)
  - `device_event_dedup_window_ms`: 500ms

### Frontend (TypeScript)
- **Source**: `applications/shared/src/types/playback-timing.ts`
- **Hook**: `usePlaybackTiming()` - Fetches config from backend
- **Fallback**: `DEFAULT_TIMING_CONFIG` if fetch fails or on web platform

## Key Timing Values

### Position Update Interval (500ms default)

**What it controls**:
- How often backend emits `playback:position-updated` events during playback

**Why 500ms?**
- ✅ Smooth UI updates (2x per second)
- ✅ Low CPU usage (vs 100ms = 5x more events)
- ✅ Aligns with human perception (100ms changes barely noticeable for progress bars)

**Configurable range**: 50ms - 2000ms
- Faster (100ms): Better responsiveness, higher overhead
- Slower (1000ms): Lower overhead, jerky progress bar

**Where it's used**:
- Backend: `playback.rs` event emission loop
- Frontend: Not directly, but affects ignore window calculation

### Ignore Window (600ms default)

**What it controls**:
- How long frontend ignores position updates after seeking

**Calculation**: `position_update_interval × 1.2`
- The 1.2 multiplier ensures we ignore slightly longer than one update cycle
- Prevents race where backend emits stale position just as seek completes

**Example timeline**:
```
0ms:   User clicks seek to 30s
0ms:   Frontend immediately updates UI to 30s
0ms:   Frontend sets ignore flag = true
10ms:  Seek command sent to backend
50ms:  Backend still emitting old position (29s) - IGNORED
500ms: Backend emits new position (30s) - IGNORED
600ms: Ignore window expires, flag = false
1000ms: Backend emits position (30.5s) - ACCEPTED
```

**Where it's used**:
- Frontend: `useSeekBar.ts` - setTimeout duration after seek

### Device Event Deduplication Window (500ms)

**What it controls**:
- How long to ignore duplicate device events from platform APIs

**Why needed?**:
Platform APIs (CoreAudio/PipeWire/WinRT) can emit duplicates:
- Device removed twice
- Default device changed to same device multiple times
- Property changed fired repeatedly

**Where it's used**:
- Backend: `playback.rs` - `LastDeviceEvent.is_duplicate()`

## Usage Examples

### Frontend: Using the timing config

```typescript
import { usePlaybackTiming } from '@soul-player/shared/hooks';

function MyComponent() {
  const timing = usePlaybackTiming();

  // Use ignore window for seek operations
  setTimeout(() => {
    enablePositionUpdates();
  }, timing.ignoreWindowMs);
}
```

### Backend: Updating the interval

To change the position update interval:

1. Edit `playback_constants.rs`:
```rust
pub const DEFAULT_POSITION_UPDATE_INTERVAL_MS: u64 = 250; // Faster updates
```

2. Rebuild - frontend automatically picks up new value via `get_playback_timing_config`

### Future: User-configurable interval

To make interval user-configurable:

1. Add setting to `PlaybackConfig`:
```rust
pub struct PlaybackConfig {
    pub position_update_interval_ms: Option<u64>,
    // ... other fields
}
```

2. Update `get_playback_timing_config` command:
```rust
async fn get_playback_timing_config(
    playback: State<'_, LazyPlaybackManager>,
) -> Result<PlaybackTimingConfig, String> {
    let interval = playback.get_config().position_update_interval_ms
        .unwrap_or(DEFAULT_POSITION_UPDATE_INTERVAL_MS);
    Ok(PlaybackTimingConfig::with_position_interval(interval))
}
```

3. Frontend automatically adapts via `usePlaybackTiming()`

## Testing

### Verify synchronization

1. Change backend interval in `playback_constants.rs`
2. Run app and seek to position
3. Check logs: `[useSeekBar] Seeking to position: X ignore window: Y`
4. Y should be interval × 1.2

### Test edge cases

```rust
#[test]
fn test_ignore_window_calculation() {
    let config = PlaybackTimingConfig::with_position_interval(500);
    assert_eq!(config.ignore_window_ms, 600); // 500 * 1.2
}

#[test]
fn test_interval_clamping() {
    let config = PlaybackTimingConfig::with_position_interval(10); // Too low
    assert_eq!(config.position_update_interval_ms, MIN_POSITION_UPDATE_INTERVAL_MS);
}
```

## Troubleshooting

### Progress bar jumps back after seek

**Symptom**: Click seek to 30s, bar jumps to 30s, then back to 29s, then forward to 30s

**Cause**: Ignore window too short - backend emitting stale position before window expires

**Fix**: Increase `IGNORE_WINDOW_MULTIPLIER` in `playback_constants.rs`:
```rust
pub const IGNORE_WINDOW_MULTIPLIER: f64 = 1.5; // Was 1.2
```

### Duplicate device switches

**Symptom**: Single device unplug triggers two switches

**Cause**: Deduplication window too short for platform API

**Fix**: Increase `DEVICE_EVENT_DEDUP_WINDOW_MS`:
```rust
pub const DEVICE_EVENT_DEDUP_WINDOW_MS: u64 = 1000; // Was 500
```

### Jerky progress bar

**Symptom**: Progress bar advances in large jumps

**Cause**: Position update interval too long

**Fix**: Decrease `DEFAULT_POSITION_UPDATE_INTERVAL_MS`:
```rust
pub const DEFAULT_POSITION_UPDATE_INTERVAL_MS: u64 = 250; // Was 500
```

**Note**: Also update `IGNORE_WINDOW_MULTIPLIER` proportionally to maintain buffer.

## References

- Backend constants: `applications/desktop/src-tauri/src/playback_constants.rs`
- Frontend types: `applications/shared/src/types/playback-timing.ts`
- Frontend hook: `applications/shared/src/hooks/usePlaybackTiming.ts`
- Seek handling: `applications/shared/src/hooks/useSeekBar.ts`
- Event emission: `applications/desktop/src-tauri/src/playback.rs`
