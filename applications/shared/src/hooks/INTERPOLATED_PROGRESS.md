# Smooth Progress Interpolation

## Problem

The backend emits position updates every 500ms, causing the progress bar to visibly jump between updates. This creates a jarring user experience that doesn't feel smooth.

## Solution

The `useInterpolatedProgress` hook provides 60fps smooth interpolation between backend updates using `requestAnimationFrame`. It automatically handles:

- **Smooth animation**: Advances progress at the actual playback rate (1 second per second)
- **Pause detection**: Stops interpolating when playback is paused
- **Seek detection**: Resets on backward/forward seeks (jumps > 0.5%)
- **Track changes**: Resets to 0 when track changes
- **Bounds checking**: Never overshoots duration or drifts too far from backend

## Usage

```typescript
import { useInterpolatedProgress } from '@soul-player/shared';

function MyProgressBar() {
  // Get smoothly interpolated progress (60fps)
  const { progress, duration } = useInterpolatedProgress();

  // progress: 0-100 (percentage)
  // duration: seconds

  const currentTimeSeconds = duration > 0
    ? (progress / 100) * duration
    : 0;

  return (
    <div>
      <div style={{ width: `${progress}%` }} />
      <span>{formatDuration(currentTimeSeconds)} / {formatDuration(duration)}</span>
    </div>
  );
}
```

## How It Works

1. **Backend Updates**: Every 500ms, the backend emits `playback:position-updated` event
2. **Store Update**: `usePlaybackEvents` updates the store with new progress percentage
3. **Interpolation**: Hook detects the update and smoothly interpolates between old and new values
4. **Animation Loop**: Uses `requestAnimationFrame` to advance progress at ~60fps
5. **Sync Points**: Resets to backend value on pause, seek, or track change

## Implementation Details

### Seek Detection

Seeks are detected by comparing progress change between updates:

```typescript
const progressDiff = Math.abs(progress - lastBackendProgress.current);
const SEEK_THRESHOLD = 0.5; // 0.5% difference = seek

if (progressDiff > SEEK_THRESHOLD) {
  // Reset to new position
  setInterpolatedProgress(progress);
}
```

This threshold is:
- **Large enough** to avoid false positives from normal interpolation drift
- **Small enough** to catch actual seeks (even small ones)

### Drift Prevention

To prevent the interpolated value from drifting too far ahead:

```typescript
const maxProgress = Math.min(100, lastBackendProgress.current + 2);
return Math.min(newProgress, maxProgress);
```

This allows up to 2% drift ahead of the backend's last known position.

### Cleanup

Animation frames are properly cleaned up on:
- Component unmount
- Pause (no need to animate while paused)
- Track change (reset to 0)
- Seek (reset to new position)

## Testing

Comprehensive tests cover:
- ✅ Initial progress from store
- ✅ Smooth interpolation during playback
- ✅ Pause detection (stops interpolating)
- ✅ Track change (resets to 0)
- ✅ Backward seek detection
- ✅ Forward seek detection
- ✅ Duration bounds (never exceeds 100%)
- ✅ Drift prevention (max 2% ahead)
- ✅ Zero duration handling
- ✅ Memory leak prevention (cleanup on unmount)
- ✅ Backend sync (accepts normal updates)

Run tests:
```bash
yarn workspace @soul-player/shared test useInterpolatedProgress
```

## Performance

- **Frame Rate**: Targets 60fps via `requestAnimationFrame`
- **CPU Usage**: Minimal - only runs when playing
- **Memory**: No leaks - proper cleanup on unmount
- **Re-renders**: Uses Zustand selectors to minimize unnecessary re-renders

## Platform Compatibility

Works with both playback backends:
- ✅ **Tauri (Desktop)**: Rust backend with 500ms position events
- ✅ **Web**: WASM playback adapter (future)

The hook is platform-agnostic - it only depends on the player store, which is updated by the appropriate event bridge for each platform.
