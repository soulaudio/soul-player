# Playback State Persistence Design

**Date:** 2026-02-14
**Status:** Approved
**Author:** Claude (Brainstorming Session)

---

## Problem Statement

When the Soul Player app refreshes or hot-reloads, playback state is lost from the frontend (Zustand store). This causes:
- Current track shows as `null` in the sidebar
- User cannot pause/control playback
- Queue, volume, and position are not visible in UI

Additionally, when the app fully restarts (cold start), all playback state is lost entirely.

---

## Requirements

1. **Auto-resume behavior**: Smart restore
   - Hot reload (dev): Auto-resume if was playing
   - Cold start (production): Restore state but remain paused

2. **State to persist**:
   - Current track + queue + queue position
   - Playback position (seek time in current track)
   - Volume level
   - Repeat & shuffle modes
   - Playback context (album/artist/playlist)

3. **Save timing**:
   - **Immediate**: Track changes, queue updates, pause/stop, volume changes (>5% threshold), mode changes
   - **Debounced (5s)**: Position updates (to avoid excessive DB writes)

4. **Edge case handling**:
   - Skip missing tracks and continue playback
   - If all tracks missing, clear persisted state
   - If current track missing, advance to next valid track

---

## Architecture Overview

### Two-Scenario Approach (Dual Sync)

**Scenario 1: Hot Reload (Dev)**
- Backend (Rust PlaybackManager) is still running
- Frontend (React) rebuilds
- **Solution**: Query backend state → Populate Zustand store

**Scenario 2: Cold Start (Production)**
- Both frontend + backend restart
- **Solution**: Load from database → Hydrate backend + frontend

### Detection Logic

```typescript
const backendTrack = await invoke('get_current_track');

if (backendTrack) {
  // Hot reload - backend is alive
  await syncFromBackend();
} else {
  // Cold start - restore from database
  await restoreFromDatabase();
}
```

---

## Database Schema

Uses existing `user_settings` table with these keys:

| Key | Type | Description |
|-----|------|-------------|
| `playback.current_track_id` | number | Current track ID |
| `playback.queue_track_ids` | JSON array | Queue as array of track IDs |
| `playback.queue_index` | number | Current position in queue |
| `playback.position_seconds` | number | Seek position in current track |
| `playback.volume` | number | Volume 0.0-1.0 |
| `playback.repeat_mode` | string | "off" \| "all" \| "one" |
| `playback.shuffle_mode` | string | "off" \| "random" \| "smart" |
| `playback.context_type` | string \| null | "album" \| "artist" \| "playlist" |
| `playback.context_id` | string \| null | Context entity ID |
| `playback.was_playing` | boolean | For smart resume behavior |

---

## Components & Modules

### Backend (Rust)

**New Tauri Commands** (`applications/desktop/src-tauri/src/main.rs`):
- `get_current_track()` → Current track or null
- `get_queue()` → Full queue as Vec<Track>
- `get_queue_index()` → Current queue position
- `get_position()` → Playback position in seconds
- `get_volume()` → Volume 0.0-1.0
- `save_playback_session(state: PlaybackSessionState)` → Writes to database
- `restore_playback_session()` → Loads from database and hydrates backend

**Files Modified:**
- `applications/desktop/src-tauri/src/main.rs` - Command handlers
- `applications/desktop/src-tauri/src/playback.rs` - Wrapper functions
- `libraries/soul-storage/src/settings/mod.rs` - No changes (uses existing functions)

### Frontend (TypeScript)

**Files Modified:**
- `applications/desktop/src/providers/TauriPlayerCommandsProvider.tsx`
  - Extend `syncInitialState()` for dual sync
  - Add `savePlaybackSession()` helper
  - Add debounced position saver (5s delay)
  - Hook into event listeners to trigger saves

- `applications/shared/src/stores/player.ts`
  - No structural changes (already has all state)

**No changes to:**
- UI components (use Zustand store)
- `PlayerCommandsContext` interface
- Event listeners (already in place)

---

## Data Flow

### Save Flow (State → Database)

**Trigger Points:**
1. **Immediate save**:
   - `playback:track-changed` event
   - `playback:queue-updated` event
   - Pause/stop events
   - Volume changes (>5% threshold)
   - Repeat/shuffle mode changes

2. **Debounced save (5s)**:
   - `playback:position-updated` event (fires every 250ms)

**Implementation:**
```typescript
// Immediate save
useEffect(() => {
  const unsubscribe = usePlayerStore.subscribe(
    (state) => state.currentTrack,
    (currentTrack) => {
      savePlaybackSession(); // Immediate
    }
  );
  return unsubscribe;
}, []);

// Debounced save
const debouncedSave = useMemo(
  () => debounce(savePlaybackSession, 5000),
  []
);

useEffect(() => {
  const unsubscribe = usePlayerStore.subscribe(
    (state) => state.progress,
    () => {
      debouncedSave(); // Debounced 5s
    }
  );
  return unsubscribe;
}, []);
```

### Restore Flow (Database → State)

**Cold Start:**
```typescript
async function restoreFromDatabase() {
  // 1. Load persisted session
  const session = await invoke('restore_playback_session');
  if (!session) return;

  // 2. Fetch full track objects by IDs
  const tracks = await backend.getTracksByIds(session.queueTrackIds);

  // 3. Filter out missing tracks
  const validTracks = tracks.filter(t => t !== null);
  if (validTracks.length === 0) {
    await invoke('clear_playback_session');
    return;
  }

  // 4. Adjust queue index if current track missing
  let queueIndex = session.queueIndex;
  if (!validTracks[queueIndex]) {
    queueIndex = 0; // Start from first valid track
  }

  // 5. Hydrate backend
  await invoke('load_queue', {
    tracks: validTracks,
    index: queueIndex,
    seekPosition: session.positionSeconds
  });

  // 6. Set volume and modes
  await invoke('set_volume', { volume: session.volume });
  await invoke('set_repeat_mode', { mode: session.repeatMode });
  await invoke('set_shuffle_mode', { mode: session.shuffleMode });

  // 7. Update Zustand store
  usePlayerStore.setState({
    queue: validTracks,
    queueIndex,
    currentTrack: validTracks[queueIndex],
    volume: session.volume,
    isPlaying: false, // Always paused on cold start
    repeatMode: session.repeatMode,
    shuffleMode: session.shuffleMode
  });

  // 8. Restore playback context
  if (session.contextType && session.contextId) {
    await backend.recordContext({
      contextType: session.contextType,
      contextId: session.contextId
    });
  }
}
```

**Hot Reload:**
```typescript
async function syncFromBackend() {
  const [track, queue, queueIndex, position, volume, repeat, shuffle] =
    await Promise.all([
      invoke('get_current_track'),
      invoke('get_queue'),
      invoke('get_queue_index'),
      invoke('get_position'),
      invoke('get_volume'),
      invoke('get_repeat_mode'),
      invoke('get_shuffle_mode')
    ]);

  usePlayerStore.setState({
    currentTrack: track,
    queue,
    queueIndex,
    volume,
    progress: position ? (position / track.durationSeconds) * 100 : 0,
    duration: track?.durationSeconds ?? 0,
    repeatMode: repeat,
    shuffleMode: shuffle
  });
}
```

---

## Error Handling

### Database Write Failures
- Log error: `tracing::error!("[PERSISTENCE] Failed to save: {}")`
- Don't crash - playback continues
- Show toast: "Failed to save playback state"
- Retry once after 1 second delay

### Corrupted/Invalid State
- Validate restored state (null checks, bounds checks)
- If invalid → clear state and start fresh
- Log warning: `tracing::warn!("[PERSISTENCE] Corrupted state detected")`

### Missing Tracks
- Filter out null tracks from queue
- If all missing → clear state
- If current track missing → skip to next valid track
- Optional toast: "Some tracks were unavailable and skipped"

### Backend Sync Failures
- If commands timeout → fall back to database restore
- Log errors for debugging

---

## Testing Strategy

### Unit Tests

**Rust** (`libraries/soul-storage/src/settings/mod.rs`):
- ✅ Save/load each setting key
- ✅ Batch operations
- ✅ Edge cases (null values, invalid JSON)

**TypeScript** (`TauriPlayerCommandsProvider.test.tsx`):
- ✅ Mock Tauri invoke calls
- ✅ Hot reload detection logic
- ✅ Save debouncing
- ✅ Restore with missing tracks

### Integration Tests

**Rust** (`libraries/soul-audio-desktop/tests/playback_persistence_test.rs`):
- ✅ Full save → restore cycle
- ✅ Queue restoration with missing tracks
- ✅ Volume/repeat/shuffle persistence

### E2E Tests

**New test** (`playback_state_persistence_e2e_test.rs`):

| Test Case | Scenario | Expected |
|-----------|----------|----------|
| Happy path | Play track → save → restart → verify | State restored correctly |
| Missing track | Persist queue → delete track → restart | Track skipped, playback continues |
| Hot reload | Play → kill frontend → restart | Frontend syncs with backend |
| Position restore | Seek to 30s → restart | Resumes at 30s (paused) |
| Empty state | No persisted state | App starts normally |
| Corrupted state | Invalid JSON in database | Clears and starts fresh |

**Test Harness:**
- Use testcontainers for real SQLite
- Use actual PlaybackManager instance
- Simulate app restart by dropping/recreating providers

---

## Implementation Phases

### Phase 1: Backend Commands
- Add Tauri commands to query PlaybackManager state
- Add `save_playback_session()` and `restore_playback_session()`
- Add `clear_playback_session()` helper

### Phase 2: Frontend Sync
- Extend `TauriPlayerCommandsProvider.syncInitialState()`
- Implement `syncFromBackend()` for hot reload
- Implement `restoreFromDatabase()` for cold start

### Phase 3: Auto-Save
- Add immediate save triggers (track change, queue update)
- Add debounced save for position updates
- Hook into event listeners

### Phase 4: Edge Cases
- Handle missing tracks gracefully
- Validate restored state
- Add retry logic for DB failures

### Phase 5: Testing
- Write unit tests
- Write integration tests
- Write E2E tests

---

## Success Criteria

1. ✅ Hot reload preserves playback state (sidebar shows current track)
2. ✅ Cold start restores state but remains paused
3. ✅ Missing tracks are skipped gracefully
4. ✅ Position is restored within 5 seconds accuracy
5. ✅ Volume, repeat, shuffle modes are preserved
6. ✅ Playback context (album/artist/playlist) is restored
7. ✅ All tests pass (unit, integration, E2E)

---

## Future Enhancements

- Add state versioning for schema migrations
- Add telemetry to track persistence success rate
- Add user setting to disable persistence
- Add "Restore last session" prompt on startup
- Add maximum session age (clear sessions older than 7 days)

---

## References

- Investigation Report: [Playback State Management Investigation](#)
- Existing Settings System: `libraries/soul-storage/src/settings/mod.rs`
- Zustand Store: `applications/shared/src/stores/player.ts`
- Tauri Provider: `applications/desktop/src/providers/TauriPlayerCommandsProvider.tsx`
