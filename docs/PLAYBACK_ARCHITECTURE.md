# Playback Architecture

**Last Updated**: 2026-01-12

This document provides a comprehensive overview of Soul Player's playback architecture, including the separation of concerns between data and control, event flow, and best practices.

---

## Table of Contents

1. [Overview](#overview)
2. [Separation of Concerns](#separation-of-concerns)
3. [Frontend Architecture](#frontend-architecture)
4. [Backend Architecture](#backend-architecture)
5. [Command Flow](#command-flow)
6. [Event System](#event-system)
7. [State Management](#state-management)
8. [Common Patterns](#common-patterns)
9. [Anti-Patterns to Avoid](#anti-patterns-to-avoid)
10. [Adding New Features](#adding-new-features)

---

## Overview

Soul Player's playback system is built on a **strict separation of concerns**:

- **BackendContext**: Handles library data fetching (tracks, albums, playlists)
- **PlayerCommandsContext**: Handles playback control (play, pause, seek, volume)

This separation ensures:
- ✅ Clear boundaries between data and control logic
- ✅ No duplicate implementations
- ✅ Platform-agnostic UI components
- ✅ Maintainable codebase

**⚠️ CRITICAL**: These two contexts must NEVER overlap. Methods should exist in ONE context only.

---

## Separation of Concerns

### BackendContext - Library Data

**Purpose**: Fetch library data and record playback context.

**Responsibilities**:
- Query database for tracks, albums, artists, playlists, genres
- Fetch related data (album tracks, artist albums, etc.)
- Record playback context for "Jump Back In" feature
- Health checks and diagnostic queries

**What it does NOT do**:
- ❌ Playback control (play, pause, skip)
- ❌ Queue management
- ❌ Volume/seek operations
- ❌ Audio state management

**Key Files**:
- `applications/shared/src/contexts/BackendContext.tsx` - Interface definition
- `applications/desktop/src/providers/TauriBackendProvider.tsx` - Desktop implementation
- `applications/shared/src/providers/MockBackendProvider.tsx` - Mock implementation (for marketing demo)

### PlayerCommandsContext - Playback Control

**Purpose**: Control audio playback and manage audio state.

**Responsibilities**:
- Playback commands: play, pause, resume, stop
- Navigation: skipNext, skipPrevious, skipToQueueIndex
- Audio control: seek, setVolume, setShuffle, setRepeatMode
- Queue management: getQueue, playQueue
- Event subscriptions: onStateChange, onTrackChange, etc.

**What it does NOT do**:
- ❌ Database queries
- ❌ Library data fetching
- ❌ File system operations
- ❌ Playlist/album metadata

**Key Files**:
- `applications/shared/src/contexts/PlayerCommandsContext.tsx` - Interface definition
- `applications/desktop/src/providers/TauriPlayerCommandsProvider.tsx` - Desktop implementation
- `applications/marketing/src/providers/DemoPlayerCommandsProvider.tsx` - Demo implementation (WASM audio)

---

## Frontend Architecture

### Context Hierarchy

```
App Root
├── PlatformProvider (detects desktop/web)
├── BackendProvider (data fetching)
│   └── TauriBackendProvider OR MockBackendProvider
├── PlayerCommandsProvider (playback control)
│   └── TauriPlayerCommandsProvider OR DemoPlayerCommandsProvider
└── Page Components
    ├── Use useBackend() for data
    └── Use usePlayerCommands() for control
```

### Hooks

**Data Hooks**:
```typescript
const backend = useBackend()
// Available methods:
// - getAllTracks(), getAlbumTracks(id), getPlaylistTracks(id)
// - getAllAlbums(), getAlbumById(id)
// - getAllArtists(), getArtistById(id)
// - recordContext(context)
```

**Control Hooks**:
```typescript
const commands = usePlayerCommands()
// Available methods:
// - playQueue(queue, startIndex)
// - pausePlayback(), resumePlayback(), stopPlayback()
// - skipNext(), skipPrevious()
// - seek(position), setVolume(volume)
// - setShuffle(enabled), setRepeatMode(mode)

const events = usePlaybackEvents()
// Available subscriptions:
// - onStateChange(callback)
// - onTrackChange(callback)
// - onPositionUpdate(callback)
// - onVolumeChange(callback)
// - onQueueUpdate(callback)
// - onError(callback)
```

### State Management

**Zustand Store** (`player.ts`):
```typescript
interface PlayerStore {
  // Current playback state
  currentTrack: Track | null
  isPlaying: boolean
  volume: number           // 0-1 (UI scale)
  progress: number         // 0-100
  duration: number

  // Queue state
  queue: QueueTrack[]

  // Settings
  repeatMode: 'off' | 'all' | 'one'
  shuffleEnabled: boolean
}
```

**State Updates**: Zustand store is automatically updated by Tauri events:
- `playback:state-changed` → updates `isPlaying`
- `playback:track-changed` → updates `currentTrack`
- `playback:position-updated` → updates `progress`
- `playback:volume-changed` → updates `volume`
- `playback:queue-updated` → triggers queue refresh

---

## Backend Architecture

### Rust Component Stack

```
┌──────────────────────────────────────────────────────────┐
│  Tauri Commands (main.rs)                                │
│  - play_queue(), pause_playback(), set_volume()          │
│  - Route commands to PlaybackManager                     │
├──────────────────────────────────────────────────────────┤
│  PlaybackManager (playback.rs)                           │
│  - Wraps DesktopPlayback                                 │
│  - Emits Tauri events to frontend                        │
│  - Thread-safe command routing                           │
├──────────────────────────────────────────────────────────┤
│  DesktopPlayback (soul-audio-desktop)                    │
│  - Platform integration (CPAL audio output)              │
│  - Device management (ASIO/WASAPI/CoreAudio)             │
│  - Audio stream management                               │
├──────────────────────────────────────────────────────────┤
│  PlaybackManager (soul-playback)                         │
│  - Core playback orchestration                           │
│  - Queue management (source + explicit tiers)            │
│  - Crossfade engine                                      │
│  - Volume control & ReplayGain                           │
├──────────────────────────────────────────────────────────┤
│  AudioSource + Decoder (soul-audio)                      │
│  - Symphonia decoder integration                         │
│  - Format support (FLAC, MP3, AAC, Opus, etc.)           │
│  - Metadata extraction                                   │
└──────────────────────────────────────────────────────────┘
```

### Command Processing

**Example: play_queue() flow**

1. **Frontend**: `commands.playQueue(queue, 0)` called
2. **Tauri IPC**: `invoke('play_queue', {queue, startIndex})`
3. **Rust Command**: `play_queue` handler in `main.rs`
4. **PlaybackManager**:
   ```rust
   playback.stop()              // Clear current playback
   playback.load_playlist(tracks) // Load as source queue
   playback.play()              // Start playback
   ```
5. **Event Emission**:
   - `playback:state-changed` (Stopped → Playing)
   - `playback:track-changed` (new track info)
   - `playback:queue-updated` (new queue loaded)
6. **Frontend**: Events received, Zustand store updated, UI re-renders

---

## Command Flow

### Playing an Album (Step-by-Step)

```typescript
// 1. User clicks "Play" on an album card
async function handlePlayAlbum(albumId: number) {
  // 2. Fetch album data
  const backend = useBackend()
  const tracks = await backend.getAlbumTracks(albumId)
  const album = await backend.getAlbumById(albumId)

  // 3. Filter playable tracks
  const playableTracks = tracks.filter(t => t.file_path)

  // 4. Transform to queue format
  const queue = playableTracks.map(t => ({
    trackId: String(t.id),
    title: t.title,
    artist: t.artist_name || 'Unknown',
    album: t.album_title || null,
    filePath: t.file_path!,
    durationSeconds: t.duration_seconds || null,
    trackNumber: t.track_number || null,
  }))

  // 5. Record playback context (for "Jump Back In")
  await backend.recordContext({
    contextType: 'album',
    contextId: String(albumId),
    contextName: album.title,
    contextArtworkPath: album.cover_art_path || null,
  })

  // 6. Start playback
  const commands = usePlayerCommands()
  await commands.playQueue(queue, 0)

  // 7. Events fire automatically, UI updates
}
```

---

## Event System

### Event Types

**Desktop (Tauri Events)**:
```typescript
// Emitted by Rust backend
'playback:state-changed'     // { state: 'Playing' | 'Paused' | 'Stopped' }
'playback:track-changed'     // { id, title, artist, album, duration, coverArtPath }
'playback:position-updated'  // number (seconds)
'playback:volume-changed'    // number (0-100)
'playback:queue-updated'     // void (trigger queue refresh)
'playback:error'             // string (error message)
'playback:sample-rate-changed' // { from: Hz, to: Hz }
'playback:crossfade-started' // { fromTrackId, toTrackId, durationMs }
'playback:crossfade-progress' // { progress: 0-1, metadataSwitched: boolean }
'playback:crossfade-completed' // void
```

**Marketing Demo (WASM Events)**:
- Similar events emitted via custom event system
- Web Audio API integration for playback
- No Tauri IPC overhead

### Event Listeners

```typescript
// In TauriPlayerCommandsProvider.tsx
useEffect(() => {
  const unlisten = listen('playback:track-changed', (event) => {
    const track = event.payload
    usePlayerStore.setState({ currentTrack: track })
  })

  return () => { unlisten() }
}, [])
```

---

## State Management

### Sources of Truth

1. **Rust Backend** (primary):
   - Current playback state (Playing/Paused/Stopped)
   - Current track and position
   - Queue (source + explicit)
   - Volume level
   - Shuffle/repeat modes

2. **Frontend Zustand Store** (cache):
   - Synchronized via Tauri events
   - Used for UI rendering
   - May lag slightly behind backend

### Synchronization Strategy

**One-way flow**: Backend → Events → Frontend

```
Rust Backend (source of truth)
  ↓ (emits events)
Tauri Event Bridge
  ↓ (updates)
Zustand Store (cache)
  ↓ (renders)
UI Components
```

**Commands flow opposite direction**:

```
UI Component
  ↓ (calls)
PlayerCommands Hook
  ↓ (invokes)
Tauri IPC
  ↓ (processes)
Rust Backend (updates state, emits events)
```

---

## Common Patterns

### Pattern 1: Play from Context

```typescript
// ✅ CORRECT
async function playFromContext(
  contextType: 'album' | 'artist' | 'playlist',
  contextId: number | string,
  contextName: string
) {
  const backend = useBackend()
  const commands = usePlayerCommands()

  // 1. Fetch data
  let tracks: BackendTrack[]
  switch (contextType) {
    case 'album':
      tracks = await backend.getAlbumTracks(Number(contextId))
      break
    case 'artist':
      tracks = await backend.getArtistTracks(Number(contextId))
      break
    case 'playlist':
      tracks = await backend.getPlaylistTracks(String(contextId))
      break
  }

  // 2. Build queue
  const queue = buildQueue(tracks)

  // 3. Record context
  await backend.recordContext({
    contextType,
    contextId: String(contextId),
    contextName,
    contextArtworkPath: null,
  })

  // 4. Play
  await commands.playQueue(queue, 0)
}
```

### Pattern 2: Play Single Track

```typescript
// ✅ CORRECT
async function playSingleTrack(track: BackendTrack) {
  const commands = usePlayerCommands()

  const queue = [{
    trackId: String(track.id),
    title: track.title,
    artist: track.artist_name || 'Unknown',
    album: track.album_title || null,
    filePath: track.file_path!,
    durationSeconds: track.duration_seconds || null,
    trackNumber: track.track_number || null,
  }]

  await commands.playQueue(queue, 0)
}
```

### Pattern 3: Skip Within Queue

```typescript
// ✅ CORRECT
async function skipToTrack(queueIndex: number) {
  const commands = usePlayerCommands()
  await commands.skipToQueueIndex(queueIndex)
}
```

---

## Anti-Patterns to Avoid

### ❌ Anti-Pattern 1: Adding playQueue to BackendContext

```typescript
// WRONG - playQueue is playback control, not data
export interface BackendInterface {
  getAllTracks: () => Promise<Track[]>
  playQueue: (queue, index) => Promise<void>  // ❌ Don't!
}
```

**Why**: Violates separation of concerns. Playback control belongs in PlayerCommandsContext.

### ❌ Anti-Pattern 2: Mixing Backend and Commands

```typescript
// WRONG - inconsistent interface usage
const backend = useBackend()
const commands = usePlayerCommands()

const tracks = await backend.getAllTracks()
await backend.playQueue(tracks, 0)  // ❌ Should use commands!
```

**Why**: Creates confusion about which context to use and can lead to duplicate implementations.

### ❌ Anti-Pattern 3: Direct invoke() in Shared Pages

```typescript
// WRONG - bypasses abstraction layer
import { invoke } from '@tauri-apps/api/core'

function LibraryPage() {
  const tracks = await invoke('get_all_tracks')  // ❌
}
```

**Why**: Breaks marketing demo compatibility. Always use `useBackend()` or `usePlayerCommands()`.

### ❌ Anti-Pattern 4: Duplicate Queue Building Logic

```typescript
// WRONG - logic duplicated in multiple places
function AlbumPage() {
  const queue = tracks.map(t => ({ trackId: t.id, ... }))
  await backend.playQueue(queue, 0)
}

function ArtistPage() {
  const queue = tracks.map(t => ({ trackId: t.id, ... }))  // Duplicate!
  await commands.playQueue(queue, 0)
}
```

**Why**: DRY violation. Extract to shared utility function `buildQueue()`.

---

## Adding New Features

### Adding a Data Operation

**Example**: Add `getTracksByGenre(genreId)`

1. **Update BackendContext interface**:
   ```typescript
   // applications/shared/src/contexts/BackendContext.tsx
   export interface BackendInterface {
     // ...existing methods
     getTracksByGenre: (genreId: number) => Promise<BackendTrack[]>
   }
   ```

2. **Implement in TauriBackendProvider**:
   ```typescript
   // applications/desktop/src/providers/TauriBackendProvider.tsx
   async getTracksByGenre(genreId: number) {
     return invoke<BackendTrack[]>('get_tracks_by_genre', { genreId })
   }
   ```

3. **Implement in MockBackendProvider**:
   ```typescript
   // applications/shared/src/providers/MockBackendProvider.tsx
   async getTracksByGenre(genreId: number) {
     return storage.getTracksByGenre(genreId)
       .map((t, i) => toBackendTrack(t, i))
   }
   ```

4. **Add Rust command** (if needed for desktop):
   ```rust
   // applications/desktop/src-tauri/src/main.rs
   #[tauri::command]
   async fn get_tracks_by_genre(
     pool: State<'_, SqlitePool>,
     genre_id: i64,
   ) -> Result<Vec<Track>, String> {
     soul_storage::tracks::get_by_genre(&pool, 1, genre_id)
       .await
       .map_err(|e| e.to_string())
   }
   ```

### Adding a Control Operation

**Example**: Add `skipBackward(seconds)`

1. **Update PlayerCommandsContext interface**:
   ```typescript
   // applications/shared/src/contexts/PlayerCommandsContext.tsx
   export interface PlayerCommandsInterface {
     // ...existing methods
     skipBackward: (seconds: number) => Promise<void>
   }
   ```

2. **Implement in TauriPlayerCommandsProvider**:
   ```typescript
   // applications/desktop/src/providers/TauriPlayerCommandsProvider.tsx
   async skipBackward(seconds: number) {
     await invoke('skip_backward', { seconds })
   }
   ```

3. **Implement in Demo Provider** (if applicable)
4. **Add Rust command**:
   ```rust
   #[tauri::command]
   async fn skip_backward(
     playback: State<'_, Arc<PlaybackManager>>,
     seconds: f64,
   ) -> Result<(), String> {
     let current_pos = playback.get_position();
     let new_pos = (current_pos - seconds).max(0.0);
     playback.seek(new_pos)
   }
   ```

### Checklist

Before adding a new feature, ask:

- [ ] Is this data fetching or playback control?
- [ ] Does this belong in BackendContext or PlayerCommandsContext?
- [ ] Have I updated both desktop AND demo implementations?
- [ ] Have I tested on both platforms?
- [ ] Does this duplicate any existing functionality?
- [ ] Are types exported from `applications/shared/src/index.ts`?

---

## Key Takeaways

1. **Strict Separation**: BackendContext = data, PlayerCommandsContext = control
2. **No Duplication**: Methods exist in ONE context only
3. **Event-Driven**: Backend emits events, frontend reacts
4. **Platform-Agnostic**: Shared pages use hooks, not `invoke()` directly
5. **State Flow**: One-way flow from backend → events → frontend
6. **Clear Boundaries**: Never mix data fetching with playback control

**When in doubt, ask**: "Is this fetching data or controlling audio?"
- Data → BackendContext
- Control → PlayerCommandsContext

---

**See Also**:
- [CLAUDE.md](../CLAUDE.md) - Quick reference and project rules
- [ARCHITECTURE.md](./ARCHITECTURE.md) - Overall system architecture
- [CONVENTIONS.md](./CONVENTIONS.md) - Coding conventions
