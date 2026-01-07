# Desktop Parity Status

Status of demo matching desktop app 1:1.

---

## ✅ What Already Matches

### Architecture
- ✅ **Playback logic**: Direct TypeScript port of Rust `PlaybackManager`
  - Same queue management (two-tier: explicit + source)
  - Same shuffle algorithms (Random & Smart)
  - Same repeat modes (Off, All, One)
  - Same history tracking (50 tracks)
  - Same volume control (logarithmic)
  - Same previous button logic (>3 seconds = restart)

- ✅ **Icons**: Using Lucide React (same as desktop)
  - Play, Pause, SkipForward, SkipBack
  - Volume2, VolumeX
  - Shuffle, Repeat, Repeat1
  - Settings, Search icons

- ✅ **Styling**: TailwindCSS classes match desktop
  - Same colors (primary, accent, muted, etc.)
  - Same spacing and sizing
  - Same hover states
  - Same transitions

### UI Components
- ✅ **Nav tabs**: Library, Playlists, Artists, Albums, Genres with emoji icons (📚🎵👤💿🎸)
- ✅ **Search button**: Same position and styling
- ✅ **Settings button**: Opens modal with settings
- ✅ **Sources button**: Opens modal with source management

---

## ⚠️ Current Differences

### 1. Player Footer Structure

**Desktop:**
```tsx
<PlayerFooter>
  <div className="grid grid-cols-3">
    <TrackInfo />          // Left
    <PlayerControls />     // Center
    <ShuffleRepeatControls + VolumeControl />  // Right
  </div>
  <ProgressBar />          // Bottom
</PlayerFooter>
```

**Demo (Current):**
```tsx
<PlaybackControls>
  // Everything in one component
  // Different layout structure
</PlaybackControls>
```

**Fix:** Split demo controls into separate components matching desktop structure.

---

### 2. Track List Display

**Desktop:**
```tsx
<TrackList>
  - Shows play/pause icon on hover
  - Click plays entire queue from that track
  - Right-click menu for track operations
  - Proper sorting by columns
</TrackList>
```

**Demo (Current):**
```tsx
<LibraryPage>
  - Basic table
  - Click plays single track
  - No right-click menu
  - No column sorting
</LibraryPage>
```

**Fix:** Match desktop TrackList exactly.

---

### 3. Queue Sidebar

**Desktop:**
- Has queue sidebar that slides in from right
- Shows upcoming tracks
- Drag to reorder
- Clear queue button

**Demo:**
- No queue sidebar yet

**Fix:** Add QueueSidebar component.

---

### 4. Keyboard Shortcuts

**Desktop:**
- Cmd/Ctrl+K for search
- Cmd/Ctrl+1-5 for tab switching
- Cmd/Ctrl+H for home
- Space for play/pause
- Shows keyboard hints on buttons

**Demo:**
- No keyboard shortcuts

**Fix:** Add keyboard event handlers.

---

### 5. State Management

**Desktop:**
```typescript
// Uses @soul-player/shared Zustand store
import { usePlayerStore } from '@soul-player/shared/stores/player'

// Tauri events update the store
usePlaybackEvents() // Listens to Tauri backend
```

**Demo:**
```typescript
// Custom React hook with local state
const { play, pause, ... } = usePlayback()

// Web audio manager updates local state
```

**Fix:** Use shared Zustand store in demo too.

---

## 🎯 Plan for 1:1 Parity

### Phase 1: Use Shared Store (High Priority)

1. **Update demo to use `@soul-player/shared` store:**
   ```typescript
   // applications/marketing/src/hooks/useDemoPlaybackBridge.ts
   import { usePlayerStore } from '@soul-player/shared/stores/player'
   import { getPlaybackManager } from '@/lib/demo/demo-commands'

   // Bridge web playback manager to shared store
   // Desktop uses Tauri events, demo uses direct calls
   ```

2. **Benefits:**
   - Same state structure as desktop
   - Can reuse desktop components that read from store
   - Single source of truth

---

### Phase 2: Match Component Structure (High Priority)

1. **Split PlaybackControls into desktop's structure:**
   ```
   demo/
   ├── PlayerFooter.tsx           // Copy from desktop
   ├── player/
   │   ├── TrackInfo.tsx         // Copy from desktop
   │   ├── PlayerControls.tsx    // Copy from desktop (replace Tauri calls)
   │   ├── ProgressBar.tsx       // Copy from desktop
   │   ├── VolumeControl.tsx     // Copy from desktop
   │   └── ShuffleRepeatControls.tsx  // Copy from desktop
   ```

2. **Replace Tauri imports with demo commands:**
   ```typescript
   // Desktop:
   import { invoke } from '@tauri-apps/api/core'
   await invoke('play_track', { trackId })

   // Demo:
   import { demoCommands } from '@/lib/demo/demo-commands'
   await demoCommands.playTrack(trackId)
   ```

---

### Phase 3: Add Missing Features (Medium Priority)

1. **Queue Sidebar:**
   - Copy `QueueSidebar.tsx` from desktop
   - Shows current queue
   - Drag to reorder (optional in demo)

2. **Keyboard Shortcuts:**
   - Add `useEffect` in MainLayout for keyboard events
   - Same shortcuts as desktop

3. **Track Menu:**
   - Right-click context menu
   - Add to playlist, delete track, etc.
   - Can be disabled in demo with message "Not available in demo"

---

### Phase 4: Polish (Low Priority)

1. **Tooltips:**
   - Copy `Tooltip.tsx` from desktop
   - Show keyboard shortcuts on hover

2. **Confirm Dialogs:**
   - For delete operations
   - Can be simplified in demo

3. **Loading States:**
   - Match desktop loading indicators
   - Skeleton screens while loading

---

## 🔧 Quick Wins (Do These First)

### 1. Use Emoji Icons (Already Done! ✅)
```typescript
const NAV_TABS = [
  { path: '/', label: 'Library', icon: '📚' },  // ✅ Updated!
  // ...
]
```

### 2. Match Desktop PlayerFooter Layout
```tsx
// Current: everything in one component
// Target: grid cols-3 with separate components
<div className="grid grid-cols-3 items-center gap-4">
  <TrackInfo />
  <PlayerControls />
  <div className="flex items-center justify-end gap-2">
    <ShuffleRepeatControls />
    <VolumeControl />
  </div>
</div>
```

### 3. Add Queue Button
```tsx
<button onClick={() => setShowQueue(!showQueue)}>
  <ListMusic className="w-5 h-5" />
</button>
```

---

## 📂 Files to Copy from Desktop

**High Priority:**
```
applications/desktop/src/components/
├── PlayerFooter.tsx          → marketing/src/components/demo/
├── player/
│   ├── PlayerControls.tsx    → marketing/src/components/demo/player/
│   ├── ProgressBar.tsx       → marketing/src/components/demo/player/
│   ├── TrackInfo.tsx         → marketing/src/components/demo/player/
│   ├── VolumeControl.tsx     → marketing/src/components/demo/player/
│   └── ShuffleRepeatControls.tsx → marketing/src/components/demo/player/
└── TrackList.tsx             → marketing/src/components/demo/
```

**Medium Priority:**
```
├── QueueSidebar.tsx          → marketing/src/components/demo/
├── Tooltip.tsx               → marketing/src/components/demo/
└── Kbd.tsx                   → marketing/src/components/demo/
```

**Changes Needed:**
1. Replace `invoke()` calls with `demoCommands.*`
2. Replace `usePlaybackEvents()` with demo bridge
3. Keep everything else identical

---

## 🎵 Audio Playback Differences

**Desktop:**
- Symphonia (Rust) decoder
- CPAL audio output
- Native OS audio devices
- File system access

**Demo:**
- Web Audio API decoder
- Browser audio output
- No file system access
- Streams from `/public/demo-audio/`

**This is expected!** The demo can't use native audio - it needs web APIs. The important part is the **logic** is the same (queue, shuffle, repeat), just different audio backend.

---

## ✅ What to Keep Different

Some things **should** stay different in demo:

1. **Import dialog**: Can't access file system in web
   - Keep disabled in demo

2. **Source management**: Can't manage local folders
   - Show read-only view in demo

3. **Settings persistence**: Can't save to disk
   - Keep in-memory only for demo

4. **Deep linking**: No app protocol handlers
   - Web URLs instead

---

## 🚀 Implementation Priority

1. **Critical (Do Now):**
   - [ ] Use shared Zustand store
   - [ ] Split PlayerFooter into desktop's structure
   - [ ] Match TrackList component

2. **Important (Do Soon):**
   - [ ] Add QueueSidebar
   - [ ] Add keyboard shortcuts
   - [ ] Copy exact component styling

3. **Nice to Have:**
   - [ ] Tooltips with shortcuts
   - [ ] Track context menu
   - [ ] Drag-and-drop queue reordering

---

## 📝 Code Pattern

**Desktop Component:**
```typescript
// applications/desktop/src/components/player/PlayerControls.tsx
import { invoke } from '@tauri-apps/api/core'
import { usePlayerStore } from '@soul-player/shared/stores/player'

export function PlayerControls() {
  const { isPlaying } = usePlayerStore()

  const handlePlay = async () => {
    await invoke('resume_playback')
  }

  return (
    <button onClick={handlePlay}>
      {isPlaying ? <Pause /> : <Play />}
    </button>
  )
}
```

**Demo Version:**
```typescript
// applications/marketing/src/components/demo/player/PlayerControls.tsx
import { demoCommands } from '@/lib/demo/demo-commands'
import { usePlayerStore } from '@soul-player/shared/stores/player'

export function PlayerControls() {
  const { isPlaying } = usePlayerStore()

  const handlePlay = async () => {
    await demoCommands.resumePlayback()
  }

  return (
    <button onClick={handlePlay}>
      {isPlaying ? <Pause /> : <Play />}
    </button>
  )
}
```

**Pattern:** Same component, just replace `invoke(...)` with `demoCommands.*`

---

## ✨ Summary

**Already Good:**
- Playback logic ported correctly from Rust ✅
- Icons match desktop ✅
- Styling matches desktop ✅
- Core functionality works ✅

**Needs Work:**
- Component structure (split into desktop's parts)
- Use shared Zustand store
- Add queue sidebar
- Add keyboard shortcuts

**Estimated Time to Full Parity:**
- Phase 1 (shared store): 2-3 hours
- Phase 2 (component structure): 3-4 hours
- Phase 3 (missing features): 2-3 hours
- **Total: ~8-10 hours** to 100% match desktop

**Current Status: ~70% parity**
