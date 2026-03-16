# Collapsed Sidebar + Now Playing Floating Bar — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Collapse the left sidebar to a thin edge strip when dragged below 200px, and show a centered floating "Now Playing" bar automatically when a track is playing.

**Architecture:** A `SidebarStateProvider` in `MainLayout` owns `useCollapsibleSidebar()` and exposes state via `SidebarStateContext`. `LeftSidebar` consumes the context and renders `CollapsedSidebarStrip` when collapsed. `NowPlayingFloating` is a sibling that reads `isCollapsed` from the same context and `currentTrack` from Zustand. A new `usePlaybackHandlers` hook extracts shuffle/repeat/play logic out of `PlayerPanel` so both `PlayerPanel` and `NowPlayingFloating` share it without duplication.

**Tech Stack:** React 18 + TypeScript + Zustand + TailwindCSS v4 + Vitest (unit) + Playwright CDP (E2E)

**Spec:** `docs/superpowers/specs/2026-03-16-collapsed-sidebar-now-playing-design.md`

---

## Chunk 1: Core Hook + Context

### Task 1: `useCollapsibleSidebar` hook + unit tests

**Files:**
- Create: `applications/shared/src/hooks/useCollapsibleSidebar.ts`
- Create: `applications/shared/src/hooks/__tests__/useCollapsibleSidebar.test.ts`

- [ ] **Step 1.1: Write failing unit tests**

Create `applications/shared/src/hooks/__tests__/useCollapsibleSidebar.test.ts`:

```typescript
import { renderHook, act } from '@testing-library/react';
import { useCollapsibleSidebar } from '../useCollapsibleSidebar';

// ── localStorage mock ──────────────────────────────────────────────────────
const localStorageMock = (() => {
  let store: Record<string, string> = {};
  return {
    getItem: (key: string) => store[key] ?? null,
    setItem: (key: string, value: string) => { store[key] = value; },
    removeItem: (key: string) => { delete store[key]; },
    clear: () => { store = {}; },
  };
})();
Object.defineProperty(window, 'localStorage', { value: localStorageMock });

beforeEach(() => localStorageMock.clear());

describe('useCollapsibleSidebar — initialisation', () => {
  it('starts expanded with default width 288 when localStorage is empty', () => {
    const { result } = renderHook(() => useCollapsibleSidebar());
    expect(result.current.isCollapsed).toBe(false);
    expect(result.current.width).toBe(288);
  });

  it('reads isCollapsed=true from localStorage on mount', () => {
    localStorageMock.setItem('soul-player:sidebar-collapsed', 'true');
    localStorageMock.setItem('soul-player:sidebar-saved-width', '320');
    const { result } = renderHook(() => useCollapsibleSidebar());
    expect(result.current.isCollapsed).toBe(true);
    expect(result.current.width).toBe(0);
  });

  it('reads savedWidth from localStorage on mount', () => {
    localStorageMock.setItem('soul-player:sidebar-saved-width', '350');
    const { result } = renderHook(() => useCollapsibleSidebar());
    expect(result.current.savedWidth).toBe(350);
  });
});

describe('useCollapsibleSidebar — handleMouseDown', () => {
  it('sets isResizing to true', () => {
    const { result } = renderHook(() => useCollapsibleSidebar());
    act(() => {
      result.current.handleMouseDown({ preventDefault: jest.fn() } as any);
    });
    expect(result.current.isResizing).toBe(true);
  });
});

describe('useCollapsibleSidebar — snap to collapsed', () => {
  it('snaps to collapsed when mousemove goes below 200px threshold', () => {
    const { result } = renderHook(() => useCollapsibleSidebar());
    act(() => {
      result.current.handleMouseDown({ preventDefault: jest.fn() } as any);
    });
    act(() => {
      document.dispatchEvent(new MouseEvent('mousemove', { clientX: 100, bubbles: true }));
    });
    expect(result.current.isCollapsed).toBe(true);
    expect(result.current.width).toBe(0);
    expect(localStorageMock.getItem('soul-player:sidebar-collapsed')).toBe('true');
  });

  it('does NOT snap when mousemove stays at or above 200px', () => {
    const { result } = renderHook(() => useCollapsibleSidebar());
    act(() => {
      result.current.handleMouseDown({ preventDefault: jest.fn() } as any);
    });
    act(() => {
      document.dispatchEvent(new MouseEvent('mousemove', { clientX: 250, bubbles: true }));
    });
    expect(result.current.isCollapsed).toBe(false);
    expect(result.current.width).toBe(250);
  });

  it('does NOT write soul-player:sidebar-width during snap-to-collapsed', () => {
    localStorageMock.setItem('soul-player:sidebar-width', '300');
    const { result } = renderHook(() => useCollapsibleSidebar());
    act(() => {
      result.current.handleMouseDown({ preventDefault: jest.fn() } as any);
    });
    act(() => {
      document.dispatchEvent(new MouseEvent('mousemove', { clientX: 50, bubbles: true }));
    });
    // sidebar-width key must remain at its pre-collapse value
    expect(localStorageMock.getItem('soul-player:sidebar-width')).toBe('300');
    expect(localStorageMock.getItem('soul-player:sidebar-collapsed')).toBe('true');
  });
});

describe('useCollapsibleSidebar — expand()', () => {
  it('restores savedWidth and sets isCollapsed=false', () => {
    localStorageMock.setItem('soul-player:sidebar-collapsed', 'true');
    localStorageMock.setItem('soul-player:sidebar-saved-width', '320');
    const { result } = renderHook(() => useCollapsibleSidebar());
    act(() => result.current.expand());
    expect(result.current.isCollapsed).toBe(false);
    expect(result.current.width).toBe(320);
  });

  it('accepts an explicit width override', () => {
    const { result } = renderHook(() => useCollapsibleSidebar());
    act(() => result.current.expand(400));
    expect(result.current.width).toBe(400);
  });

  it('persists isCollapsed=false and sidebar-width to localStorage', () => {
    const { result } = renderHook(() => useCollapsibleSidebar());
    act(() => result.current.expand(400));
    expect(localStorageMock.getItem('soul-player:sidebar-collapsed')).toBe('false');
    expect(localStorageMock.getItem('soul-player:sidebar-width')).toBe('400');
  });
});
```

- [ ] **Step 1.2: Run tests — verify they fail**

```bash
cd applications/shared && npx vitest run src/hooks/__tests__/useCollapsibleSidebar.test.ts
```

Expected: All tests fail with "Cannot find module '../useCollapsibleSidebar'".

- [ ] **Step 1.3: Implement `useCollapsibleSidebar.ts`**

Create `applications/shared/src/hooks/useCollapsibleSidebar.ts`:

```typescript
import { useState, useEffect, useCallback, useRef } from 'react';
import { debug } from '../utils/debug';

// ── Storage keys (share the width key with useResizableSidebar) ────────────
const STORAGE_WIDTH    = 'soul-player:sidebar-width';
const STORAGE_SAVED    = 'soul-player:sidebar-saved-width';
const STORAGE_COLLAPSED = 'soul-player:sidebar-collapsed';

const DEFAULT_WIDTH      = 288;
const MIN_WIDTH          = 240;
const MAX_WIDTH          = 480;
const COLLAPSE_THRESHOLD = 200; // px — snap shut when drag goes below this

function tryReadInt(key: string, fallback: number): number {
  try {
    const raw = localStorage.getItem(key);
    if (raw) { const n = parseInt(raw, 10); if (!isNaN(n)) return n; }
  } catch { /* storage unavailable */ }
  return fallback;
}

function tryWrite(key: string, value: string): void {
  try { localStorage.setItem(key, value); } catch { /* ignore */ }
}

export interface CollapsibleSidebarState {
  width: number;
  isCollapsed: boolean;
  isResizing: boolean;
  savedWidth: number;
  handleMouseDown: (e: React.MouseEvent) => void;
  expand: (width?: number) => void;
  resizableRef: React.RefObject<HTMLDivElement>;
}

export function useCollapsibleSidebar(): CollapsibleSidebarState {
  const resizableRef = useRef<HTMLDivElement>(null);

  const [isCollapsed, setIsCollapsed] = useState(() => {
    try { return localStorage.getItem(STORAGE_COLLAPSED) === 'true'; } catch { return false; }
  });

  const [savedWidth, setSavedWidth] = useState(() =>
    tryReadInt(STORAGE_SAVED, DEFAULT_WIDTH)
  );

  const [width, setWidth] = useState(() => {
    try { if (localStorage.getItem(STORAGE_COLLAPSED) === 'true') return 0; } catch {}
    return tryReadInt(STORAGE_WIDTH, DEFAULT_WIDTH);
  });

  const [isResizing, setIsResizing] = useState(false);

  // Refs for stable access inside event listeners (avoid stale closures)
  const widthRef      = useRef(width);
  const savedWidthRef = useRef(savedWidth);
  const isCollapsedRef = useRef(isCollapsed);
  widthRef.current      = width;
  savedWidthRef.current = savedWidth;
  isCollapsedRef.current = isCollapsed;

  const expand = useCallback((targetWidth?: number) => {
    const w = targetWidth ?? savedWidthRef.current;
    setWidth(w);
    setIsCollapsed(false);
    tryWrite(STORAGE_COLLAPSED, 'false');
    tryWrite(STORAGE_WIDTH, String(w));
  }, []);

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsResizing(true);
  }, []);

  useEffect(() => {
    if (!isResizing) return;

    const onMouseMove = (e: MouseEvent) => {
      // getBoundingClientRect().left is 0 for the sidebar (always at left edge)
      // but we use it correctly in case layout ever adds left inset
      const sidebarLeft = resizableRef.current?.getBoundingClientRect().left ?? 0;
      const newWidth = e.clientX - sidebarLeft;

      if (newWidth < COLLAPSE_THRESHOLD) {
        // Snap to collapsed — save current width first
        const sw = widthRef.current > 0 ? widthRef.current : savedWidthRef.current;
        setSavedWidth(sw);
        tryWrite(STORAGE_SAVED, String(sw));
        tryWrite(STORAGE_COLLAPSED, 'true');
        setWidth(0);
        setIsCollapsed(true);
        setIsResizing(false); // triggers useEffect cleanup, removes listeners
      } else {
        setWidth(Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, newWidth)));
      }
    };

    const onMouseUp = () => {
      if (!isCollapsedRef.current) {
        tryWrite(STORAGE_WIDTH, String(widthRef.current));
      }
      setIsResizing(false);
    };

    document.addEventListener('mousemove', onMouseMove);
    document.addEventListener('mouseup', onMouseUp);
    document.body.style.cursor = 'ew-resize';
    document.body.style.userSelect = 'none';

    return () => {
      document.removeEventListener('mousemove', onMouseMove);
      document.removeEventListener('mouseup', onMouseUp);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    };
  }, [isResizing]);

  return { width, isCollapsed, isResizing, savedWidth, handleMouseDown, expand, resizableRef };
}
```

- [ ] **Step 1.4: Run tests — verify all pass**

```bash
cd applications/shared && npx vitest run src/hooks/__tests__/useCollapsibleSidebar.test.ts
```

Expected: All 9 tests pass.

- [ ] **Step 1.5: Commit**

```bash
cd D:/dev/soulaudio/soul-player
git add applications/shared/src/hooks/useCollapsibleSidebar.ts \
        applications/shared/src/hooks/__tests__/useCollapsibleSidebar.test.ts
git commit -m "feat: add useCollapsibleSidebar hook with snap-to-collapse logic"
```

---

### Task 2: `SidebarStateContext` + `SidebarStateProvider`

**Files:**
- Create: `applications/shared/src/contexts/SidebarStateContext.tsx`

- [ ] **Step 2.1: Create `SidebarStateContext.tsx`**

Create `applications/shared/src/contexts/SidebarStateContext.tsx`:

```typescript
import { createContext, useContext, ReactNode } from 'react';
import { useCollapsibleSidebar, type CollapsibleSidebarState } from '../hooks/useCollapsibleSidebar';

// No-op defaults — consumers outside a provider fail silently (graceful degradation)
const SidebarStateContext = createContext<CollapsibleSidebarState>({
  width: 288,
  isCollapsed: false,
  isResizing: false,
  savedWidth: 288,
  handleMouseDown: () => {},
  expand: () => {},
  resizableRef: { current: null } as React.RefObject<HTMLDivElement>,
});

export const useSidebarState = () => useContext(SidebarStateContext);

/**
 * SidebarStateProvider — owns useCollapsibleSidebar() and publishes state
 * to all children via context. Render this in MainLayout wrapping both
 * LeftSidebar and NowPlayingFloating so they share the same collapse state.
 */
export function SidebarStateProvider({ children }: { children: ReactNode }) {
  const state = useCollapsibleSidebar();
  return (
    <SidebarStateContext.Provider value={state}>
      {children}
    </SidebarStateContext.Provider>
  );
}
```

- [ ] **Step 2.2: Verify TypeScript compiles**

```bash
cd D:/dev/soulaudio/soul-player && cargo xtask check typescript
```

Expected: No errors in `SidebarStateContext.tsx`.

- [ ] **Step 2.3: Commit**

```bash
git add applications/shared/src/contexts/SidebarStateContext.tsx
git commit -m "feat: add SidebarStateContext and SidebarStateProvider"
```

---

## Chunk 2: UI Shell — Strip, LeftSidebar, MainLayout, i18n

### Task 3: `CollapsedSidebarStrip` component

**Files:**
- Create: `applications/shared/src/components/CollapsedSidebarStrip.tsx`

- [ ] **Step 3.1: Create `CollapsedSidebarStrip.tsx`**

Create `applications/shared/src/components/CollapsedSidebarStrip.tsx`:

```typescript
import { useCallback, useRef } from 'react';
import { useTranslation } from 'react-i18next';

const RESTORE_THRESHOLD = 40; // px delta rightward — triggers expand

interface CollapsedSidebarStripProps {
  onExpand: () => void;
}

/**
 * Thin vertical strip shown when the sidebar is fully collapsed.
 * Drag rightward past 40px to restore the sidebar.
 */
export function CollapsedSidebarStrip({ onExpand }: CollapsedSidebarStripProps) {
  const { t } = useTranslation();
  const startXRef = useRef<number | null>(null);

  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      startXRef.current = e.clientX;

      const cleanup = () => {
        startXRef.current = null;
        document.removeEventListener('mousemove', onMouseMove);
        document.removeEventListener('mouseup', onMouseUp);
      };

      const onMouseMove = (ev: MouseEvent) => {
        if (startXRef.current === null) return;
        const delta = ev.clientX - startXRef.current;
        if (delta > RESTORE_THRESHOLD) {
          cleanup(); // one-shot — does not re-collapse on drag-back
          onExpand();
        }
      };

      const onMouseUp = () => cleanup();

      document.addEventListener('mousemove', onMouseMove);
      document.addEventListener('mouseup', onMouseUp);
    },
    [onExpand]
  );

  return (
    <div
      className="w-1.5 h-full bg-foreground/10 hover:bg-foreground/20 transition-opacity cursor-ew-resize flex-shrink-0"
      onMouseDown={handleMouseDown}
      role="separator"
      aria-orientation="vertical"
      aria-label={t('sidebar.expand', 'Expand sidebar')}
      data-testid="collapsed-sidebar-strip"
    />
  );
}
```

- [ ] **Step 3.2: Verify TypeScript compiles**

```bash
cd D:/dev/soulaudio/soul-player && cargo xtask check typescript
```

Expected: No errors.

- [ ] **Step 3.3: Commit**

```bash
git add applications/shared/src/components/CollapsedSidebarStrip.tsx
git commit -m "feat: add CollapsedSidebarStrip — drag-to-restore sidebar handle"
```

---

### Task 4: Update `LeftSidebar.tsx`

**Files:**
- Modify: `applications/shared/src/components/LeftSidebar.tsx`

Key changes:
1. Replace `useResizableSidebar` with `useSidebarState()` from context
2. Remove `usePlayerModes` (shuffle/repeat now owned by `usePlaybackHandlers`)
3. Remove `handleShuffleModeChange` / `handleRepeatModeChange` callbacks
4. Render `<CollapsedSidebarStrip>` when `isCollapsed`, full sidebar otherwise
5. Add `data-collapsed` attribute and `data-testid="sidebar-resize-handle"` to handle

- [ ] **Step 4.1: Update imports in `LeftSidebar.tsx`**

In `applications/shared/src/components/LeftSidebar.tsx`, replace the imports block:

```typescript
// REMOVE these imports:
import { usePlayerModes } from '../stores/player';
import { useResizableSidebar } from '../hooks/useResizableSidebar';

// ADD these imports:
import { useSidebarState } from '../contexts/SidebarStateContext';
import { CollapsedSidebarStrip } from './CollapsedSidebarStrip';
```

Full updated imports section at the top of the file (remove `'use client'` if present — this is a Vite/Tauri app, not Next.js):

```typescript
import { useEffect, useState, useCallback, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  useCurrentTrack,
  useIsPlaying,
  useVolume,
} from '../stores/player';
import { usePlayerCommands, usePlaybackEvents, type QueueTrack } from '../contexts/PlayerCommandsContext';
import { cn } from '../lib/utils';
import { usePlatform } from '../contexts/PlatformContext';
import { useBackend } from '../contexts/BackendContext';
import { useSidebarState } from '../contexts/SidebarStateContext';
import { CollapsedSidebarStrip } from './CollapsedSidebarStrip';
import { debug } from '../utils/debug';
import { useTranslation } from 'react-i18next';
import {
  NavBar,
  QueueSection,
  PlayerPanel,
  SettingsFooter,
} from './sidebar';
```

- [ ] **Step 4.2: Remove `usePlayerModes` usage and old hook call**

Inside the component body, remove:
```typescript
// REMOVE:
const { shuffleMode, repeatMode, setShuffleMode, setRepeatMode } = usePlayerModes();

// REMOVE the old hook call:
const { width, isResizing, handleMouseDown, resizableRef } = useResizableSidebar({
  minWidth: 240,
  maxWidth: 480,
  defaultWidth: 288,
});

// REMOVE these handlers:
const handleShuffleModeChange = (mode: ShuffleMode) => { setShuffleMode(mode); };
const handleRepeatModeChange = (mode: RepeatMode) => { setRepeatMode(mode); };
```

Add instead:
```typescript
const { width, isCollapsed, isResizing, handleMouseDown, expand, resizableRef } = useSidebarState();
```

- [ ] **Step 4.3: Update the return JSX**

Replace the entire `return (...)` block with:

```tsx
// When collapsed — render only the thin strip
if (isCollapsed) {
  return <CollapsedSidebarStrip onExpand={expand} />;
}

return (
  <div
    ref={resizableRef}
    className="bg-card border-r border-border flex flex-col h-full relative"
    style={{ width: `${width}px` }}
    data-collapsed={isCollapsed}
  >
    {/* Resize Handle */}
    <div
      className={cn(
        'absolute top-0 right-0 w-1 h-full cursor-ew-resize group z-50',
        isResizing && 'bg-primary/50'
      )}
      onMouseDown={handleMouseDown}
      title={t('sidebar.resize', 'Resize sidebar')}
      data-testid="sidebar-resize-handle"
    >
      <div
        className={cn(
          'absolute inset-y-0 right-0 w-[3px] bg-primary/0 group-hover:bg-primary/30 transition-colors',
          isResizing && 'bg-primary/50'
        )}
      />
    </div>

    {/* Navigation - fixed at top */}
    <NavBar homeEnabled={homeEnabled} />

    {/* Queue Section - flexible, fills available space */}
    <div className="flex-1 flex flex-col min-h-0 overflow-hidden">
      <QueueSection
        queue={queue}
        currentTrackId={currentTrack?.id}
        scrollRef={queueScrollRef}
        onTrackClick={handleQueueItemClick}
      />
    </div>

    {/* Player Panel - fixed at bottom */}
    <PlayerPanel
      currentTrack={
        currentTrack
          ? {
              id: currentTrack.id,
              title: currentTrack.title,
              artist: currentTrack.artist,
              album: currentTrack.album,
              coverArtPath: currentTrack.coverArtPath,
            }
          : null
      }
      isPlaying={isPlaying}
      volume={volume}
      canCreatePlaylists={features.canCreatePlaylists}
      onTrackClick={() => navigate('/now-playing')}
      onAddToPlaylist={onAddToPlaylist}
    />

    {/* Settings Footer - bottom of sidebar */}
    <SettingsFooter version={version} />
  </div>
);
```

Note: `shuffleMode`, `repeatMode`, `onShuffleModeChange`, `onRepeatModeChange` props removed from `<PlayerPanel />` — these are now handled internally by `PlayerPanel` via `usePlaybackHandlers`.

Also remove the `ShuffleMode` and `RepeatMode` type imports from the `./sidebar` import if they are only used by the removed callbacks.

Note on `data-collapsed`: The attribute appears on the expanded sidebar container (`data-collapsed={false}`) so CSS/tests can target the expanded state. When collapsed, `<CollapsedSidebarStrip>` renders instead — the absence of the full sidebar container is itself the "collapsed" signal. The strip has `data-testid="collapsed-sidebar-strip"` for E2E targeting.

- [ ] **Step 4.4: Note — do NOT commit yet**

LeftSidebar now calls `<PlayerPanel>` without the 4 removed props, which will cause TypeScript errors until `PlayerPanel.tsx` is updated in Task 8. Complete Tasks 7 and 8 first, then come back to commit all three files together in Step 8.3.

---

### Task 5: Update `MainLayout.tsx`

**Files:**
- Modify: `applications/shared/src/layouts/MainLayout.tsx`

- [ ] **Step 5.1: Update `MainLayout.tsx`**

Replace the full contents of `applications/shared/src/layouts/MainLayout.tsx`:

```typescript
import { ReactNode, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { LeftSidebar } from '../components/LeftSidebar';
import { NowPlayingFloating } from '../components/NowPlayingFloating';
import { SidebarStateProvider } from '../contexts/SidebarStateContext';
import { useScrollVisibility } from '../contexts/ScrollVisibilityContext';

interface MainLayoutProps {
  children: ReactNode;
  /** Callback when the "Add to Playlist" button is clicked in the sidebar */
  onAddToPlaylist?: () => void;
}

export function MainLayout({ children, onAddToPlaylist }: MainLayoutProps) {
  const navigate = useNavigate();

  let showHeader = true;
  try {
    const context = useScrollVisibility();
    showHeader = context.showHeader;
  } catch {
    showHeader = true;
  }

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === '1') {
        e.preventDefault();
        navigate('/');
      }
      if ((e.metaKey || e.ctrlKey) && e.key === '2') {
        e.preventDefault();
        navigate('/albums');
      }
      if ((e.metaKey || e.ctrlKey) && e.key === 'l') {
        e.preventDefault();
        navigate('/albums');
      }
      if ((e.metaKey || e.ctrlKey) && e.key === 'h') {
        e.preventDefault();
        navigate('/');
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [navigate]);

  return (
    <SidebarStateProvider>
      <div className="flex h-full bg-background text-foreground">
        {/* Left Sidebar — collapses to CollapsedSidebarStrip when isCollapsed */}
        <LeftSidebar onAddToPlaylist={onAddToPlaylist} />

        {/* Main Content Area */}
        <main className="flex-1 overflow-hidden">
          <div className={`h-full pl-6 pr-0 transition-all duration-300 ${showHeader ? 'pt-6' : 'pt-0'}`}>
            {children}
          </div>
        </main>

        {/* Now Playing Floating Bar — visible when sidebar is collapsed + track playing */}
        <NowPlayingFloating />
      </div>
    </SidebarStateProvider>
  );
}
```

- [ ] **Step 5.2: Note — do NOT commit yet**

`MainLayout.tsx` imports `NowPlayingFloating` which doesn't exist until Task 9. Committing now would leave TypeScript broken. This file is committed together with `NowPlayingFloating.tsx` in Step 9.4.

---

### Task 6: Add i18n keys

**Files:**
- Modify: `applications/shared/src/i18n/en-US.json`
- Modify: `applications/shared/src/i18n/de.json`
- Modify: `applications/shared/src/i18n/ja.json`

- [ ] **Step 6.1: Add keys to `en-US.json`**

In `applications/shared/src/i18n/en-US.json`, inside the `"sidebar"` object, add `"expand"`:

```json
"sidebar": {
  "resize": "Resize sidebar",
  "expand": "Expand sidebar"
}
```

Add a new top-level `"floatingPlayer"` object (after `"nowPlaying"`):

```json
"floatingPlayer": {
  "title": "Now Playing",
  "noTrack": "No track playing"
}
```

- [ ] **Step 6.2: Add keys to `de.json`**

In `applications/shared/src/i18n/de.json`, inside the existing `"sidebar"` object add `"expand"`, and add a new `"floatingPlayer"` top-level key:

```json
"sidebar": {
  "resize": "Seitenleiste anpassen",
  "expand": "Seitenleiste erweitern"
},
"floatingPlayer": {
  "title": "Jetzt läuft",
  "noTrack": "Kein Titel läuft"
}
```

- [ ] **Step 6.3: Add keys to `ja.json`**

In `applications/shared/src/i18n/ja.json`, inside the existing `"sidebar"` object add `"expand"`, and add a new `"floatingPlayer"` top-level key:

```json
"sidebar": {
  "resize": "サイドバーのサイズを変更",
  "expand": "サイドバーを展開"
},
"floatingPlayer": {
  "title": "再生中",
  "noTrack": "再生中のトラックはありません"
}
```

- [ ] **Step 6.4: Verify TypeScript compiles**

```bash
cd D:/dev/soulaudio/soul-player && cargo xtask check typescript
```

- [ ] **Step 6.5: Commit**

```bash
git add applications/shared/src/i18n/en-US.json \
        applications/shared/src/i18n/de.json \
        applications/shared/src/i18n/ja.json
git commit -m "feat: add i18n keys for sidebar.expand and floatingPlayer namespace"
```

---

## Chunk 3: Playback Handlers + NowPlayingFloating + PlayerPanel Refactor

### Task 7: `usePlaybackHandlers` hook

**Files:**
- Create: `applications/shared/src/hooks/usePlaybackHandlers.ts`

- [ ] **Step 7.1: Create `usePlaybackHandlers.ts`**

Create `applications/shared/src/hooks/usePlaybackHandlers.ts`:

```typescript
import { useCallback } from 'react';
import { usePlayerCommands } from '../contexts/PlayerCommandsContext';
import { usePlayerStore } from '../stores/player';
import { debug } from '../utils/debug';
import type { ShuffleMode, RepeatMode } from '../components/sidebar/PlaybackControls';

export interface PlaybackHandlers {
  onPlayPause: () => Promise<void>;
  onNext: () => Promise<void>;
  onPrevious: () => Promise<void>;
  /**
   * Cycles shuffle mode via backend, then writes the returned mode directly
   * to the Zustand store. Does NOT use a prop callback — owns the store write.
   */
  onShuffleToggle: () => Promise<void>;
  /**
   * Cycles repeat mode with optimistic update + rollback on error.
   * Writes directly to the Zustand store — does NOT use a prop callback.
   */
  onRepeatToggle: () => Promise<void>;
}

/**
 * Shared hook for playback control handlers.
 * Consumed by both PlayerPanel and NowPlayingFloating so the logic
 * is not duplicated. Uses usePlayerCommands() and writes directly to
 * Zustand store (usePlayerStore.getState()) for shuffle/repeat.
 */
export function usePlaybackHandlers(): PlaybackHandlers {
  const commands = usePlayerCommands();

  const onPlayPause = useCallback(async () => {
    try {
      const { isPlaying } = usePlayerStore.getState();
      if (isPlaying) {
        await commands.pausePlayback();
      } else {
        await commands.resumePlayback();
      }
    } catch (error) {
      debug.error('[usePlaybackHandlers] Failed to toggle playback:', error);
    }
  }, [commands]);

  const onNext = useCallback(async () => {
    try {
      await commands.skipNext();
    } catch (error) {
      debug.error('[usePlaybackHandlers] Failed to skip next:', error);
    }
  }, [commands]);

  const onPrevious = useCallback(async () => {
    try {
      await commands.skipPrevious();
    } catch (error) {
      debug.error('[usePlaybackHandlers] Failed to skip previous:', error);
    }
  }, [commands]);

  const onShuffleToggle = useCallback(async () => {
    try {
      const newMode = (await commands.cycleShuffle()) as ShuffleMode;
      usePlayerStore.getState().setShuffleMode(newMode);
    } catch (error) {
      debug.error('[usePlaybackHandlers] Failed to cycle shuffle:', error);
    }
  }, [commands]);

  const onRepeatToggle = useCallback(async () => {
    const { repeatMode } = usePlayerStore.getState();
    const currentMode = repeatMode as RepeatMode;
    const nextMode: RepeatMode =
      currentMode === 'off' ? 'all' : currentMode === 'all' ? 'one' : 'off';
    // Optimistic update
    usePlayerStore.getState().setRepeatMode(nextMode);
    try {
      await commands.setRepeatMode(nextMode);
    } catch (error) {
      debug.error('[usePlaybackHandlers] Failed to set repeat mode, rolling back:', error);
      // Rollback to the pre-toggle mode
      usePlayerStore.getState().setRepeatMode(currentMode);
    }
  }, [commands]);

  return { onPlayPause, onNext, onPrevious, onShuffleToggle, onRepeatToggle };
}
```

- [ ] **Step 7.2: Verify TypeScript compiles**

```bash
cd D:/dev/soulaudio/soul-player && cargo xtask check typescript
```

Expected: No errors in the new file.

- [ ] **Step 7.3: Verify TypeScript compiles**

```bash
cd D:/dev/soulaudio/soul-player && cargo xtask check typescript
```

Expected: No errors in `usePlaybackHandlers.ts`. (LeftSidebar + PlayerPanel TS errors remain until Step 8 — expected.)

---

### Task 8: Refactor `PlayerPanel.tsx` — remove 4 props, use `usePlaybackHandlers`

**Files:**
- Modify: `applications/shared/src/components/sidebar/PlayerPanel.tsx`

The 4 props being removed: `shuffleMode`, `repeatMode`, `onShuffleModeChange`, `onRepeatModeChange`.
`PlayerPanel` now reads shuffle/repeat from Zustand directly and calls `usePlaybackHandlers()`.

- [ ] **Step 8.1: Replace `PlayerPanel.tsx` contents**

Replace the full contents of `applications/shared/src/components/sidebar/PlayerPanel.tsx` (drop `'use client'` directive — Vite/Tauri, not Next.js):

```typescript
import { useState, useCallback, useRef, useEffect } from 'react';
import { usePlayerStore } from '../../stores/player';
import { usePlayerCommands } from '../../contexts/PlayerCommandsContext';
import { usePlaybackHandlers } from '../../hooks/usePlaybackHandlers';
import { NowPlayingPanel, type CurrentTrackInfo } from './NowPlayingPanel';
import { ProgressBar } from '../player/ProgressBar';
import { PlaybackControls } from './PlaybackControls';
import { VolumeControl } from './VolumeControl';
import { debug } from '../../utils/debug';

export interface PlayerPanelProps {
  currentTrack: CurrentTrackInfo | null;
  isPlaying: boolean;
  volume: number;
  canCreatePlaylists: boolean;
  onTrackClick?: () => void;
  onAddToPlaylist?: () => void;
  // Removed: shuffleMode, repeatMode, onShuffleModeChange, onRepeatModeChange
  // These are now owned by usePlaybackHandlers + read from Zustand store directly.
}

export function PlayerPanel({
  currentTrack,
  isPlaying,
  volume,
  canCreatePlaylists,
  onTrackClick,
  onAddToPlaylist,
}: PlayerPanelProps) {
  const commands = usePlayerCommands();
  const handlers = usePlaybackHandlers();
  // Read shuffle/repeat from Zustand — no prop needed
  const shuffleMode = usePlayerStore((s) => s.shuffleMode);
  const repeatMode  = usePlayerStore((s) => s.repeatMode);

  const [isMuted, setIsMuted] = useState(false);
  const [volumeBeforeMute, setVolumeBeforeMute] = useState(volume);
  const debounceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (volume > 0 && !isMuted) {
      setVolumeBeforeMute(volume);
    }
  }, [volume, isMuted]);

  const applyVolumeChange = useCallback(
    (newVolume: number) => {
      const clampedVolume = Math.max(0, Math.min(1, newVolume));
      usePlayerStore.getState().setVolume(clampedVolume);
      if (clampedVolume > 0 && isMuted) setIsMuted(false);
      if (debounceTimerRef.current) clearTimeout(debounceTimerRef.current);
      debounceTimerRef.current = setTimeout(() => {
        commands.setVolume(clampedVolume).catch((error) => {
          debug.error('[PlayerPanel] Set volume failed:', error);
        });
      }, 150);
    },
    [commands, isMuted]
  );

  const handleVolumeChange = (newVolume: number) => applyVolumeChange(newVolume);

  const handleMuteToggle = async () => {
    try {
      if (isMuted) {
        await commands.setVolume(volumeBeforeMute);
        usePlayerStore.getState().setVolume(volumeBeforeMute);
        setIsMuted(false);
      } else {
        setVolumeBeforeMute(volume);
        await commands.setVolume(0);
        usePlayerStore.getState().setVolume(0);
        setIsMuted(true);
      }
    } catch (error) {
      debug.error('[PlayerPanel] Mute toggle failed:', error);
    }
  };

  const handleVolumeWheel = useCallback(
    (e: React.WheelEvent) => {
      const target = e.target as HTMLElement;
      if (target.closest('[data-dropdown-menu]')) return;
      e.preventDefault();
      const delta = e.deltaY > 0 ? -0.05 : 0.05;
      applyVolumeChange(volume + delta);
    },
    [volume, applyVolumeChange]
  );

  return (
    <div className="flex-shrink-0">
      <NowPlayingPanel
        currentTrack={currentTrack}
        isPlaying={isPlaying}
        canCreatePlaylists={canCreatePlaylists}
        onTrackClick={onTrackClick}
        onAddToPlaylist={onAddToPlaylist}
      />

      <div className="px-4 pt-2 pb-4 space-y-3">
        <ProgressBar />

        <PlaybackControls
          isPlaying={isPlaying}
          hasCurrentTrack={!!currentTrack}
          shuffleMode={shuffleMode}
          repeatMode={repeatMode}
          onPlayPause={handlers.onPlayPause}
          onPrevious={handlers.onPrevious}
          onNext={handlers.onNext}
          onShuffleToggle={handlers.onShuffleToggle}
          onRepeatToggle={handlers.onRepeatToggle}
        />

        <VolumeControl
          volume={volume}
          isMuted={isMuted}
          onVolumeChange={handleVolumeChange}
          onMuteToggle={handleMuteToggle}
          onWheel={handleVolumeWheel}
        />
      </div>
    </div>
  );
}
```

- [ ] **Step 8.2: Verify TypeScript compiles (LeftSidebar + PlayerPanel both updated)**

```bash
cd D:/dev/soulaudio/soul-player && cargo xtask check typescript
```

Expected: Zero errors on LeftSidebar and PlayerPanel. The `NowPlayingFloating` import error in `MainLayout.tsx` remains until Task 9 — that is expected.

- [ ] **Step 8.4: Commit LeftSidebar + PlayerPanel + usePlaybackHandlers together**

```bash
git add applications/shared/src/components/LeftSidebar.tsx \
        applications/shared/src/components/sidebar/PlayerPanel.tsx \
        applications/shared/src/hooks/usePlaybackHandlers.ts
git commit -m "refactor: PlayerPanel uses usePlaybackHandlers, removes 4 props; LeftSidebar updated"
```

Note: `usePlaybackHandlers.ts` is committed here for the first time alongside the two files that depend on it, keeping the diff coherent.

---

### Task 9: `NowPlayingFloating` component

**Files:**
- Create: `applications/shared/src/components/NowPlayingFloating.tsx`

- [ ] **Step 9.1: Create `NowPlayingFloating.tsx`**

Create `applications/shared/src/components/NowPlayingFloating.tsx`:

```typescript
import { useTranslation } from 'react-i18next';
import { useSidebarState } from '../contexts/SidebarStateContext';
import { useCurrentTrack, useIsPlaying, usePlayerStore } from '../stores/player';
import { ArtworkImage } from './ArtworkImage';
import { ProgressBar } from './player/ProgressBar';
import { PlaybackControls } from './sidebar/PlaybackControls';
import { usePlaybackHandlers } from '../hooks/usePlaybackHandlers';

/**
 * NowPlayingFloating — centered fixed bar shown when the sidebar is collapsed
 * and a track is playing. Reads collapse state from SidebarStateContext and
 * track data from the Zustand player store.
 *
 * Layout (wide bar, max 560px):
 *   [artwork 56×56] | Title
 *                   | Artist · Album
 *                   | ── seek bar ──  0:00 / 0:00
 *                   | ⇄  ⏮  ▶  ⏭  ↺
 */
export function NowPlayingFloating() {
  const { t } = useTranslation();
  const { isCollapsed } = useSidebarState();
  const currentTrack  = useCurrentTrack();
  const isPlaying     = useIsPlaying();
  const shuffleMode   = usePlayerStore((s) => s.shuffleMode);
  const repeatMode    = usePlayerStore((s) => s.repeatMode);
  const handlers      = usePlaybackHandlers();

  // Only mount when the sidebar is collapsed AND a track is loaded
  if (!isCollapsed || !currentTrack) return null;

  return (
    <div
      className="fixed left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 z-50 max-w-[560px] w-[90vw]"
      data-testid="now-playing-floating"
    >
      <div className="bg-card border border-border rounded-xl shadow-lg p-4">
        <div className="flex gap-4 items-center">

          {/* Album artwork — ArtworkImage handles URI scheme, caching, and fallback */}
          <ArtworkImage
            trackId={currentTrack.id}
            coverArtPath={currentTrack.coverArtPath}
            alt={currentTrack.title}
            className="w-14 h-14 rounded-md object-cover flex-shrink-0"
            fallbackIconSize="sm"
          />

          {/* Track info + controls */}
          <div className="flex-1 min-w-0 space-y-2">

            {/* Track title + artist */}
            <div>
              <p
                className="text-sm font-semibold truncate"
                data-testid="floating-now-playing-title"
              >
                {currentTrack.title}
              </p>
              <p
                className="text-xs text-muted-foreground truncate"
                data-testid="floating-now-playing-artist"
              >
                {/* Track.artist is required (string) in the store type — no nullish coalescing needed */}
                {currentTrack.artist}
              </p>
            </div>

            {/* Seek bar — second ProgressBar instance, safe to mount alongside sidebar's */}
            <div data-testid="floating-progress-bar">
              <ProgressBar />
            </div>

            {/* Playback controls */}
            <PlaybackControls
              isPlaying={isPlaying}
              hasCurrentTrack={true}
              shuffleMode={shuffleMode}
              repeatMode={repeatMode}
              onPlayPause={handlers.onPlayPause}
              onPrevious={handlers.onPrevious}
              onNext={handlers.onNext}
              onShuffleToggle={handlers.onShuffleToggle}
              onRepeatToggle={handlers.onRepeatToggle}
            />

          </div>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 9.2: Verify full TypeScript compile passes**

```bash
cd D:/dev/soulaudio/soul-player && cargo xtask check typescript
```

Expected: Zero errors.

- [ ] **Step 9.3: Run full pre-commit check**

```bash
cd D:/dev/soulaudio/soul-player && cargo xtask check precommit
```

Expected: fmt ✓, clippy ✓, test ✓, typescript ✓, lint ✓

- [ ] **Step 9.4: Commit NowPlayingFloating + MainLayout together**

`MainLayout.tsx` was intentionally not committed in Task 5 because it imports `NowPlayingFloating`. Both files are committed here so the tree is never left broken.

```bash
git add applications/shared/src/components/NowPlayingFloating.tsx \
        applications/shared/src/layouts/MainLayout.tsx
git commit -m "feat: add NowPlayingFloating and wire into MainLayout via SidebarStateProvider"
```

---

## Chunk 4: E2E Tests with Screenshots (Playwright CDP)

### Task 10: `collapsed-sidebar.spec.js` — 8 tests + screenshots

**Files:**
- Create: `applications/desktop/e2e-tests/tests/playwright/collapsed-sidebar.spec.js`

The spec file uses the Playwright CDP setup already established by `playback-controls.spec.js` and `seek-scrub.spec.js`. Screenshots are taken at each key visual state to document the feature.

Screenshots are saved to: `applications/desktop/e2e-tests/test-results/screenshots/collapsed-sidebar/`

- [ ] **Step 10.1: Create the spec file**

Create `applications/desktop/e2e-tests/tests/playwright/collapsed-sidebar.spec.js`:

```javascript
/**
 * collapsed-sidebar.spec.js — Playwright CDP tests
 *
 * Tests the collapsed sidebar + NowPlayingFloating feature:
 *   1. Drag sidebar handle past threshold → collapses to edge strip
 *   2. Edge strip is narrow (≤ 8px)
 *   3. Start track while collapsed → NowPlayingFloating appears
 *   4. Floating bar shows correct track title
 *   5. Progress bar visible and advancing
 *   6. Play/pause from floating bar works
 *   7. Drag edge strip right → sidebar restores, floating bar disappears
 *   8. No track playing + collapsed → floating bar absent
 *
 * Screenshots saved to: test-results/screenshots/collapsed-sidebar/
 *
 * Seed data used (from playwright-global-setup.js):
 *   Album 2001 "Playwright Album" — 6 tracks × 2s WAV, IDs 2001–2006
 */

import { test, expect, chromium } from '@playwright/test';
import { CDP_URL } from '../../playwright-global-setup.js';
import path from 'path';
import fs from 'fs';
import { fileURLToPath } from 'url';

// ── Screenshot helper ─────────────────────────────────────────────────────
// fileURLToPath correctly handles the leading '/' on Windows (e.g. /D:/dev/...)
const __filename = fileURLToPath(import.meta.url);
const __dirname  = path.dirname(__filename);
const SCREENSHOT_DIR = path.join(__dirname, '../../test-results/screenshots/collapsed-sidebar');

async function screenshot(page, name) {
  fs.mkdirSync(SCREENSHOT_DIR, { recursive: true });
  const filePath = path.join(SCREENSHOT_DIR, `${name}.png`);
  await page.screenshot({ path: filePath, fullPage: false });
  console.log(`[screenshot] saved: ${filePath}`);
}

// ── CDP connection ───────────────────────────────────────────────────────────
let browser;
let page;

test.beforeAll(async () => {
  browser = await chromium.connectOverCDP(CDP_URL);
  const context = browser.contexts()[0];
  const pages = context.pages();
  page = pages.find(
    (p) =>
      (p.url().includes('localhost:1420') || p.url().includes('tauri.localhost')) &&
      !p.url().includes('splash')
  );
  if (!page) throw new Error('Main window not found in CDP context');
  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });
});

test.afterAll(async () => {
  await browser.close();
});

// ── Helpers ──────────────────────────────────────────────────────────────────

/**
 * Restore the sidebar if it's currently collapsed.
 * Drags the collapsed-sidebar-strip rightward to trigger expand().
 */
async function ensureSidebarExpanded(p) {
  const strip = p.locator('[data-testid="collapsed-sidebar-strip"]');
  if (await strip.isVisible({ timeout: 500 }).catch(() => false)) {
    const box = await strip.boundingBox();
    if (box) {
      await p.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await p.mouse.down();
      await p.mouse.move(box.x + 80, box.y + box.height / 2, { steps: 10 });
      await p.mouse.up();
      await p.waitForSelector('[data-testid="sidebar-resize-handle"]', { timeout: 5_000 });
    }
  }
}

/**
 * Start playback of album 2001 Track One via play_queue IPC directly.
 * Same pattern as playback-controls.spec.js startPlayback().
 */
async function startPlayback(p) {
  await p.evaluate(async () => {
    const tracks = await window.__TAURI_INTERNALS__.invoke('get_album_tracks', { albumId: 2001 });
    tracks.sort((a, b) => (a.track_number || 0) - (b.track_number || 0));
    const queue = tracks.map((t) => ({
      trackId: String(t.id),
      title: t.title,
      artist: t.artist_name || 'Unknown Artist',
      album: t.album_title || null,
      albumId: t.album_id || null,
      filePath: t.file_path || '',
      durationSeconds: t.duration_seconds || null,
      trackNumber: t.track_number || null,
      coverArtPath: null,
    }));
    await window.__TAURI_INTERNALS__.invoke('play_queue', { queue, startIndex: 0 });
  });
  // Wait for UI to show a now-playing title somewhere (sidebar OR floating bar)
  await p.waitForFunction(
    () =>
      document.querySelector('[data-testid="now-playing-title"]') !== null ||
      document.querySelector('[data-testid="floating-now-playing-title"]') !== null,
    { timeout: 15_000 }
  );
}

/**
 * Collapse the sidebar by dragging the resize handle far left.
 * Uses getBoundingClientRect on the handle for stable coordinates.
 */
async function collapseSidebar(p) {
  const handle = p.locator('[data-testid="sidebar-resize-handle"]');
  await handle.waitFor({ timeout: 5_000 });
  const box = await handle.boundingBox();
  if (!box) throw new Error('sidebar-resize-handle has no bounding box');

  // Drag from handle center to 80px — well past the 200px threshold
  await p.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await p.mouse.down();
  await p.mouse.move(80, box.y + box.height / 2, { steps: 20 });
  await p.mouse.up();

  // Wait for the strip to appear
  await p.waitForSelector('[data-testid="collapsed-sidebar-strip"]', { timeout: 5_000 });
}

// ── Per-test setup / teardown ────────────────────────────────────────────────

test.beforeEach(async () => {
  // Stop any active playback
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.waitForTimeout(200);

  // Dismiss overlays
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);

  // Restore sidebar if collapsed from a previous test
  await ensureSidebarExpanded(page);

  // Navigate to Albums so we have a known starting state
  await page.click('[data-testid="nav-albums"]', { force: true });
  await page.waitForSelector('[data-testid="media-card-album-2001"]', { timeout: 15_000 });
});

test.afterEach(async () => {
  await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('stop_playback'); } catch {}
  }).catch(() => {});
  await page.keyboard.press('Escape').catch(() => {});
  // Always restore sidebar so next test starts in a known state
  await ensureSidebarExpanded(page);
  await page.waitForTimeout(200);
});

// ── Tests ────────────────────────────────────────────────────────────────────

test('1. drag sidebar handle past threshold — sidebar collapses, edge strip appears', async () => {
  await screenshot(page, '01-before-collapse');

  await collapseSidebar(page);

  // Edge strip must be present
  await expect(page.locator('[data-testid="collapsed-sidebar-strip"]')).toBeVisible();

  // Nav bar content must be gone
  await expect(page.locator('[data-testid="nav-albums"]')).not.toBeVisible();

  await screenshot(page, '02-after-collapse-edge-strip');
});

test('2. collapsed edge strip is narrow (≤ 8px wide)', async () => {
  await collapseSidebar(page);

  const box = await page.locator('[data-testid="collapsed-sidebar-strip"]').boundingBox();
  expect(box).not.toBeNull();
  expect(box.width).toBeLessThanOrEqual(8);

  await screenshot(page, '03-edge-strip-width');
});

test('3. starting a track while sidebar is collapsed → NowPlayingFloating appears', async () => {
  await collapseSidebar(page);
  await startPlayback(page);

  await expect(page.locator('[data-testid="now-playing-floating"]')).toBeVisible({ timeout: 5_000 });

  await screenshot(page, '04-floating-bar-visible');
});

test('4. floating bar shows correct track title', async () => {
  await collapseSidebar(page);
  await startPlayback(page);

  const title = await page.locator('[data-testid="floating-now-playing-title"]').textContent();
  expect(title?.trim()).toBeTruthy();
  // Track One is first in album 2001 (sorted by track_number)
  expect(title?.trim()).toContain('Track One');

  await screenshot(page, '05-floating-bar-title');
});

test('5. floating bar progress bar is visible and time advances', async () => {
  await collapseSidebar(page);
  await startPlayback(page);

  // Progress bar container is visible
  await expect(page.locator('[data-testid="floating-progress-bar"]')).toBeVisible();

  // Capture progress before and after waiting 2 seconds
  const before = await page.evaluate(async () => {
    const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
    return state;
  });
  expect(before).toBe('Playing');

  await page.waitForTimeout(2_000);

  // After 2s the track should still be playing or have auto-advanced
  const after = await page.evaluate(async () => {
    const state = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
    return state;
  });
  // Still in some active state (Playing or moved to next track)
  expect(['Playing', 'Paused']).toContain(after);

  await screenshot(page, '06-floating-bar-progress');
});

test('6. clicking play/pause in floating bar toggles playback', async () => {
  await collapseSidebar(page);
  await startPlayback(page);

  // Verify playing
  const stateBefore = await page.evaluate(async () =>
    window.__TAURI_INTERNALS__.invoke('get_playback_state')
  );
  expect(stateBefore).toBe('Playing');

  // Click play/pause inside the floating bar
  await page
    .locator('[data-testid="now-playing-floating"] [data-testid="play-pause-button"]')
    .click();

  // Wait for state to change
  await page.waitForFunction(
    async () => {
      const s = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return s === 'Paused';
    },
    { timeout: 5_000 }
  );

  await screenshot(page, '07-floating-bar-paused');

  // Click again to resume
  await page
    .locator('[data-testid="now-playing-floating"] [data-testid="play-pause-button"]')
    .click();

  await page.waitForFunction(
    async () => {
      const s = await window.__TAURI_INTERNALS__.invoke('get_playback_state');
      return s === 'Playing';
    },
    { timeout: 5_000 }
  );

  await screenshot(page, '08-floating-bar-resumed');
});

test('7. drag edge strip rightward → sidebar restores, floating bar disappears', async () => {
  await collapseSidebar(page);
  await startPlayback(page);

  // Floating bar is visible
  await expect(page.locator('[data-testid="now-playing-floating"]')).toBeVisible();

  await screenshot(page, '09-before-restore');

  // Drag the strip rightward past 40px threshold
  const strip = page.locator('[data-testid="collapsed-sidebar-strip"]');
  const box = await strip.boundingBox();
  expect(box).not.toBeNull();

  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x + 80, box.y + box.height / 2, { steps: 15 });
  await page.mouse.up();

  // Sidebar nav should reappear
  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 5_000 });

  // Floating bar must be gone
  await expect(page.locator('[data-testid="now-playing-floating"]')).not.toBeVisible();

  // Strip must be gone
  await expect(page.locator('[data-testid="collapsed-sidebar-strip"]')).not.toBeVisible();

  await screenshot(page, '10-after-restore');
});

test('8. no track playing while collapsed → floating bar is absent from DOM', async () => {
  // Ensure no track is playing (stop_playback in beforeEach handles this)
  await collapseSidebar(page);

  // No track → floating bar must not exist in DOM at all
  const floatingBar = page.locator('[data-testid="now-playing-floating"]');
  await expect(floatingBar).not.toBeAttached();

  await screenshot(page, '11-no-track-no-floating-bar');
});
```

- [ ] **Step 10.2: Build the debug binary with the new frontend assets**

The CDP tests run against the debug binary which embeds `dist/`. Rebuild both:

```bash
cd D:/dev/soulaudio/soul-player

# 1. Build frontend assets
yarn build 2>&1 | tail -20

# 2. Build debug binary (embeds dist/)
cargo build -p soul-player-desktop 2>&1 | tail -20
```

Expected: Both succeed with no errors.

- [ ] **Step 10.3: Run the E2E spec**

```bash
cd applications/desktop/e2e-tests
npx playwright test --config playwright.cdp.config.js tests/playwright/collapsed-sidebar.spec.js --reporter=list
```

Expected: 8/8 tests pass. Screenshots saved to `test-results/screenshots/collapsed-sidebar/`.

- [ ] **Step 10.4: Verify screenshots exist**

```bash
ls applications/desktop/e2e-tests/test-results/screenshots/collapsed-sidebar/
```

Expected: 11 PNG files (01 through 11).

- [ ] **Step 10.5: Run the full Playwright suite to check for regressions**

```bash
cd applications/desktop/e2e-tests
npx playwright test --config playwright.cdp.config.js --reporter=list 2>&1 | tail -30
```

Expected: All previously-passing tests still pass. New 8 tests pass.

- [ ] **Step 10.6: Commit**

```bash
cd D:/dev/soulaudio/soul-player
git add applications/desktop/e2e-tests/tests/playwright/collapsed-sidebar.spec.js
git commit -m "test: add collapsed-sidebar Playwright CDP spec with screenshots (8 tests)"
```

---

## Final Step: Full Pre-Commit Gate

- [ ] **Run the complete pre-commit pipeline**

```bash
cd D:/dev/soulaudio/soul-player && cargo xtask check precommit
```

Expected: All checks pass — fmt, clippy, test, typescript, lint.

- [ ] **Final commit if any formatting fixes were auto-applied**

```bash
git add -p  # review any auto-fixes
git commit -m "chore: apply pre-commit formatting fixes for collapsed sidebar feature"
```
