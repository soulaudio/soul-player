# WebPlaybackProvider

**Abstract web playback provider for Soul Player**

`WebPlaybackProvider` is a reusable React component that bridges WASM-based audio playback (`@soul-player/playback-web`) to the shared `PlayerCommandsContext` interface. It enables consistent playback behavior across marketing demo, web player, and future web applications.

---

## Architecture Overview

```
┌────────────────────────────────────────────────────────────┐
│  React Application                                         │
│  └── Uses usePlayerCommands() hook                         │
├────────────────────────────────────────────────────────────┤
│  WebPlaybackProvider (applications/shared)                 │
│  ├── Initializes WasmPlaybackAdapter                       │
│  ├── Wires events to Zustand store                         │
│  ├── Provides PlayerCommandsContext                        │
│  └── Accepts generic PlaybackDataStorage                   │
├────────────────────────────────────────────────────────────┤
│  @soul-player/playback-web (libraries/soul-playback-web)   │
│  ├── WasmPlaybackAdapter - TypeScript adapter             │
│  ├── WebAudioPlayer - Web Audio API integration            │
│  └── WASM bindings (soul-playback Rust crate)              │
├────────────────────────────────────────────────────────────┤
│  WASM Module (libraries/soul-playback compiled to WASM)    │
│  ├── Queue management (PlayNext, AddToQueue, History)      │
│  ├── Shuffle/repeat modes                                  │
│  └── Playback state machine                                │
└────────────────────────────────────────────────────────────┘
```

---

## Key Features

- **Generic data source**: Accepts any storage implementing `PlaybackDataStorage` interface
- **Automatic initialization**: WASM module initialized internally on mount
- **Event bridge**: Automatically wires WASM events to shared Zustand store
- **Context provider**: Provides `PlayerCommandsContext` to children components
- **Cleanup handling**: Stops playback and cleans up resources on unmount
- **Type-safe**: Full TypeScript support with shared interfaces

---

## Usage

### Basic Setup

```tsx
import { WebPlaybackProvider, DemoStorage } from '@soul-player/shared';

// Create storage instance
const storage = new DemoStorage();
await storage.loadFromJson('/demo-data.json');

// Wrap your app
function App() {
  return (
    <WebPlaybackProvider storage={storage}>
      <YourApp />
    </WebPlaybackProvider>
  );
}
```

### Using Player Commands

```tsx
import { usePlayerCommands } from '@soul-player/shared';

function PlayButton() {
  const commands = usePlayerCommands();

  const handlePlay = async () => {
    await commands.playTrack('track-123');
  };

  return <button onClick={handlePlay}>Play</button>;
}
```

### Custom Storage Implementation

```tsx
import { PlaybackDataStorage, DemoTrack } from '@soul-player/shared';

class MyCustomStorage implements PlaybackDataStorage {
  private tracks: Map<string, DemoTrack> = new Map();

  getTrackById(id: string): DemoTrack | null {
    return this.tracks.get(id) || null;
  }

  // ... other methods
}

// Use with provider
<WebPlaybackProvider storage={new MyCustomStorage()}>
  <App />
</WebPlaybackProvider>
```

---

## PlaybackDataStorage Interface

The provider requires a data source implementing this interface:

```typescript
export interface PlaybackDataStorage {
  /**
   * Get a track by its ID
   * Used by playback provider to fetch track metadata (e.g., cover art URLs)
   */
  getTrackById(id: string): DemoTrack | null;
}
```

**Note**: This interface is intentionally minimal. The provider only needs track lookup for:
- Cover art URLs (WASM doesn't store these)
- Additional metadata enrichment
- Queue building

---

## API Reference

### Props

| Prop       | Type                    | Required | Description                          |
|------------|-------------------------|----------|--------------------------------------|
| `storage`  | `PlaybackDataStorage`   | Yes      | Data source for track metadata       |
| `children` | `ReactNode`             | Yes      | Child components to render           |

### PlayerCommandsInterface

All standard player commands are implemented:

```typescript
// Playback control
playTrack(trackId: string | number): Promise<void>
pausePlayback(): Promise<void>
resumePlayback(): Promise<void>
stopPlayback(): Promise<void>

// Navigation
skipNext(): Promise<void>
skipPrevious(): Promise<void>

// Seek and volume
seek(position: number): Promise<void>
setVolume(volume: number): Promise<void> // 0-1 range

// Shuffle and repeat
setShuffle(mode: 'off' | 'random' | 'smart'): Promise<void>
cycleShuffle(): Promise<'off' | 'random' | 'smart'>
getShuffle(): Promise<'off' | 'random' | 'smart'>
setRepeatMode(mode: 'off' | 'all' | 'one'): Promise<void>

// Queue management
getQueue(): Promise<QueueTrack[]>
playQueue(queue: QueueTrack[], startIndex?: number): Promise<void>
playQueueWithContext(context, initialBatch, startIndex, shuffle): Promise<void>
skipToQueueIndex(index: number): Promise<void>

// Three-tier queue operations
addPlayNext(track: QueueTrack): Promise<void>
addToQueueEnd(track: QueueTrack): Promise<void>
clearPlayNext(): Promise<void>
clearAddToQueue(): Promise<void>

// Capabilities
getPlaybackCapabilities(): Promise<PlaybackCapabilities>
getAllSources(): Promise<Source[]>
```

### PlaybackEventsInterface

Event listeners automatically wired to Zustand store:

```typescript
onStateChange(callback: (isPlaying: boolean) => void): () => void
onTrackChange(callback: (track: any) => void): () => void
onPositionUpdate(callback: (position: number) => void): () => void
onVolumeChange(callback: (volume: number) => void): () => void
onQueueUpdate(callback: () => void): () => void
onError(callback: (error: string) => void): () => void
```

---

## Event Bridge

`WebPlaybackProvider` automatically bridges WASM events to the shared Zustand store (`usePlayerStore`):

| WASM Event       | Store Update                                           |
|------------------|--------------------------------------------------------|
| `stateChange`    | `isPlaying: boolean`                                   |
| `trackChange`    | `currentTrack: Track`, `duration: number`              |
| `positionUpdate` | `progress: number` (percentage)                        |
| `volumeChange`   | `volume: number` (0-1 range, converted from 0-100)     |
| `shuffleChange`  | `shuffleMode: 'off' \| 'random' \| 'smart'`            |
| `repeatChange`   | `repeatMode: 'off' \| 'all' \| 'one'`                  |
| `queueChange`    | `queue: Track[]`                                       |

---

## Migration Guide

### From DemoPlayerCommandsProvider

**Before** (362 lines):
```tsx
// DemoPlayerCommandsProvider.tsx
import { getManager, getManagerSync } from '@/lib/demo/bridge';

export function DemoPlayerCommandsProvider({ storage, children }) {
  // Manual WASM initialization
  useEffect(() => {
    getManager(storage).then(() => setIsInitialized(true));
  }, [storage]);

  // Manual command implementations (100+ lines)
  const commands = {
    async playTrack(trackId) { /* ... */ },
    async pausePlayback() { /* ... */ },
    // ... 20+ more commands
  };

  // Manual event wiring (50+ lines)
  const events = {
    onStateChange(callback) { /* ... */ },
    onTrackChange(callback) { /* ... */ },
    // ... 6+ more events
  };

  return <PlayerCommandsProvider value={{ commands, events }}>
    {children}
  </PlayerCommandsProvider>;
}
```

**After** (25 lines):
```tsx
// DemoPlayerCommandsProvider.tsx
import { WebPlaybackProvider } from '@soul-player/shared';

export function DemoPlayerCommandsProvider({ storage, children }) {
  return <WebPlaybackProvider storage={storage}>{children}</WebPlaybackProvider>;
}
```

### What Changed?

1. **WASM initialization**: Now handled internally by `WebPlaybackProvider`
2. **Command implementations**: Moved to reusable provider in `@soul-player/shared`
3. **Event wiring**: Automatically configured by `setupEventBridge()` function
4. **Bridge removal**: `bridge.ts` logic integrated into `WebPlaybackProvider`

### No Changes Required

- UI components using `usePlayerCommands()` - work as-is
- Store selectors (`usePlayerStore`) - same behavior
- Event listeners - same callbacks, same timing

---

## Implementation Details

### WASM Initialization

```typescript
useEffect(() => {
  const manager = new WasmPlaybackAdapter();
  managerRef.current = manager;

  manager.initialize()
    .then(() => {
      setupEventBridge(manager, storage);
      setIsInitialized(true);
    });

  return () => {
    manager.stop(); // Cleanup on unmount
    managerRef.current = null;
  };
}, [storage]);
```

### Volume Conversion

WASM expects 0-100 range, shared interface uses 0-1:

```typescript
async setVolume(volume: number) {
  const volumePercent = Math.max(0, Math.min(100, Math.round(volume * 100)));
  getManagerOrThrow().setVolume(volumePercent);
}
```

### Queue Format Conversion

Shared `QueueTrack` → WASM `QueueTrack`:

```typescript
const wasmQueue = queue.map(track => ({
  id: track.trackId,
  title: track.title || 'Unknown',
  artist: track.artist || 'Unknown Artist', // CRITICAL: never undefined
  album: track.album || undefined,
  path: track.filePath,
  duration_secs: track.durationSeconds || 0, // underscore field name
  track_number: track.trackNumber || undefined, // underscore field name
  coverUrl: storage.getTrackById(track.trackId)?.coverUrl, // from storage
}));
```

---

## Testing

### Unit Tests

```typescript
import { render } from '@testing-library/react';
import { WebPlaybackProvider } from '@soul-player/shared';

test('initializes WASM manager', async () => {
  const storage = new DemoStorage();
  const { container } = render(
    <WebPlaybackProvider storage={storage}>
      <div>Test</div>
    </WebPlaybackProvider>
  );

  // Wait for initialization
  await waitFor(() => {
    expect(container.textContent).toBe('Test');
  });
});
```

### Integration Tests

```typescript
test('plays track and updates store', async () => {
  const storage = new DemoStorage();
  await storage.loadFromJson('/test-data.json');

  const { result } = renderHook(() => usePlayerCommands(), {
    wrapper: ({ children }) => (
      <WebPlaybackProvider storage={storage}>
        {children}
      </WebPlaybackProvider>
    ),
  });

  await act(async () => {
    await result.current.playTrack('track-1');
  });

  expect(usePlayerStore.getState().isPlaying).toBe(true);
});
```

---

## Future Enhancements

### Web Player Application

When building the full web player app:

```tsx
// applications/web-player/src/providers/WebPlayerCommandsProvider.tsx
import { WebPlaybackProvider, PlaybackDataStorage } from '@soul-player/shared';

class IndexedDBStorage implements PlaybackDataStorage {
  async getTrackById(id: string) {
    return await db.tracks.get(id);
  }
}

export function WebPlayerCommandsProvider({ children }) {
  const storage = useIndexedDBStorage(); // Hook to access IndexedDB

  return <WebPlaybackProvider storage={storage}>{children}</WebPlaybackProvider>;
}
```

### Server-Side Streaming

For streaming from Soul Server:

```tsx
class StreamingStorage implements PlaybackDataStorage {
  constructor(private apiClient: ApiClient) {}

  async getTrackById(id: string) {
    const track = await this.apiClient.getTrack(id);
    // Convert track.filePath to streaming URL
    track.path = `/api/stream/${id}`;
    return track;
  }
}
```

---

## Comparison: Desktop vs Web

| Feature                | Desktop (Tauri)                  | Web (WebPlaybackProvider)        |
|------------------------|----------------------------------|----------------------------------|
| **Backend**            | Rust (soul-playback)             | WASM (soul-playback compiled)    |
| **Audio Output**       | Native (cpal/rodio)              | Web Audio API                    |
| **File Access**        | Direct filesystem                | HTTP/IndexedDB                   |
| **Context**            | TauriPlayerCommandsProvider      | WebPlaybackProvider              |
| **Commands**           | Tauri invoke()                   | WASM function calls              |
| **Events**             | Tauri events                     | WASM event emitter               |
| **Store Sync**         | usePlaybackEvents hook           | setupEventBridge function        |
| **Shared Interface**   | PlayerCommandsContext ✓          | PlayerCommandsContext ✓          |

---

## Troubleshooting

### Children Not Rendering

**Problem**: Component tree not appearing

**Solution**: WASM initialization is async. Provider waits for `isInitialized` before rendering children.

```tsx
// Add loading state if needed
{!isInitialized && <LoadingSpinner />}
```

### Volume Not Updating

**Problem**: Volume slider not reflecting changes

**Solution**: Ensure volume is in 0-1 range (not 0-100). Provider converts internally.

```tsx
// ✅ Correct
await commands.setVolume(0.75); // 75%

// ❌ Wrong
await commands.setVolume(75); // Will be clamped to 1.0
```

### Cover Art Missing

**Problem**: Tracks show without album art

**Solution**: Ensure `PlaybackDataStorage.getTrackById()` returns tracks with `coverUrl` field.

```typescript
getTrackById(id: string): DemoTrack | null {
  const track = this.tracks.get(id);
  if (track && !track.coverUrl) {
    // Fallback to placeholder or fetch from API
    track.coverUrl = '/placeholder.jpg';
  }
  return track || null;
}
```

---

## Related Documentation

- **WASM Playback Library**: [libraries/soul-playback-web/README.md](../libraries/soul-playback-web/README.md)
- **Player Commands Context**: [applications/shared/src/contexts/PlayerCommandsContext.tsx](../applications/shared/src/contexts/PlayerCommandsContext.tsx)
- **Demo Storage**: [applications/shared/src/lib/demo-storage.ts](../applications/shared/src/lib/demo-storage.ts)
- **Playback Architecture**: [CLAUDE.md](../CLAUDE.md#playback-architecture-critical)

---

## Summary

`WebPlaybackProvider` eliminates code duplication by providing a reusable, generic playback provider for all web-based Soul Player applications. It:

- ✅ Reduces DemoPlayerCommandsProvider from 362 lines to 25 lines
- ✅ Provides consistent API across desktop and web
- ✅ Handles WASM initialization and cleanup automatically
- ✅ Wires events to store without manual bridge code
- ✅ Supports any data source via `PlaybackDataStorage` interface
- ✅ Fully type-safe with TypeScript

**Use it for**: Marketing demo, web player, web-based testing environments, any browser-based playback

**Don't use it for**: Desktop app (use `TauriPlayerCommandsProvider` instead)
