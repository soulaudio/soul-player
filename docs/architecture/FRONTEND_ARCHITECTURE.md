# Frontend Architecture

This document describes the React frontend architecture for Soul Player's desktop and mobile applications.

---

## Overview

Soul Player uses a **shared component library** approach:
- Common UI components in `applications/shared/`
- Platform-specific features in `applications/desktop/` and `applications/mobile/`
- Same tech stack across platforms (React + Tailwind + shadcn/ui)

---

## Technology Stack

### Core
- **React 18+**: UI framework with concurrent features
- **TypeScript**: Type-safe JavaScript
- **Vite**: Build tool and dev server
- **Tauri 2.0**: Desktop/mobile runtime

### UI & Styling
- **Tailwind CSS 4**: Utility-first CSS with @theme design tokens
- **shadcn/ui**: Copy-paste component library built on Radix UI
- **Radix UI**: Headless, accessible UI primitives
- **Lucide React**: Icon library

### State Management
- **Zustand**: Lightweight state management (recommended over Redux)
- **React Query** (optional): Server state caching for sync features

### Testing
- **Vitest**: Fast unit test runner
- **Testing Library**: Component testing utilities
- **WebdriverIO**: E2E tests for Tauri apps

---

## Folder Structure

```
applications/
├── shared/                          # Shared components
│   ├── src/
│   │   ├── components/
│   │   │   ├── ui/                  # shadcn/ui base components
│   │   │   ├── player/              # Player-specific components
│   │   │   ├── library/             # Library views
│   │   │   └── playlists/           # Playlist components
│   │   ├── stores/                  # Zustand stores
│   │   ├── hooks/                   # React hooks
│   │   └── lib/                     # Utilities
│   └── tests/
│
├── desktop/                         # Desktop app
│   ├── src/
│   │   ├── main.tsx
│   │   ├── App.tsx
│   │   └── features/                # Desktop-only features
│   └── src-tauri/
│
└── mobile/                          # Mobile app
    ├── src/
    │   ├── main.tsx
    │   ├── App.tsx
    │   └── features/                # Mobile-only features
    └── src-tauri/
```

---

## Component Architecture

### Atomic Design Principles

Soul Player follows a modified atomic design:

1. **UI Primitives** (`shared/components/ui/`)
   - shadcn/ui components (Button, Slider, Dialog, etc.)
   - Basic building blocks
   - Platform-agnostic

2. **Domain Components** (`shared/components/{player,library,playlists}/`)
   - Composed from UI primitives
   - Business logic integrated
   - Reusable across platforms

3. **Feature Components** (`desktop/features/`, `mobile/features/`)
   - Platform-specific compositions
   - Desktop: MenuBar, TitleBar, SystemTray
   - Mobile: BottomNav, Gestures, NowPlayingSheet

4. **Layouts** (`App.tsx` in each app)
   - Top-level composition
   - Platform-specific routing
   - Shell structure

---

## Example: Player Controls

### Shared Component

**Location**: `applications/shared/src/components/player/Controls.tsx`

```typescript
import { Button } from '@shared/components/ui/button';
import { Play, Pause, SkipBack, SkipForward } from 'lucide-react';
import { usePlayerStore } from '@shared/stores/player';
import { invoke } from '@tauri-apps/api/core';

export interface PlayerControlsProps {
  /** Size variant for different layouts */
  size?: 'sm' | 'md' | 'lg';
  /** Show skip buttons */
  showSkip?: boolean;
  /** Custom className for styling */
  className?: string;
}

export function PlayerControls({
  size = 'md',
  showSkip = true,
  className,
}: PlayerControlsProps) {
  const { isPlaying, currentTrack } = usePlayerStore();

  const handlePlayPause = async () => {
    try {
      if (isPlaying) {
        await invoke('pause_playback');
      } else {
        await invoke('resume_playback');
      }
    } catch (error) {
      console.error('Playback error:', error);
    }
  };

  const handleSkipBack = async () => {
    await invoke('skip_previous');
  };

  const handleSkipForward = async () => {
    await invoke('skip_next');
  };

  const buttonSize = {
    sm: 'h-8 w-8',
    md: 'h-10 w-10',
    lg: 'h-12 w-12',
  }[size];

  const iconSize = {
    sm: 'h-4 w-4',
    md: 'h-5 w-5',
    lg: 'h-6 w-6',
  }[size];

  return (
    <div className={`flex items-center gap-2 ${className}`}>
      {showSkip && (
        <Button
          variant="ghost"
          size="icon"
          className={buttonSize}
          onClick={handleSkipBack}
          aria-label="Previous track"
        >
          <SkipBack className={iconSize} />
        </Button>
      )}

      <Button
        variant="default"
        size="icon"
        className={`${buttonSize} rounded-full`}
        onClick={handlePlayPause}
        disabled={!currentTrack}
        aria-label={isPlaying ? 'Pause' : 'Play'}
      >
        {isPlaying ? (
          <Pause className={iconSize} />
        ) : (
          <Play className={iconSize} />
        )}
      </Button>

      {showSkip && (
        <Button
          variant="ghost"
          size="icon"
          className={buttonSize}
          onClick={handleSkipForward}
          aria-label="Next track"
        >
          <SkipForward className={iconSize} />
        </Button>
      )}
    </div>
  );
}
```

### Desktop Usage

**Location**: `applications/desktop/src/App.tsx`

```typescript
import { PlayerControls } from '@shared/components/player';
import { CustomTitleBar } from '@/features/titlebar/CustomTitleBar';

export function App() {
  return (
    <div className="flex flex-col h-screen">
      <CustomTitleBar />

      {/* Main content */}
      <div className="flex-1 overflow-auto">
        {/* Library, playlists, etc. */}
      </div>

      {/* Player bar - desktop size */}
      <div className="border-t bg-background p-4">
        <PlayerControls size="md" showSkip={true} />
      </div>
    </div>
  );
}
```

### Mobile Usage

**Location**: `applications/mobile/src/App.tsx`

```typescript
import { PlayerControls } from '@shared/components/player';
import { BottomNavigation } from '@/features/bottom-nav/BottomNavigation';
import { NowPlayingSheet } from '@/features/now-playing-sheet/NowPlayingSheet';

export function App() {
  return (
    <div className="flex flex-col h-screen">
      {/* Main content */}
      <div className="flex-1 overflow-auto pb-32">
        {/* Routes */}
      </div>

      {/* Mobile player bar - larger touch targets */}
      <div className="fixed bottom-16 left-0 right-0 border-t bg-background p-4">
        <PlayerControls size="lg" showSkip={false} />
      </div>

      {/* Bottom navigation */}
      <BottomNavigation />

      {/* Now playing sheet */}
      <NowPlayingSheet />
    </div>
  );
}
```

---

## State Management with Zustand

### Player Store

**Location**: `applications/shared/src/stores/player.ts`

```typescript
import { create } from 'zustand';
import { Track } from '@shared/types';

interface PlayerState {
  // Playback state
  currentTrack: Track | null;
  isPlaying: boolean;
  volume: number;
  progress: number; // 0-100%
  duration: number; // seconds

  // Queue
  queue: Track[];
  queueIndex: number;

  // Actions
  setCurrentTrack: (track: Track | null) => void;
  setIsPlaying: (isPlaying: boolean) => void;
  setVolume: (volume: number) => void;
  setProgress: (progress: number) => void;
  addToQueue: (tracks: Track[]) => void;
  removeFromQueue: (index: number) => void;
  clearQueue: () => void;
}

export const usePlayerStore = create<PlayerState>((set) => ({
  // Initial state
  currentTrack: null,
  isPlaying: false,
  volume: 0.8,
  progress: 0,
  duration: 0,
  queue: [],
  queueIndex: -1,

  // Actions
  setCurrentTrack: (track) => set({ currentTrack: track }),
  setIsPlaying: (isPlaying) => set({ isPlaying }),
  setVolume: (volume) => set({ volume }),
  setProgress: (progress) => set({ progress }),

  addToQueue: (tracks) =>
    set((state) => ({
      queue: [...state.queue, ...tracks],
    })),

  removeFromQueue: (index) =>
    set((state) => ({
      queue: state.queue.filter((_, i) => i !== index),
    })),

  clearQueue: () => set({ queue: [], queueIndex: -1 }),
}));
```

### Library Store

**Location**: `applications/shared/src/stores/library.ts`

```typescript
import { create } from 'zustand';
import { Track, Album, Artist } from '@shared/types';
import { invoke } from '@tauri-apps/api/core';

interface LibraryState {
  tracks: Track[];
  albums: Album[];
  artists: Artist[];
  isLoading: boolean;
  error: string | null;

  // Actions
  loadLibrary: () => Promise<void>;
  addTracks: (files: string[]) => Promise<void>;
  searchLibrary: (query: string) => Track[];
}

export const useLibraryStore = create<LibraryState>((set, get) => ({
  tracks: [],
  albums: [],
  artists: [],
  isLoading: false,
  error: null,

  loadLibrary: async () => {
    set({ isLoading: true, error: null });
    try {
      const tracks = await invoke<Track[]>('get_all_tracks');
      const albums = await invoke<Album[]>('get_all_albums');
      const artists = await invoke<Artist[]>('get_all_artists');

      set({ tracks, albums, artists, isLoading: false });
    } catch (error) {
      set({ error: String(error), isLoading: false });
    }
  },

  addTracks: async (files) => {
    set({ isLoading: true });
    try {
      await invoke('scan_files', { files });
      await get().loadLibrary();
    } catch (error) {
      set({ error: String(error), isLoading: false });
    }
  },

  searchLibrary: (query) => {
    const { tracks } = get();
    const lowerQuery = query.toLowerCase();

    return tracks.filter(
      (track) =>
        track.title.toLowerCase().includes(lowerQuery) ||
        track.artist.toLowerCase().includes(lowerQuery) ||
        track.album.toLowerCase().includes(lowerQuery)
    );
  },
}));
```

---

## Tauri API Integration

### Type-Safe Tauri Commands

**Location**: `applications/shared/src/lib/tauri.ts`

```typescript
import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { Track, Playlist, Album } from '@shared/types';

// Type-safe wrappers for Tauri commands

export const commands = {
  // Player commands
  playTrack: (trackId: number) =>
    tauriInvoke<void>('play_track', { trackId }),

  pausePlayback: () =>
    tauriInvoke<void>('pause_playback'),

  resumePlayback: () =>
    tauriInvoke<void>('resume_playback'),

  stopPlayback: () =>
    tauriInvoke<void>('stop_playback'),

  setVolume: (volume: number) =>
    tauriInvoke<void>('set_volume', { volume }),

  // Library commands
  getAllTracks: () =>
    tauriInvoke<Track[]>('get_all_tracks'),

  getAllAlbums: () =>
    tauriInvoke<Album[]>('get_all_albums'),

  scanFiles: (files: string[]) =>
    tauriInvoke<void>('scan_files', { files }),

  // Playlist commands
  createPlaylist: (name: string) =>
    tauriInvoke<Playlist>('create_playlist', { name }),

  addToPlaylist: (playlistId: number, trackIds: number[]) =>
    tauriInvoke<void>('add_to_playlist', { playlistId, trackIds }),

  getPlaylists: () =>
    tauriInvoke<Playlist[]>('get_playlists'),
};
```

### Custom Hooks

**Location**: `applications/shared/src/hooks/useAudioPlayer.ts`

```typescript
import { useEffect } from 'react';
import { usePlayerStore } from '@shared/stores/player';
import { commands } from '@shared/lib/tauri';
import { listen } from '@tauri-apps/api/event';

export function useAudioPlayer() {
  const { setProgress, setIsPlaying, setCurrentTrack } = usePlayerStore();

  useEffect(() => {
    // Listen to playback events from Rust backend
    const unlistenProgress = listen<number>('playback-progress', (event) => {
      setProgress(event.payload);
    });

    const unlistenEnded = listen('playback-ended', () => {
      setIsPlaying(false);
      // Auto-play next track
      commands.skipNext();
    });

    const unlistenError = listen<string>('playback-error', (event) => {
      console.error('Playback error:', event.payload);
      setIsPlaying(false);
    });

    return () => {
      unlistenProgress.then((fn) => fn());
      unlistenEnded.then((fn) => fn());
      unlistenError.then((fn) => fn());
    };
  }, []);

  return usePlayerStore();
}
```

---

## Responsive Design

### Tailwind Breakpoints

```css
/* applications/shared/src/styles/index.css */
@import "tailwindcss";

@theme {
  /* Responsive breakpoints */
  --breakpoint-sm: 640px;   /* Mobile landscape */
  --breakpoint-md: 768px;   /* Tablet */
  --breakpoint-lg: 1024px;  /* Desktop */
  --breakpoint-xl: 1280px;  /* Large desktop */
}
```

### Platform Detection Hook

**Location**: `applications/shared/src/hooks/usePlatform.ts`

```typescript
import { useEffect, useState } from 'react';

export type Platform = 'desktop' | 'mobile' | 'unknown';

export function usePlatform(): Platform {
  const [platform, setPlatform] = useState<Platform>('unknown');

  useEffect(() => {
    // Detect platform from Tauri
    import('@tauri-apps/plugin-os')
      .then((os) => os.platform())
      .then((platformName) => {
        if (platformName === 'ios' || platformName === 'android') {
          setPlatform('mobile');
        } else {
          setPlatform('desktop');
        }
      })
      .catch(() => setPlatform('desktop')); // Fallback to desktop
  }, []);

  return platform;
}
```

### Adaptive Components

```typescript
import { usePlatform } from '@shared/hooks/usePlatform';

export function AdaptiveLayout({ children }: { children: React.ReactNode }) {
  const platform = usePlatform();

  if (platform === 'mobile') {
    return (
      <div className="mobile-layout pb-safe">
        {children}
      </div>
    );
  }

  return (
    <div className="desktop-layout">
      {children}
    </div>
  );
}
```

---

## Testing Strategy

### Component Tests (Vitest + Testing Library)

**Location**: `applications/shared/tests/unit/components/PlayerControls.test.tsx`

```typescript
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { PlayerControls } from '@shared/components/player/Controls';
import { usePlayerStore } from '@shared/stores/player';

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('PlayerControls', () => {
  it('renders play button when not playing', () => {
    usePlayerStore.setState({ isPlaying: false });

    render(<PlayerControls />);

    const playButton = screen.getByLabelText('Play');
    expect(playButton).toBeInTheDocument();
  });

  it('renders pause button when playing', () => {
    usePlayerStore.setState({ isPlaying: true });

    render(<PlayerControls />);

    const pauseButton = screen.getByLabelText('Pause');
    expect(pauseButton).toBeInTheDocument();
  });

  it('calls pause command when clicked', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    usePlayerStore.setState({ isPlaying: true, currentTrack: mockTrack });

    render(<PlayerControls />);

    const pauseButton = screen.getByLabelText('Pause');
    fireEvent.click(pauseButton);

    expect(invoke).toHaveBeenCalledWith('pause_playback');
  });
});
```

### Store Tests

**Location**: `applications/shared/tests/unit/stores/player.test.ts`

```typescript
import { describe, it, expect, beforeEach } from 'vitest';
import { usePlayerStore } from '@shared/stores/player';

describe('Player Store', () => {
  beforeEach(() => {
    usePlayerStore.setState({
      currentTrack: null,
      isPlaying: false,
      queue: [],
    });
  });

  it('sets current track', () => {
    const track = { id: 1, title: 'Test Track' };

    usePlayerStore.getState().setCurrentTrack(track);

    expect(usePlayerStore.getState().currentTrack).toEqual(track);
  });

  it('adds tracks to queue', () => {
    const tracks = [
      { id: 1, title: 'Track 1' },
      { id: 2, title: 'Track 2' },
    ];

    usePlayerStore.getState().addToQueue(tracks);

    expect(usePlayerStore.getState().queue).toHaveLength(2);
  });

  it('removes track from queue by index', () => {
    const tracks = [
      { id: 1, title: 'Track 1' },
      { id: 2, title: 'Track 2' },
    ];

    usePlayerStore.getState().addToQueue(tracks);
    usePlayerStore.getState().removeFromQueue(0);

    expect(usePlayerStore.getState().queue).toHaveLength(1);
    expect(usePlayerStore.getState().queue[0].id).toBe(2);
  });
});
```

---

## Best Practices

### 1. **Component Composition over Props Drilling**
```typescript
// Good: Use context/stores
const { isPlaying } = usePlayerStore();

// Avoid: Prop drilling through many layers
<Parent isPlaying={isPlaying}>
  <Child isPlaying={isPlaying}>
    <GrandChild isPlaying={isPlaying} />
  </Child>
</Parent>
```

### 2. **Separation of Concerns**
- UI components: Pure presentation
- Stores: Business logic and state
- Hooks: Side effects and integrations
- Lib: Utilities and helpers

### 3. **TypeScript Everywhere**
- Define types in `@shared/types`
- Use Tauri's TypeScript bindings
- No `any` types in production code

### 4. **Accessibility**
- Use semantic HTML
- ARIA labels on interactive elements
- Keyboard navigation support
- Screen reader testing

### 5. **Performance**
- Lazy load routes
- Virtualize long lists
- Memoize expensive computations
- Optimize re-renders with React.memo

---

## Summary

Soul Player's frontend uses a **shared component library** approach with platform-specific features layered on top. This enables:

- **Code reuse**: 70-80% of UI code shared between desktop and mobile
- **Consistency**: Same look and feel across platforms
- **Maintainability**: Changes to shared components benefit all platforms
- **Flexibility**: Platform-specific features when needed

The architecture balances abstraction with pragmatism, avoiding over-engineering while maintaining clean separation of concerns.
