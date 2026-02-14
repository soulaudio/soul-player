# Seek Performance Debugging - Why Still Slow?

## Issue
After simplification, seek still takes too long. Need to investigate actual bottleneck.

## What We Changed
1. ✅ Simplified useSeekBar: 151 → 44 lines
2. ✅ Removed timing config fetching
3. ✅ Hardcoded ignore window: 120ms
4. ✅ Hardcoded position update interval: 100ms

## What We DIDN'T Change (Potential Bottlenecks)
1. ❓ Backend Symphonia seek implementation (still slow for MP3 VBR)
2. ❓ Position update interval is 100ms but might not be applying
3. ❓ Ignore window might be TOO LONG (120ms feels slow)
4. ❓ No visual feedback - user can't tell if seek is happening

## Test in Running App

Open DevTools console and run:
```javascript
// Measure seek latency
let seekStart;
const originalSeek = window.__TAURI__?.core?.invoke;
if (originalSeek) {
  window.__TAURI__.core.invoke = function(cmd, args) {
    if (cmd === 'seek_to') {
      seekStart = performance.now();
      console.log('[PERF] Seek command sent at:', seekStart);
    }
    return originalSeek.call(this, cmd, args);
  };
}

// Watch for position updates
let lastUpdate = 0;
const store = window.__soul_player_store__ || usePlayerStore?.getState();
if (store?.subscribe) {
  store.subscribe((state) => {
    if (state.progress !== lastUpdate) {
      if (seekStart) {
        console.log('[PERF] Seek completed in:', performance.now() - seekStart, 'ms');
        seekStart = null;
      }
      lastUpdate = state.progress;
    }
  });
}
```

## Expected vs Actual Latency

**Expected (if working correctly)**:
- Click → Store update: <5ms (optimistic)
- Backend seek: 10-50ms (MP3 VBR worst case)
- Ignore window: 120ms (hardcoded)
- **Total perceived**: ~130-175ms

**If Still Slow** (>300ms):
Possible causes:
1. Position update interval NOT actually 100ms (still 500ms?)
2. Ignore window calculation wrong
3. Backend seek taking >200ms (disk I/O issue)
4. React re-renders blocking UI thread

## Quick Fixes to Try

### Fix 1: Reduce Ignore Window to 50ms
```typescript
// TauriPlayerCommandsProvider.tsx
const IGNORE_WINDOW_MS = 50;  // Was 120ms
```

### Fix 2: Verify Position Updates Are 100ms
Check backend logs - should see position updates every 100ms, not 500ms

### Fix 3: Add Performance Logging
```typescript
// useSeekBar.ts
const handleSeek = useCallback((position: number) => {
  const t0 = performance.now();
  console.log('[SEEK PERF] Start:', t0);

  const { duration } = usePlayerStore.getState();
  const clampedPosition = Math.max(0, Math.min(position, duration - 0.1));

  const progressPercentage = duration > 0
    ? (clampedPosition / duration) * 100
    : 0;

  const t1 = performance.now();
  usePlayerStore.setState({ progress: progressPercentage });
  console.log('[SEEK PERF] Store updated:', t1 - t0, 'ms');

  commands.seek(clampedPosition)
    .then(() => {
      console.log('[SEEK PERF] Backend completed:', performance.now() - t0, 'ms');
    })
    .catch((error) => {
      debug.error('[useSeekBar] Seek failed:', error);
    });
}, [commands]);
```

### Fix 4: Remove Interpolation During Seek
The interpolation might be conflicting with seek. Try disabling during seek window.

## Investigation Steps

1. **Verify Constants Applied**:
   - Check console for `IGNORE_WINDOW_MS = 120` (should be in logs)
   - Check backend position updates (should be 100ms apart)

2. **Measure Actual Times**:
   - Add performance.now() timestamps
   - Log: click → store → backend → completion

3. **Test Different File Formats**:
   - FLAC should seek instantly (1-5ms)
   - MP3 VBR will be slower (20-50ms)
   - If FLAC is also slow, it's not the decoder

4. **Check for Race Conditions**:
   - Multiple seeks queuing up?
   - Store updates conflicting with interpolation?

## Nuclear Option: Zero Ignore Window

If latency is critical, try removing ignore window entirely:
```typescript
async seek(position: number) {
  // No ignore window - just send and update immediately
  usePlayerStore.setState({ progress: (position / duration) * 100 });
  await invoke('seek_to', { position });
}
```

Trade-off: Progress bar might jump back briefly, but seek feels instant.
