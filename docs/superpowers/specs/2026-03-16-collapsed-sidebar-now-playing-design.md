# Collapsed Sidebar + Now Playing Floating Bar

**Date:** 2026-03-16
**Status:** Revised (post-review v4)

---

## Overview

When the user drags the sidebar past a minimum threshold, it snaps fully shut. A thin, clean edge strip replaces it on the left edge as a restore indicator. A "Now Playing" wide-bar floats automatically centered on screen whenever the sidebar is collapsed and a track is playing.

---

## Decisions

| Question | Decision |
|---|---|
| Collapse trigger | Drag past 200px threshold → snap to 0 |
| Restore trigger | Drag edge strip rightward delta past 40px → expand to saved width |
| Edge strip style | 4–6px, `bg-foreground/10` default, `hover:bg-foreground/20` on hover (opacity-only, CSS-first) |
| Modal trigger | Always visible when collapsed + track playing |
| Modal layout | Wide bar: artwork left, title/artist + seek bar + shuffle/prev/play/next/repeat right |
| Modal backdrop | None — floats freely over content |
| Approach | `SidebarStateProvider` component owns hook + context; `LeftSidebar` and `NowPlayingFloating` are consumers |

---

## Architecture

### State Propagation: `SidebarStateProvider`

**The core problem:** `LeftSidebar` and `NowPlayingFloating` are siblings inside shared `MainLayout`. They both need collapse state, but neither can be a parent of the other. `NowPlayingFloating` cannot consume a context provided by a sibling.

**Solution:** A `SidebarStateProvider` wrapper component lives inside shared `MainLayout.tsx`. It calls `useCollapsibleSidebar()` and provides the full state to all children via `SidebarStateContext`. Both `LeftSidebar` and `NowPlayingFloating` are children of this provider and consume the context.

```
SharedMainLayout
  └── SidebarStateProvider             ← owns useCollapsibleSidebar(), provides context
        ├── <LeftSidebar />            ← consumes useSidebarState() context
        └── <NowPlayingFloating />     ← consumes useSidebarState() context
```

`SidebarStateProvider` is a single-purpose wrapper component, not a context-only file. It renders `<SidebarStateContext.Provider value={...}>{children}</SidebarStateContext.Provider>` and is the sole caller of `useCollapsibleSidebar()`.

This placement means `NowPlayingFloating` will also render on the marketing and web platforms when collapsed — which is correct behaviour since `MockBackendProvider` returns no current track, so the floating bar will never render (the `currentTrack !== null` guard prevents it).

---

### New: `useCollapsibleSidebar.ts`

Location: `applications/shared/src/hooks/useCollapsibleSidebar.ts`

This hook **re-implements** `handleMouseMove` entirely rather than composing `useResizableSidebar`'s internal handlers. `useResizableSidebar` clamps width to `minWidth=240` on every move event — delegating to it would prevent the collapse threshold from ever triggering. The new hook borrows the localStorage key names and default values from `useResizableSidebar` but owns all mouse event logic itself.

```typescript
interface CollapsibleSidebarState {
  width: number           // 0 when collapsed, otherwise current resize width
  isCollapsed: boolean
  isResizing: boolean     // true during active drag (used for body cursor + user-select)
  savedWidth: number      // last non-zero width, persisted to localStorage
  handleMouseDown: (e: React.MouseEvent) => void
  expand: (width?: number) => void
  resizableRef: React.RefObject<HTMLDivElement>
}
```

**Initialisation on mount (localStorage):**
1. Read `soul-player:sidebar-collapsed` (boolean string) → if `"true"`, set `isCollapsed=true`, `width=0`
2. Read `soul-player:sidebar-saved-width` (number string) → `savedWidth` (default 288 if absent)
3. If not collapsed: read `soul-player:sidebar-width` as starting width (default 288)
4. While `isCollapsed=true`, do **not** write to `soul-player:sidebar-width` — prevents clobbering saved width

**Mouse handling:**
- `handleMouseDown`: set `isResizing=true`, attach `mousemove`/`mouseup` to `document`, set body `cursor: ew-resize` and `user-select: none` (same pattern as `useResizableSidebar`)
- `handleMouseMove`: compute `newWidth = e.clientX - resizableRef.current.getBoundingClientRect().left`
  - If `newWidth < 200`: **snap** — set `width=0`, `isCollapsed=true`, persist `sidebar-collapsed=true`, persist `sidebar-saved-width=currentNonZeroWidth`, clean up listeners
  - Else: clamp to `[minWidth=240, maxWidth=480]`, set `width=newWidth`
- `handleMouseUp`: finalise width, persist `sidebar-width`, set `isResizing=false`, remove listeners, restore body cursor

**`expand(width?)`:** Sets `width = width ?? savedWidth`, `isCollapsed=false`, persists `sidebar-collapsed=false`, persists `sidebar-width=restoredWidth`.

---

### New: `SidebarStateContext.tsx`

Location: `applications/shared/src/contexts/SidebarStateContext.tsx`

```typescript
interface SidebarStateContextValue {
  width: number
  isCollapsed: boolean
  isResizing: boolean
  savedWidth: number
  handleMouseDown: (e: React.MouseEvent) => void
  expand: (width?: number) => void
  resizableRef: React.RefObject<HTMLDivElement>
}

// Default is a no-op context — fails silently outside provider (graceful degradation)
const SidebarStateContext = createContext<SidebarStateContextValue>({ ... defaults ... })

export const useSidebarState = () => useContext(SidebarStateContext)

// The provider component — owns the hook call
export const SidebarStateProvider = ({ children }: { children: ReactNode }) => {
  const state = useCollapsibleSidebar()
  return (
    <SidebarStateContext.Provider value={state}>
      {children}
    </SidebarStateContext.Provider>
  )
}
```

---

### Modified: Shared `MainLayout.tsx`

Location: `applications/shared/src/layouts/MainLayout.tsx`

- Wrap `<LeftSidebar />` and main content (including `<NowPlayingFloating />`) inside `<SidebarStateProvider>`
- Render `<NowPlayingFloating />` as a sibling to main content area inside the provider
- No new props to `MainLayout` itself

---

### Modified: `LeftSidebar.tsx`

- Remove `useResizableSidebar` call — state now comes from `useSidebarState()` context
- Consume `{ width, isCollapsed, isResizing, handleMouseDown, expand, resizableRef }` from context
- When `isCollapsed === true`: render `<CollapsedSidebarStrip onExpand={expand} />` instead of full sidebar content
- Add `data-collapsed={isCollapsed}` to the root container element
- `LeftSidebarProps` interface: **no new props added** — all state from context

---

### New: `CollapsedSidebarStrip.tsx`

Location: `applications/shared/src/components/CollapsedSidebarStrip.tsx`

```typescript
interface CollapsedSidebarStripProps {
  onExpand: () => void
}
```

- Width: 4–6px, full height
- Styling: `bg-foreground/10 hover:bg-foreground/20 transition-opacity cursor-ew-resize` — opacity-only change, no color change (CLAUDE.md §9)
- `aria-label={t('sidebar.expand')}`
- `data-testid="collapsed-sidebar-strip"`
- `role="separator"` with `aria-orientation="vertical"`

**Drag-to-restore logic (delta-based, not absolute-X):**
- `mousedown`: record `startX = e.clientX`, set `isDragging=true`, attach document-level `mousemove`/`mouseup`
- `mousemove`: compute `delta = e.clientX - startX`. If `delta > 40`: call `onExpand()`, remove listeners (one-shot — does not re-collapse on drag-back once expanded)
- `mouseup`: remove listeners, reset `isDragging=false`

---

### New: `NowPlayingFloating.tsx`

Location: `applications/shared/src/components/NowPlayingFloating.tsx`

- Consumes `useSidebarState()` for `isCollapsed`
- Reads `currentTrack` from existing Zustand player store (`usePlayerStore`)
- Renders only when `isCollapsed && currentTrack !== null`
- Position: `fixed left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 z-50`
- Size: `max-w-[560px] w-[90vw]` — wide horizontal bar
- Card: `bg-card border border-border rounded-xl shadow-lg`
- `data-testid="now-playing-floating"`

**Testids inside this component:**
- `data-testid="floating-now-playing-title"` — track title element (distinct from `now-playing-title` used in `NowPlayingPanel` inside the sidebar, which is absent when collapsed)
- `data-testid="floating-now-playing-artist"` — artist element
- `data-testid="floating-progress-bar"` — progress bar wrapper

**Layout:**
```
┌──────────────────────────────────────────────────────┐
│ [artwork 56×56]  │  Title              [floating-now-playing-title]  │
│                  │  Artist · Album     [floating-now-playing-artist] │
│                  │  ━━━━━━━━━━━━━━━━   1:23 / 3:45                   │
│                  │  ⇄   ⏮   ▶   ⏭   ↺                               │
└──────────────────────────────────────────────────────┘
```

**Reuse:**
- `<ProgressBar />` — mounts a second instance; reads from the same interpolation hook. Safe — two instances reading the same Zustand state are intentional.
- `<PlaybackControls />` — receives handlers from `usePlaybackHandlers()` (see below)

---

### New: `usePlaybackHandlers.ts`

Location: `applications/shared/src/hooks/usePlaybackHandlers.ts`

Extracts handler logic that is currently duplicated inside `PlayerPanel`. Both `PlayerPanel` and `NowPlayingFloating` consume this hook.

```typescript
interface PlaybackHandlers {
  onPlayPause: () => Promise<void>
  onNext: () => Promise<void>
  onPrevious: () => Promise<void>
  onShuffleToggle: () => Promise<void>  // cycles mode + writes to store
  onRepeatToggle: () => Promise<void>   // cycles mode + writes to store with rollback
}
```

**Important — shuffle and repeat side-effects:**

The existing `PlayerPanel` uses a prop-callback pattern: `handleShuffleToggle` calls `commands.cycleShuffle()` then invokes `onShuffleModeChange(newMode)` — a prop — which bubbles up to `LeftSidebar.handleShuffleModeChange`, which calls `setShuffleMode(mode)` from `usePlayerModes()`. `usePlaybackHandlers` **cannot use this prop chain** because it has no parent to callback to.

Instead, `usePlaybackHandlers` calls `usePlayerModes()` **directly inside the hook** and writes to the Zustand store itself:
- `onShuffleToggle`: calls `commands.cycleShuffle()`, receives returned mode, calls `setShuffleMode(newMode)` from `usePlayerModes()` inline. No prop callback.
- `onRepeatToggle`: reads current repeat mode from store, performs optimistic `setRepeatMode`, calls `commands.cycleRepeat()`, rolls back `setRepeatMode` on error. No prop callback.
- All commands via `usePlayerCommands()` — never `invoke()` directly.

**`PlayerPanel` prop interface change (breaking internal refactor):**

As part of this refactor, the following props are **removed** from `PlayerPanel`:
- `shuffleMode` — now read from Zustand store inside `usePlaybackHandlers`
- `repeatMode` — same
- `onShuffleModeChange` — replaced by hook's internal store write
- `onRepeatModeChange` — replaced by hook's internal store write

`LeftSidebar.tsx` is updated to stop passing these props to `PlayerPanel`. This is an internal refactor — no external API change. `PlayerPanel`'s visible behaviour is unchanged.

Both `PlayerPanel` and `NowPlayingFloating` call `usePlaybackHandlers()` and pass the returned handlers to `<PlaybackControls />`. Both also read `shuffleMode`/`repeatMode` for display directly from the Zustand store via `usePlayerStore`.

---

## Data Flow

```
SharedMainLayout
  └── SidebarStateProvider
        │   (calls useCollapsibleSidebar() — single source of collapse state)
        │
        ├── <LeftSidebar />
        │     ├── useSidebarState()             ← reads from context
        │     ├── isCollapsed=false → full sidebar (NavBar, Queue, PlayerPanel, SettingsFooter)
        │     └── isCollapsed=true  → <CollapsedSidebarStrip onExpand={expand} />
        │
        └── main content + <NowPlayingFloating />
              └── useSidebarState()             ← reads isCollapsed from same context
              └── usePlayerStore()              ← reads currentTrack from Zustand
```

---

## Styling Rules

Follows CLAUDE.md §9 CSS-first conventions:
- Edge strip idle: `bg-foreground/10`
- Edge strip hover: `bg-foreground/20` — background opacity increase only, no color change
- Transition: `transition-opacity` (not `transition-colors`)
- Floating bar card: `bg-card border border-border rounded-xl shadow-lg`
- State attribute: `data-collapsed="true/false"` on sidebar root
- No hardcoded colors — CSS variables only

---

## Persistence

| Key | Value | When written |
|---|---|---|
| `soul-player:sidebar-width` | current width | On resize; **not written while collapsed** |
| `soul-player:sidebar-saved-width` | last non-zero width before collapse | On snap-to-zero |
| `soul-player:sidebar-collapsed` | `"true"` / `"false"` | On collapse and on expand |

**Conflict avoidance:** While `isCollapsed=true`, `handleMouseUp` does not persist `sidebar-width`, so the saved-width key is not clobbered.

---

## i18n Keys

Add to `applications/shared/src/i18n/en-US.json`, `de.json`, `ja.json`.

The existing `nowPlaying` namespace in `en-US.json` holds context-playback strings ("Playing from Album" etc.). The new floating bar strings go into a separate `floatingPlayer` namespace to avoid collision:

```json
{
  "sidebar": {
    "expand": "Expand sidebar"
  },
  "floatingPlayer": {
    "title": "Now Playing",
    "noTrack": "No track playing"
  }
}
```

---

## Edge Cases

| Scenario | Behavior |
|---|---|
| No track playing, sidebar collapsed | `NowPlayingFloating` does not render (`currentTrack === null` guard) |
| Track starts while collapsed | Modal appears automatically (reactive to Zustand) |
| Very narrow viewport | `w-[90vw]` prevents overflow |
| Reload while collapsed | `sidebar-collapsed=true` in localStorage → restores collapsed state on mount |
| Drag strip right then back left | `onExpand()` is one-shot on crossing 40px delta — strip unmounts after expand, no re-collapse |
| Marketing / web platform | `MockBackendProvider` → no current track → floating bar never renders |

---

## Tests

### Unit: `useCollapsibleSidebar.test.ts`

Location: `applications/shared/src/hooks/__tests__/useCollapsibleSidebar.test.ts`

- Snap triggers at exactly 200px threshold (and below)
- No snap when drag stays at or above 200px
- `expand()` restores `savedWidth`
- `isCollapsed` initialises `true` from localStorage `sidebar-collapsed=true`
- `savedWidth` initialises from localStorage `sidebar-saved-width`
- `handleMouseDown` sets `isResizing=true`
- Collapsing does **not** write `soul-player:sidebar-width`
- `expand()` writes `sidebar-collapsed=false` and `sidebar-width=restoredWidth`

### E2E: `collapsed-sidebar.spec.js`

Location: `applications/desktop/e2e-tests/tests/playwright/collapsed-sidebar.spec.js`

Uses `page.mouse` for drag simulation (same pattern as `seek-scrub.spec.js`). All drag coordinates are computed from `getBoundingClientRect()` — not hard-coded absolute values. The existing resize handle gets `data-testid="sidebar-resize-handle"` added to it.

1. Get handle rect via `sidebar-resize-handle` testid. Drag from handle center leftward (to center-x minus 250px). Assert `collapsed-sidebar-strip` appears, sidebar nav content is gone.
2. Assert strip bounding box width ≤ 8px.
3. Start a track via `startPlayback()` helper (same pattern as `playback-controls.spec.js` — invokes `play_queue` IPC directly). Assert `now-playing-floating` appears.
4. Assert `floating-now-playing-title` text matches seeded Track One title.
5. Assert `floating-progress-bar` is visible; wait 2s; assert `currentTime` has advanced (via `getPlaybackState()`).
6. Click play/pause button inside `now-playing-floating`. Assert playback state toggles.
7. Get strip rect via `collapsed-sidebar-strip`. Drag from strip center rightward by 80px. Assert strip disappears, sidebar nav reappears, `now-playing-floating` is absent.
8. Stop any track; confirm sidebar is collapsed. Assert `now-playing-floating` is absent from DOM.

---

## Out of Scope

- Animation/transition on collapse (can be added later)
- Volume control in floating bar (deferred — complex debounce/mute logic stays in `PlayerPanel` for now)
- Keyboard shortcut to collapse/expand sidebar

---

## Files Touched

| File | Change |
|---|---|
| `applications/shared/src/hooks/useCollapsibleSidebar.ts` | **New** |
| `applications/shared/src/hooks/usePlaybackHandlers.ts` | **New** |
| `applications/shared/src/hooks/__tests__/useCollapsibleSidebar.test.ts` | **New** |
| `applications/shared/src/contexts/SidebarStateContext.tsx` | **New** (includes `SidebarStateProvider`) |
| `applications/shared/src/components/CollapsedSidebarStrip.tsx` | **New** |
| `applications/shared/src/components/NowPlayingFloating.tsx` | **New** |
| `applications/shared/src/components/LeftSidebar.tsx` | Modified — consume context, conditional render, remove hook call |
| `applications/shared/src/components/sidebar/PlayerPanel.tsx` | Modified — consume `usePlaybackHandlers()`, remove `shuffleMode`/`repeatMode`/`onShuffleModeChange`/`onRepeatModeChange` props |
| `applications/shared/src/layouts/MainLayout.tsx` | Modified — wrap in `SidebarStateProvider`, render `NowPlayingFloating` |
| `applications/shared/src/i18n/en-US.json` | Modified — add `sidebar.expand`, `floatingPlayer.*` keys |
| `applications/shared/src/i18n/de.json` | Modified — same |
| `applications/shared/src/i18n/ja.json` | Modified — same |
| `applications/desktop/e2e-tests/tests/playwright/collapsed-sidebar.spec.js` | **New** |
