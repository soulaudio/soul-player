# Playback State Persistence Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Enable playback state persistence across app refreshes and restarts so users never lose their queue, position, or playback context.

**Architecture:** Dual sync approach - hot reload queries backend state (fast), cold start restores from SQLite database (complete). Auto-save on track changes (immediate) and position updates (debounced 5s).

**Tech Stack:** Rust (Tauri commands + PlaybackManager), TypeScript/React (TauriPlayerCommandsProvider), SQLite (user_settings table), Zustand (UI state)

---

## Phase 1: Backend State Query Commands

### Task 1: Add get_current_track command

**Files:**
- Modify: `applications/desktop/src-tauri/src/playback.rs`
- Modify: `applications/desktop/src-tauri/src/main.rs`
- Test: Manual verification with `cargo test`

**Step 1: Add wrapper function in playback.rs**

Add after the existing command handlers (~line 800):

```rust
/// Get current track information
pub async fn get_current_track_info(playback: &PlaybackManager) -> Option<soul_playback::QueueTrack> {
    playback.get_current_track()
}

/// Get full queue
pub async fn get_queue_tracks(playback: &PlaybackManager) -> Vec<soul_playback::QueueTrack> {
    playback.get_queue()
}

/// Get current queue index
pub async fn get_queue_index_value(playback: &PlaybackManager) -> i32 {
    playback.get_queue_index()
}

/// Get current playback position in seconds
pub async fn get_playback_position(playback: &PlaybackManager) -> f64 {
    playback.get_position()
}

/// Get current volume (0.0 to 1.0)
pub async fn get_volume_level(playback: &PlaybackManager) -> f64 {
    playback.get_volume()
}
```

**Step 2: Add Tauri commands in main.rs**

Add in the command list (~line 1600, after existing playback commands):

```rust
#[tauri::command]
async fn get_current_track(playback: State<'_, LazyPlaybackManager>) -> Result<Option<soul_playback::QueueTrack>, String> {
    let playback = playback.get().await?;
    Ok(crate::playback::get_current_track_info(&playback).await)
}

#[tauri::command]
async fn get_queue(playback: State<'_, LazyPlaybackManager>) -> Result<Vec<soul_playback::QueueTrack>, String> {
    let playback = playback.get().await?;
    Ok(crate::playback::get_queue_tracks(&playback).await)
}

#[tauri::command]
async fn get_queue_index(playback: State<'_, LazyPlaybackManager>) -> Result<i32, String> {
    let playback = playback.get().await?;
    Ok(crate::playback::get_queue_index_value(&playback).await)
}

#[tauri::command]
async fn get_position(playback: State<'_, LazyPlaybackManager>) -> Result<f64, String> {
    let playback = playback.get().await?;
    Ok(crate::playback::get_playback_position(&playback).await)
}

#[tauri::command]
async fn get_volume(playback: State<'_, LazyPlaybackManager>) -> Result<f64, String> {
    let playback = playback.get().await?;
    Ok(crate::playback::get_volume_level(&playback).await)
}
```

**Step 3: Register commands in Tauri builder**

In `main.rs`, add to the `.invoke_handler()` list (~line 1900):

```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands ...
    get_current_track,
    get_queue,
    get_queue_index,
    get_position,
    get_volume,
])
```

**Step 4: Verify compilation**

Run: `cargo check --manifest-path applications/desktop/src-tauri/Cargo.toml`
Expected: No errors

**Step 5: Commit**

```bash
git add applications/desktop/src-tauri/src/playback.rs applications/desktop/src-tauri/src/main.rs
git commit -m "feat(backend): add Tauri commands to query playback state

Add get_current_track, get_queue, get_queue_index, get_position, and
get_volume commands to support frontend state synchronization.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 2: Add save_playback_session command

**Files:**
- Modify: `applications/desktop/src-tauri/src/main.rs`
- Test: Manual verification with database query

**Step 1: Define PlaybackSessionState struct**

Add near top of `main.rs` after imports (~line 50):

```rust
#[derive(Debug, serde::Deserialize)]
struct PlaybackSessionState {
    current_track_id: Option<i64>,
    queue_track_ids: Vec<i64>,
    queue_index: i32,
    position_seconds: f64,
    volume: f64,
    repeat_mode: String,
    shuffle_mode: String,
    context_type: Option<String>,
    context_id: Option<String>,
    was_playing: bool,
}
```

**Step 2: Implement save_playback_session command**

Add after other Tauri commands (~line 1700):

```rust
#[tauri::command]
async fn save_playback_session(
    state: State<'_, AppState>,
    session: PlaybackSessionState,
) -> Result<(), String> {
    use soul_storage::settings;

    let pool = &state.db_pool;
    let user_id = state.current_user_id;

    tracing::debug!("[PERSISTENCE] Saving playback session for user {}", user_id);

    // Save all session keys to database
    if let Some(track_id) = session.current_track_id {
        settings::set_setting(pool, &user_id.to_string(), "playback.current_track_id", &serde_json::json!(track_id))
            .await
            .map_err(|e| format!("Failed to save current_track_id: {}", e))?;
    }

    settings::set_setting(pool, &user_id.to_string(), "playback.queue_track_ids", &serde_json::json!(session.queue_track_ids))
        .await
        .map_err(|e| format!("Failed to save queue_track_ids: {}", e))?;

    settings::set_setting(pool, &user_id.to_string(), "playback.queue_index", &serde_json::json!(session.queue_index))
        .await
        .map_err(|e| format!("Failed to save queue_index: {}", e))?;

    settings::set_setting(pool, &user_id.to_string(), "playback.position_seconds", &serde_json::json!(session.position_seconds))
        .await
        .map_err(|e| format!("Failed to save position_seconds: {}", e))?;

    settings::set_setting(pool, &user_id.to_string(), "playback.volume", &serde_json::json!(session.volume))
        .await
        .map_err(|e| format!("Failed to save volume: {}", e))?;

    settings::set_setting(pool, &user_id.to_string(), "playback.repeat_mode", &serde_json::json!(session.repeat_mode))
        .await
        .map_err(|e| format!("Failed to save repeat_mode: {}", e))?;

    settings::set_setting(pool, &user_id.to_string(), "playback.shuffle_mode", &serde_json::json!(session.shuffle_mode))
        .await
        .map_err(|e| format!("Failed to save shuffle_mode: {}", e))?;

    if let Some(context_type) = session.context_type {
        settings::set_setting(pool, &user_id.to_string(), "playback.context_type", &serde_json::json!(context_type))
            .await
            .map_err(|e| format!("Failed to save context_type: {}", e))?;
    }

    if let Some(context_id) = session.context_id {
        settings::set_setting(pool, &user_id.to_string(), "playback.context_id", &serde_json::json!(context_id))
            .await
            .map_err(|e| format!("Failed to save context_id: {}", e))?;
    }

    settings::set_setting(pool, &user_id.to_string(), "playback.was_playing", &serde_json::json!(session.was_playing))
        .await
        .map_err(|e| format!("Failed to save was_playing: {}", e))?;

    tracing::info!("[PERSISTENCE] Playback session saved successfully");
    Ok(())
}
```

**Step 3: Register command**

Add to `.invoke_handler()` list:

```rust
save_playback_session,
```

**Step 4: Verify compilation**

Run: `cargo check --manifest-path applications/desktop/src-tauri/Cargo.toml`
Expected: No errors

**Step 5: Commit**

```bash
git add applications/desktop/src-tauri/src/main.rs
git commit -m "feat(backend): add save_playback_session command

Persist playback state to SQLite user_settings table.
Saves track, queue, position, volume, modes, and context.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 3: Add restore_playback_session command

**Files:**
- Modify: `applications/desktop/src-tauri/src/main.rs`

**Step 1: Define return struct**

Add after `PlaybackSessionState`:

```rust
#[derive(Debug, serde::Serialize)]
struct RestoredPlaybackSession {
    current_track_id: Option<i64>,
    queue_track_ids: Vec<i64>,
    queue_index: i32,
    position_seconds: f64,
    volume: f64,
    repeat_mode: String,
    shuffle_mode: String,
    context_type: Option<String>,
    context_id: Option<String>,
    was_playing: bool,
}
```

**Step 2: Implement restore command**

Add after `save_playback_session`:

```rust
#[tauri::command]
async fn restore_playback_session(
    state: State<'_, AppState>,
) -> Result<Option<RestoredPlaybackSession>, String> {
    use soul_storage::settings;

    let pool = &state.db_pool;
    let user_id = state.current_user_id;

    tracing::debug!("[PERSISTENCE] Restoring playback session for user {}", user_id);

    // Try to load current_track_id - if missing, no session exists
    let current_track_id = settings::get_setting(pool, &user_id.to_string(), "playback.current_track_id")
        .await
        .map_err(|e| format!("Failed to load current_track_id: {}", e))?
        .and_then(|v| v.as_i64());

    // If no current track, assume no session
    if current_track_id.is_none() {
        tracing::debug!("[PERSISTENCE] No saved session found");
        return Ok(None);
    }

    // Load all other settings
    let queue_track_ids = settings::get_setting(pool, &user_id.to_string(), "playback.queue_track_ids")
        .await
        .map_err(|e| format!("Failed to load queue_track_ids: {}", e))?
        .and_then(|v| serde_json::from_value::<Vec<i64>>(v).ok())
        .unwrap_or_default();

    let queue_index = settings::get_setting(pool, &user_id.to_string(), "playback.queue_index")
        .await
        .map_err(|e| format!("Failed to load queue_index: {}", e))?
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;

    let position_seconds = settings::get_setting(pool, &user_id.to_string(), "playback.position_seconds")
        .await
        .map_err(|e| format!("Failed to load position_seconds: {}", e))?
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let volume = settings::get_setting(pool, &user_id.to_string(), "playback.volume")
        .await
        .map_err(|e| format!("Failed to load volume: {}", e))?
        .and_then(|v| v.as_f64())
        .unwrap_or(0.8);

    let repeat_mode = settings::get_setting(pool, &user_id.to_string(), "playback.repeat_mode")
        .await
        .map_err(|e| format!("Failed to load repeat_mode: {}", e))?
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "off".to_string());

    let shuffle_mode = settings::get_setting(pool, &user_id.to_string(), "playback.shuffle_mode")
        .await
        .map_err(|e| format!("Failed to load shuffle_mode: {}", e))?
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "off".to_string());

    let context_type = settings::get_setting(pool, &user_id.to_string(), "playback.context_type")
        .await
        .map_err(|e| format!("Failed to load context_type: {}", e))?
        .and_then(|v| v.as_str().map(|s| s.to_string()));

    let context_id = settings::get_setting(pool, &user_id.to_string(), "playback.context_id")
        .await
        .map_err(|e| format!("Failed to load context_id: {}", e))?
        .and_then(|v| v.as_str().map(|s| s.to_string()));

    let was_playing = settings::get_setting(pool, &user_id.to_string(), "playback.was_playing")
        .await
        .map_err(|e| format!("Failed to load was_playing: {}", e))?
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    tracing::info!("[PERSISTENCE] Playback session restored successfully");

    Ok(Some(RestoredPlaybackSession {
        current_track_id,
        queue_track_ids,
        queue_index,
        position_seconds,
        volume,
        repeat_mode,
        shuffle_mode,
        context_type,
        context_id,
        was_playing,
    }))
}
```

**Step 3: Add clear_playback_session helper**

```rust
#[tauri::command]
async fn clear_playback_session(state: State<'_, AppState>) -> Result<(), String> {
    use soul_storage::settings;

    let pool = &state.db_pool;
    let user_id = state.current_user_id;

    tracing::info!("[PERSISTENCE] Clearing playback session for user {}", user_id);

    let keys = vec![
        "playback.current_track_id",
        "playback.queue_track_ids",
        "playback.queue_index",
        "playback.position_seconds",
        "playback.volume",
        "playback.repeat_mode",
        "playback.shuffle_mode",
        "playback.context_type",
        "playback.context_id",
        "playback.was_playing",
    ];

    for key in keys {
        let _ = settings::delete_setting(pool, &user_id.to_string(), key).await;
    }

    Ok(())
}
```

**Step 4: Register commands**

Add to `.invoke_handler()`:

```rust
restore_playback_session,
clear_playback_session,
```

**Step 5: Verify compilation**

Run: `cargo check --manifest-path applications/desktop/src-tauri/Cargo.toml`
Expected: No errors

**Step 6: Commit**

```bash
git add applications/desktop/src-tauri/src/main.rs
git commit -m "feat(backend): add restore and clear playback session commands

Load persisted state from database and provide cleanup helper.
Returns None if no session exists.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Phase 2: Frontend State Synchronization

### Task 4: Implement syncFromBackend (hot reload)

**Files:**
- Modify: `applications/desktop/src/providers/TauriPlayerCommandsProvider.tsx`

**Step 1: Add syncFromBackend function**

Inside `TauriPlayerCommandsProvider`, after `syncInitialState` function (~line 100):

```typescript
// Sync state from backend (hot reload scenario - backend is still running)
const syncFromBackend = async () => {
  tracing::debug!("[PERSISTENCE] Hot reload detected - syncing from backend");

  try {
    const [track, queue, queueIndex, position, volume, repeat, shuffle] = await Promise.all([
      invoke<QueueTrack | null>('get_current_track'),
      invoke<QueueTrack[]>('get_queue'),
      invoke<number>('get_queue_index'),
      invoke<number>('get_position'),
      invoke<number>('get_volume'),
      invoke<string>('get_repeat_mode'),
      invoke<string>('get_shuffle_mode'),
    ]);

    if (!isMounted) return;

    // Update store with backend state
    usePlayerStore.setState({
      currentTrack: track,
      queue,
      queueIndex,
      volume,
      progress: track && position ? (position / track.durationSeconds) * 100 : 0,
      duration: track?.durationSeconds ?? 0,
      repeatMode: repeat as 'off' | 'all' | 'one',
      shuffleMode: shuffle as 'off' | 'random' | 'smart',
    });

    console.log('[PERSISTENCE] State synced from backend:', {
      hasTrack: !!track,
      queueLength: queue.length,
      volume,
    });
  } catch (error) {
    console.error('[PERSISTENCE] Failed to sync from backend:', error);
    // Fall back to database restore
    await restoreFromDatabase();
  }
};
```

**Step 2: Modify syncInitialState to use syncFromBackend**

Replace the current `syncInitialState` implementation (~line 39):

```typescript
const syncInitialState = async () => {
  await new Promise(resolve => setTimeout(resolve, 0));

  try {
    // Check if backend has active state (hot reload scenario)
    const backendTrack = await invoke<QueueTrack | null>('get_current_track');

    if (backendTrack) {
      // Hot reload - backend is alive
      await syncFromBackend();
    } else {
      // Cold start - restore from database
      await restoreFromDatabase();
    }
  } catch (error) {
    console.error('[PERSISTENCE] Failed to sync initial state:', error);
  }
};
```

**Step 3: Add type imports**

At the top of file, ensure QueueTrack is imported:

```typescript
import type { QueueTrack } from '@soul-player/shared';
```

**Step 4: Verify compilation**

Run: `yarn typecheck`
Expected: No type errors

**Step 5: Commit**

```bash
git add applications/desktop/src/providers/TauriPlayerCommandsProvider.tsx
git commit -m "feat(frontend): implement hot reload state sync

Query backend state and populate Zustand store on hot reload.
Detects hot reload by checking if backend has active track.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 5: Implement restoreFromDatabase (cold start)

**Files:**
- Modify: `applications/desktop/src/providers/TauriPlayerCommandsProvider.tsx`
- Modify: `applications/shared/src/contexts/BackendContext.tsx` (add getTracksByIds method)

**Step 1: Add getTracksByIds to BackendInterface**

In `applications/shared/src/contexts/BackendContext.tsx` (~line 50):

```typescript
export interface BackendInterface {
  // ... existing methods ...

  /**
   * Get multiple tracks by their IDs
   * Returns null for missing tracks
   */
  getTracksByIds(trackIds: number[]): Promise<(Track | null)[]>;
}
```

**Step 2: Implement in TauriBackendProvider**

In `applications/desktop/src/providers/TauriBackendProvider.tsx` (~line 200):

```typescript
async getTracksByIds(trackIds: number[]): Promise<(Track | null)[]> {
  return await invoke('get_tracks_by_ids', { trackIds });
}
```

**Step 3: Add backend command**

In `applications/desktop/src-tauri/src/main.rs`:

```rust
#[tauri::command]
async fn get_tracks_by_ids(
    state: State<'_, AppState>,
    track_ids: Vec<i64>,
) -> Result<Vec<Option<soul_storage::Track>>, String> {
    let pool = &state.db_pool;
    let user_id = state.current_user_id;

    let mut results = Vec::new();
    for track_id in track_ids {
        match soul_storage::tracks::get_track_by_id(pool, &user_id.to_string(), track_id).await {
            Ok(track) => results.push(Some(track)),
            Err(_) => results.push(None),
        }
    }

    Ok(results)
}
```

Register in `.invoke_handler()`: `get_tracks_by_ids,`

**Step 4: Implement restoreFromDatabase function**

In `TauriPlayerCommandsProvider.tsx`, add after `syncFromBackend`:

```typescript
const restoreFromDatabase = async () => {
  console.log('[PERSISTENCE] Cold start detected - restoring from database');

  try {
    // Load persisted session
    const session = await invoke<{
      current_track_id: number | null;
      queue_track_ids: number[];
      queue_index: number;
      position_seconds: number;
      volume: number;
      repeat_mode: string;
      shuffle_mode: string;
      context_type: string | null;
      context_id: string | null;
      was_playing: boolean;
    } | null>('restore_playback_session');

    if (!session || !session.current_track_id) {
      console.log('[PERSISTENCE] No saved session found');
      return;
    }

    // Fetch full track objects by IDs
    const backend = useBackend();
    const tracks = await backend.getTracksByIds(session.queue_track_ids);

    // Filter out missing tracks
    const validTracks = tracks.filter((t): t is Track => t !== null);

    if (validTracks.length === 0) {
      console.warn('[PERSISTENCE] All tracks missing - clearing session');
      await invoke('clear_playback_session');
      return;
    }

    // Adjust queue index if current track is missing
    let queueIndex = session.queue_index;
    if (!validTracks[queueIndex]) {
      queueIndex = 0;
      console.warn('[PERSISTENCE] Current track missing - starting from first valid track');
    }

    if (!isMounted) return;

    // Update Zustand store
    usePlayerStore.setState({
      queue: validTracks,
      queueIndex,
      currentTrack: validTracks[queueIndex],
      volume: session.volume,
      isPlaying: false, // Always paused on cold start
      repeatMode: session.repeat_mode as 'off' | 'all' | 'one',
      shuffleMode: session.shuffle_mode as 'off' | 'random' | 'smart',
      progress: 0,
      duration: validTracks[queueIndex]?.durationSeconds ?? 0,
    });

    // Set backend state
    await invoke('set_volume', { volume: session.volume });
    await invoke('set_repeat_mode', { mode: session.repeat_mode });
    await invoke('set_shuffle_mode', { mode: session.shuffle_mode });

    // Restore playback context
    if (session.context_type && session.context_id) {
      await backend.recordContext({
        contextType: session.context_type,
        contextId: session.context_id,
      });
    }

    console.log('[PERSISTENCE] State restored from database:', {
      queueLength: validTracks.length,
      currentTrack: validTracks[queueIndex]?.title,
      volume: session.volume,
    });
  } catch (error) {
    console.error('[PERSISTENCE] Failed to restore from database:', error);
  }
};
```

**Step 5: Import useBackend hook**

```typescript
import { useBackend } from '../hooks/useBackend';
```

**Step 6: Verify compilation**

Run: `yarn typecheck`
Expected: No errors

**Step 7: Commit**

```bash
git add applications/desktop/src/providers/TauriPlayerCommandsProvider.tsx applications/shared/src/contexts/BackendContext.tsx applications/desktop/src/providers/TauriBackendProvider.tsx applications/desktop/src-tauri/src/main.rs
git commit -m "feat(frontend): implement cold start database restoration

Load persisted session from database, fetch tracks, handle missing
tracks gracefully, and restore playback state (paused).

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Phase 3: Auto-Save Implementation

### Task 6: Add immediate save triggers

**Files:**
- Modify: `applications/desktop/src/providers/TauriPlayerCommandsProvider.tsx`

**Step 1: Create savePlaybackSession helper**

Add inside `TauriPlayerCommandsProvider`, after `restoreFromDatabase`:

```typescript
const savePlaybackSession = async () => {
  try {
    const state = usePlayerStore.getState();
    const sessionContext = usePlaybackSession.getState();

    // Don't save if no track loaded
    if (!state.currentTrack) {
      return;
    }

    await invoke('save_playback_session', {
      session: {
        current_track_id: state.currentTrack.id,
        queue_track_ids: state.queue.map(t => t.id),
        queue_index: state.queueIndex,
        position_seconds: state.duration ? (state.progress / 100) * state.duration : 0,
        volume: state.volume,
        repeat_mode: state.repeatMode,
        shuffle_mode: state.shuffleMode,
        context_type: sessionContext.contextType,
        context_id: sessionContext.contextId,
        was_playing: state.isPlaying,
      },
    });

    console.log('[PERSISTENCE] Session saved');
  } catch (error) {
    console.error('[PERSISTENCE] Failed to save session:', error);
  }
};
```

**Step 2: Add immediate save effect for track changes**

Add in `useEffect` block (~line 150):

```typescript
// Subscribe to track changes - save immediately
useEffect(() => {
  const unsubscribe = usePlayerStore.subscribe(
    (state) => state.currentTrack?.id,
    () => {
      if (isMounted) {
        savePlaybackSession();
      }
    }
  );

  unlistenFunctions.push(unsubscribe);
  return unsubscribe;
}, []);
```

**Step 3: Add immediate save for queue changes**

```typescript
// Subscribe to queue changes - save immediately
useEffect(() => {
  const unsubscribe = usePlayerStore.subscribe(
    (state) => state.queue.length,
    () => {
      if (isMounted) {
        savePlaybackSession();
      }
    }
  );

  unlistenFunctions.push(unsubscribe);
  return unsubscribe;
}, []);
```

**Step 4: Add immediate save for volume changes (>5% threshold)**

```typescript
// Subscribe to volume changes - save if changed by >5%
let lastSavedVolume = usePlayerStore.getState().volume;

useEffect(() => {
  const unsubscribe = usePlayerStore.subscribe(
    (state) => state.volume,
    (volume) => {
      if (isMounted && Math.abs(volume - lastSavedVolume) > 0.05) {
        lastSavedVolume = volume;
        savePlaybackSession();
      }
    }
  );

  unlistenFunctions.push(unsubscribe);
  return unsubscribe;
}, []);
```

**Step 5: Add immediate save for mode changes**

```typescript
// Subscribe to repeat/shuffle mode changes - save immediately
useEffect(() => {
  const unsubscribe = usePlayerStore.subscribe(
    (state) => `${state.repeatMode}-${state.shuffleMode}`,
    () => {
      if (isMounted) {
        savePlaybackSession();
      }
    }
  );

  unlistenFunctions.push(unsubscribe);
  return unsubscribe;
}, []);
```

**Step 6: Verify compilation**

Run: `yarn typecheck`
Expected: No errors

**Step 7: Commit**

```bash
git add applications/desktop/src/providers/TauriPlayerCommandsProvider.tsx
git commit -m "feat(frontend): add immediate save triggers

Save playback session on track changes, queue updates, volume
changes (>5%), and mode changes using Zustand subscriptions.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 7: Add debounced position save

**Files:**
- Modify: `applications/desktop/src/providers/TauriPlayerCommandsProvider.tsx`

**Step 1: Install lodash.debounce if needed**

Check `package.json` - if debounce is not available, install:

```bash
yarn workspace @soul-player/desktop add lodash.debounce
yarn workspace @soul-player/desktop add -D @types/lodash.debounce
```

**Step 2: Import debounce**

At top of file:

```typescript
import debounce from 'lodash.debounce';
```

**Step 3: Create debounced save function**

Inside component, before the effects:

```typescript
const debouncedSave = useMemo(
  () => debounce(savePlaybackSession, 5000),
  []
);

// Cleanup debounced function on unmount
useEffect(() => {
  return () => {
    debouncedSave.cancel();
  };
}, [debouncedSave]);
```

**Step 4: Add debounced save for progress updates**

```typescript
// Subscribe to progress changes - save debounced (5s)
useEffect(() => {
  const unsubscribe = usePlayerStore.subscribe(
    (state) => state.progress,
    () => {
      if (isMounted) {
        debouncedSave();
      }
    }
  );

  unlistenFunctions.push(unsubscribe);
  return unsubscribe;
}, [debouncedSave]);
```

**Step 5: Verify compilation**

Run: `yarn typecheck`
Expected: No errors

**Step 6: Commit**

```bash
git add applications/desktop/src/providers/TauriPlayerCommandsProvider.tsx package.json yarn.lock
git commit -m "feat(frontend): add debounced position save

Save playback position every 5 seconds (debounced) to avoid
excessive database writes while maintaining accuracy.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Phase 4: Edge Cases & Error Handling

### Task 8: Add error handling and retry logic

**Files:**
- Modify: `applications/desktop/src/providers/TauriPlayerCommandsProvider.tsx`

**Step 1: Wrap savePlaybackSession with retry logic**

Replace the `savePlaybackSession` function:

```typescript
const savePlaybackSession = async (retryCount = 0) => {
  try {
    const state = usePlayerStore.getState();
    const sessionContext = usePlaybackSession.getState();

    if (!state.currentTrack) {
      return;
    }

    await invoke('save_playback_session', {
      session: {
        current_track_id: state.currentTrack.id,
        queue_track_ids: state.queue.map(t => t.id),
        queue_index: state.queueIndex,
        position_seconds: state.duration ? (state.progress / 100) * state.duration : 0,
        volume: state.volume,
        repeat_mode: state.repeatMode,
        shuffle_mode: state.shuffleMode,
        context_type: sessionContext.contextType,
        context_id: sessionContext.contextId,
        was_playing: state.isPlaying,
      },
    });

    console.log('[PERSISTENCE] Session saved');
  } catch (error) {
    console.error('[PERSISTENCE] Failed to save session:', error);

    // Retry once after 1 second
    if (retryCount === 0) {
      console.log('[PERSISTENCE] Retrying save in 1 second...');
      setTimeout(() => savePlaybackSession(1), 1000);
    }
  }
};
```

**Step 2: Add validation to restoreFromDatabase**

In `restoreFromDatabase`, add validation after loading session:

```typescript
// Validate session data
if (session.queue_track_ids.length === 0) {
  console.warn('[PERSISTENCE] Invalid session: empty queue');
  await invoke('clear_playback_session');
  return;
}

if (session.queue_index < 0 || session.queue_index >= session.queue_track_ids.length) {
  console.warn('[PERSISTENCE] Invalid session: queue index out of bounds');
  session.queue_index = 0;
}

if (session.volume < 0 || session.volume > 1) {
  console.warn('[PERSISTENCE] Invalid session: volume out of range');
  session.volume = 0.8;
}
```

**Step 3: Add toast notification for missing tracks**

After filtering valid tracks in `restoreFromDatabase`:

```typescript
const missingCount = tracks.length - validTracks.length;
if (missingCount > 0) {
  console.warn(`[PERSISTENCE] ${missingCount} track(s) were unavailable and skipped`);
  // TODO: Show toast notification when toast system is available
  // toast.info(`${missingCount} track(s) were unavailable and skipped`);
}
```

**Step 4: Verify compilation**

Run: `yarn typecheck`
Expected: No errors

**Step 5: Commit**

```bash
git add applications/desktop/src/providers/TauriPlayerCommandsProvider.tsx
git commit -m "feat(frontend): add error handling and validation

Add retry logic for failed saves, validate restored session data,
and log missing track warnings.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Phase 5: Testing

### Task 9: Add integration test for persistence

**Files:**
- Create: `libraries/soul-audio-desktop/tests/playback_persistence_test.rs`

**Step 1: Write the failing test**

Create new file:

```rust
//! Integration tests for playback state persistence

use soul_audio_desktop::{create_async_device_monitor, DesktopPlayback, PlaybackCommand};
use soul_playback::{PlaybackConfig, QueueTrack, RepeatMode, ShuffleMode};
use soul_storage::settings;
use sqlx::SqlitePool;
use std::path::PathBuf;
use testcontainers::{clients::Cli, Container, RunnableImage};
use testcontainers_modules::postgres::Postgres;

#[tokio::test]
async fn test_save_and_restore_playback_session() {
    // Setup test database
    let docker = Cli::default();
    let pool = setup_test_database(&docker).await;

    // Create test tracks
    let track1 = create_test_track(1, "Track 1");
    let track2 = create_test_track(2, "Track 2");
    let queue = vec![track1.clone(), track2.clone()];

    // Save session
    let user_id = "1";
    settings::set_setting(&pool, user_id, "playback.current_track_id", &serde_json::json!(1)).await.unwrap();
    settings::set_setting(&pool, user_id, "playback.queue_track_ids", &serde_json::json!(vec![1, 2])).await.unwrap();
    settings::set_setting(&pool, user_id, "playback.queue_index", &serde_json::json!(0)).await.unwrap();
    settings::set_setting(&pool, user_id, "playback.volume", &serde_json::json!(0.75)).await.unwrap();
    settings::set_setting(&pool, user_id, "playback.repeat_mode", &serde_json::json!("all")).await.unwrap();

    // Restore session
    let current_track_id = settings::get_setting(&pool, user_id, "playback.current_track_id")
        .await
        .unwrap()
        .and_then(|v| v.as_i64())
        .unwrap();

    let volume = settings::get_setting(&pool, user_id, "playback.volume")
        .await
        .unwrap()
        .and_then(|v| v.as_f64())
        .unwrap();

    // Verify
    assert_eq!(current_track_id, 1);
    assert_eq!(volume, 0.75);
}

fn create_test_track(id: i64, title: &str) -> QueueTrack {
    QueueTrack {
        id: id.to_string(),
        path: format!("test_{}.mp3", id),
        title: title.to_string(),
        artist: Some("Test Artist".to_string()),
        album: Some("Test Album".to_string()),
        duration_secs: 180.0,
        track_number: Some(id as u32),
        disc_number: Some(1),
    }
}

async fn setup_test_database(docker: &Cli) -> SqlitePool {
    // Setup testcontainers SQLite pool
    // TODO: Implement proper testcontainers setup
    unimplemented!()
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test playback_persistence_test --manifest-path libraries/soul-audio-desktop/Cargo.toml`
Expected: FAIL (unimplemented)

**Step 3: Implement test database setup**

Replace `setup_test_database`:

```rust
async fn setup_test_database() -> SqlitePool {
    let pool = SqlitePool::connect(":memory:").await.unwrap();

    // Run migrations
    sqlx::migrate!("../soul-storage/migrations")
        .run(&pool)
        .await
        .unwrap();

    // Create default user
    sqlx::query!("INSERT INTO users (id, username) VALUES (1, 'test_user')")
        .execute(&pool)
        .await
        .unwrap();

    pool
}
```

Update test signature:

```rust
#[tokio::test]
async fn test_save_and_restore_playback_session() {
    let pool = setup_test_database().await;
    // ... rest of test
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test playback_persistence_test --manifest-path libraries/soul-audio-desktop/Cargo.toml`
Expected: PASS

**Step 5: Commit**

```bash
git add libraries/soul-audio-desktop/tests/playback_persistence_test.rs
git commit -m "test: add integration test for playback persistence

Test save and restore cycle for playback session using
in-memory SQLite database.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 10: Add E2E test for full persistence cycle

**Files:**
- Create: `libraries/soul-audio-desktop/tests/playback_state_persistence_e2e_test.rs`

**Step 1: Write E2E test scaffold**

```rust
//! E2E tests for playback state persistence across app restarts

use soul_audio_desktop::{create_async_device_monitor, DesktopPlayback, PlaybackCommand, PlaybackEvent};
use soul_playback::{PlaybackConfig, QueueTrack, RepeatMode, ShuffleMode};
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_happy_path_restore() {
    // 1. Create playback manager and load queue
    let (playback, _monitor, event_rx) = create_test_playback().await;
    let tracks = create_test_tracks();

    playback.send_command(PlaybackCommand::LoadPlaylist {
        tracks: tracks.clone(),
        start_index: 0,
    }).unwrap();

    playback.send_command(PlaybackCommand::Play).unwrap();
    sleep(Duration::from_secs(1)).await;

    // 2. Save state (simulate frontend save)
    let current_track = playback.get_current_track().unwrap();
    let position = playback.get_position();

    // 3. Simulate app restart - drop playback manager
    drop(playback);
    drop(_monitor);

    // 4. Create new playback manager and restore state
    let (new_playback, _new_monitor, _) = create_test_playback().await;

    // Restore queue
    new_playback.send_command(PlaybackCommand::LoadPlaylist {
        tracks: tracks.clone(),
        start_index: 0,
    }).unwrap();

    // Verify state
    let restored_track = new_playback.get_current_track().unwrap();
    assert_eq!(restored_track.id, current_track.id);
}

#[tokio::test]
async fn test_missing_track_handling() {
    // Test that missing tracks are skipped gracefully
    // TODO: Implement
}

async fn create_test_playback() -> (DesktopPlayback, impl Drop, tokio::sync::mpsc::Receiver<PlaybackEvent>) {
    // TODO: Implement
    unimplemented!()
}

fn create_test_tracks() -> Vec<QueueTrack> {
    vec![
        QueueTrack {
            id: "1".to_string(),
            path: "test1.mp3".to_string(),
            title: "Track 1".to_string(),
            artist: Some("Artist".to_string()),
            album: Some("Album".to_string()),
            duration_secs: 180.0,
            track_number: Some(1),
            disc_number: Some(1),
        },
    ]
}
```

**Step 2: Mark as TODO for now**

Add `#[ignore]` attribute:

```rust
#[tokio::test]
#[ignore] // TODO: Implement full E2E test setup
async fn test_happy_path_restore() {
```

**Step 3: Commit**

```bash
git add libraries/soul-audio-desktop/tests/playback_state_persistence_e2e_test.rs
git commit -m "test: add E2E test scaffold for playback persistence

Placeholder for full end-to-end tests including app restart
simulation and missing track handling.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Final Verification

### Task 11: Manual testing and verification

**Step 1: Build and run the app**

```bash
yarn dev:desktop
```

**Step 2: Test hot reload**

1. Play a track
2. Trigger hot reload (save a TypeScript file)
3. Verify sidebar still shows current track
4. Verify playback continues

**Step 3: Test cold start**

1. Play a track and seek to 30 seconds
2. Fully close the app
3. Restart the app
4. Verify track and queue are restored
5. Verify position is restored (paused)
6. Verify volume and modes are preserved

**Step 4: Test missing track edge case**

1. Play a queue of tracks
2. Save state
3. Delete one of the track files from disk
4. Restart app
5. Verify missing track is skipped
6. Verify playback continues with remaining tracks

**Step 5: Document results**

Create test log in `docs/testing/playback-persistence-manual-test.md`

**Step 6: Commit**

```bash
git add docs/testing/playback-persistence-manual-test.md
git commit -m "docs: add manual testing results for playback persistence

Document hot reload, cold start, and edge case testing.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Success Criteria Checklist

Before marking this feature complete, verify:

- [ ] Hot reload preserves playback state (sidebar shows current track)
- [ ] Cold start restores state but remains paused
- [ ] Missing tracks are skipped gracefully
- [ ] Position is restored within 5 seconds accuracy
- [ ] Volume, repeat, shuffle modes are preserved
- [ ] Playback context (album/artist/playlist) is restored
- [ ] All tests pass (unit, integration, E2E)
- [ ] No console errors during normal operation
- [ ] Database writes are debounced (not every 250ms)
- [ ] State clears properly when queue is empty

---

## Additional Notes

- **Logging**: All persistence operations use `tracing::debug!` and `tracing::error!` for debugging
- **Performance**: Position saves are debounced to 5s to reduce database writes
- **Error handling**: Failed saves retry once, corrupted state is cleared
- **User ID**: Always uses `user_id = 1` for desktop (from AppState)
- **Database**: Uses existing `user_settings` table, no migrations needed

---

## Future Enhancements

Once this plan is complete, consider:

1. Add state versioning for schema migrations
2. Add user setting to disable persistence
3. Add "Restore last session" prompt on startup
4. Add telemetry to track persistence success rate
5. Add maximum session age (auto-clear after 7 days)
6. Add toast notifications for missing tracks

---

**Implementation Time Estimate:** 4-6 hours for experienced developer

**Testing Time Estimate:** 1-2 hours for manual verification

**Total Estimated Time:** 5-8 hours
