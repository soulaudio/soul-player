# Playback Debug Fixes

## Issues Fixed

### 1. Missing `coverArtPath` in Album Playback
**File:** `applications/marketing/src/components/demo/LibraryPage.tsx:107`
- Added `coverArtPath: demoTrack?.coverUrl || null` to album queue building
- Now album covers will display properly in the queue

### 2. Cover Art Missing from Queue
**File:** `applications/marketing/src/providers/DemoPlayerCommandsProvider.tsx:100-114`
- WASM doesn't store `coverUrl` (not in Rust struct)
- Added lookup from demo storage when retrieving queue
- Ensures queue items display album art

### 3. Cover Art Missing from Current Track
**File:** `applications/marketing/src/lib/demo/bridge.ts:52-71`
- Current track display was also missing cover art
- Added same lookup pattern: get track from storage by ID
- Now "Now Playing" shows album art

### 5. Wrong Field Names in `getQueue()`
**File:** `applications/marketing/src/providers/DemoPlayerCommandsProvider.tsx:105-106,113`
- Changed `track.duration` → `track.duration_secs` (correct WASM field)
- Changed `trackNumber: null` → `track.track_number || null` (correct WASM field)
- This ensures duration and track numbers are properly retrieved

### 6. WASM Track Change Event Not Firing (Fixed in Rust)
**File:** `libraries/soul-playback/src/wasm/manager.rs:49`
- Root cause: `play()` method wasn't emitting track change events
- Added `self.emit_track_change()` to notify JavaScript
- See `WASM_PLAYBACK_FIX.md` for details

### 7. Enhanced Logging for Debugging

Added comprehensive logging to trace playback flow:

**DemoPlayerCommandsProvider.tsx:**
- Try/catch around playback start with error logging (lines 153-161)

**wasm-playback-adapter.ts:**
- Log play() calls with current state and queue length (lines 114-125)
- Log loadPlaylist() with track count and first track details (lines 183-200)

## Testing Instructions

1. Run the marketing app:
   ```bash
   cd applications/marketing
   yarn dev
   ```

2. Open browser console and click a track

3. Expected log sequence:
   ```
   [LibraryPage] buildQueue called: { totalTracks: 33, clickedTrack: "Dark", clickedIndex: 0 }
   [TrackList] Playing queue with 33 tracks
   [DemoPlayerCommandsProvider] playQueue called: { queueLength: 33, startIndex: 0, firstTrack: "Dark" }
   [DemoPlayerCommandsProvider] Converted to demo queue: { length: 33, firstPath: "/demo-audio/...", allHavePaths: true }
   [DemoPlayerCommandsProvider] Loading playlist to WASM, starting track: Dark
   [WasmPlaybackAdapter] loadPlaylist called with 33 tracks
   [WasmPlaybackAdapter] First track: { id: "1", path: "/demo-audio/dark.flac", duration_secs: 42, ... }
   [WasmPlaybackAdapter] Converted to plain tracks, first: { id: "1", path: "/demo-audio/dark.flac", duration_secs: 42, ... }
   [WasmPlaybackAdapter] loadPlaylist completed, queue length: 33
   [WasmPlaybackAdapter] play() called, current state: stopped
   [WasmPlaybackAdapter] Calling WASM play(), queue length: 33
   [WasmPlaybackAdapter] WASM play() returned
   [WasmPlaybackAdapter] - New state: loading
   [WasmPlaybackAdapter] - Current track from WASM: Dark
   [WasmPlaybackAdapter] - Queue length after play: 32

   --- IF WASM EVENT FIRES (IDEAL) ---
   [WasmPlaybackAdapter] *** onTrackChange callback invoked *** with track
   [WasmPlaybackAdapter] Track change: { id: "1", title: "Dark", ... }
   [WasmPlaybackAdapter] Mapped track: { ... }
   [WebAudioPlayer] Loading track: /demo-audio/dark.flac

   --- IF WASM EVENT DOESN'T FIRE (WORKAROUND) ---
   [WasmPlaybackAdapter] WORKAROUND: Manually loading track since event did not fire
   [WasmPlaybackAdapter] Callback still not fired, manually triggering load
   [WebAudioPlayer] Loading track: /demo-audio/dark.flac

   --- EITHER WAY, SHOULD CONTINUE ---
   [WebAudioPlayer] Playback started (or error if file doesn't exist)
   [DemoPlayerCommandsProvider] Playback started successfully
   ```

## Known Issues to Investigate

### Missing Audio Files
**demo-data.json tracks 1-2 reference non-existent files:**
- `/demo-audio/dark.flac` - File doesn't exist
- `/demo-audio/eyes.flac` - File doesn't exist

Only the "pressures, father I sober" album files exist (tracks 3-33).

**Recommendation:** Either:
1. Add the missing audio files, OR
2. Remove tracks 1-2 from demo-data.json, OR
3. Update their paths to point to existing files

### No Track Change Event

If logs show state changes to "loading" but **no "Track change:" log**, this means:
- WASM's `play()` is not triggering `play_next_in_queue()`, OR
- The track change event callback isn't registered, OR
- There's an error in WASM that's being silently swallowed

**Debug steps:**
1. Check if `[WasmPlaybackAdapter] WASM play() returned` log appears
2. Check browser console for any WASM errors
3. Verify queue length is > 0 before calling play()

## Field Name Reference

### Shared Interface (TypeScript)
```typescript
interface QueueTrack {
  trackId: string;
  title: string;
  artist: string;
  album: string | null;
  filePath: string;
  durationSeconds: number | null;  // ← camelCase
  trackNumber: number | null;       // ← camelCase
  coverArtPath?: string;
}
```

### Demo WASM Interface (TypeScript)
```typescript
interface QueueTrack {
  id: string;
  path: string;
  title: string;
  artist: string;
  album?: string;
  duration_secs: number;   // ← snake_case for WASM
  track_number?: number;   // ← snake_case for WASM
  coverUrl?: string;
}
```

### Conversion Flow
1. LibraryPage builds queue with `durationSeconds` (from DB)
2. DemoPlayerCommandsProvider.playQueue converts to `duration_secs` (for WASM)
3. WasmPlaybackAdapter.loadPlaylist passes plain objects to WASM
4. WASM deserializes with serde, expecting exact field names
5. DemoPlayerCommandsProvider.getQueue converts back from `duration_secs` to `durationSeconds`

## Next Steps

If playback still doesn't start after these fixes:
1. Check console for the full log sequence
2. Identify where the log sequence stops
3. Check for JavaScript errors or WASM panics
4. Verify audio files exist at the specified paths
5. Test with a track that has a valid audio file (e.g., track 3 "BOY IN THE MIRROR")
