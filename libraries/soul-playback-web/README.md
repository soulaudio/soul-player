# @soul-player/playback-web

Web-based audio playback library using WASM and Web Audio API.

## Overview

`@soul-player/playback-web` is a reusable TypeScript library that provides high-quality audio playback capabilities for browser-based applications. It leverages WebAssembly (WASM) for performance-critical audio processing and the Web Audio API for platform-native audio output.

**Key Features:**
- 🎵 High-quality audio playback using Symphonia decoder (via WASM)
- 🔄 Queue management with shuffle and repeat modes
- 📊 Real-time playback state synchronization
- 🎚️ Volume control and crossfade support
- ⚡ Automatic event emission for UI updates
- 🧪 Fully testable with TypeScript
- 🔌 Framework-agnostic core (can be used with React, Vue, etc.)

## Purpose

This library was extracted from the Soul Player marketing demo to:
1. **Reduce code duplication** between marketing demo and future web player
2. **Improve maintainability** by centralizing web playback logic
3. **Enable reusability** across multiple web-based applications
4. **Ensure consistency** with desktop playback behavior

## Installation

This package is part of the Soul Player monorepo and uses Yarn workspaces.

```bash
# Install dependencies (from root)
yarn

# Add to your application's package.json
{
  "dependencies": {
    "@soul-player/playback-web": "workspace:*"
  }
}
```

## Usage

### Basic Setup

```typescript
import { WasmPlaybackAdapter, WebAudioPlayer } from '@soul-player/playback-web'

// Initialize the WASM adapter
const adapter = await WasmPlaybackAdapter.create()

// Create an audio player
const audioPlayer = new WebAudioPlayer()

// Set up event listeners
adapter.on('stateChange', (state) => {
  console.log('Playback state:', state) // 'playing', 'paused', 'stopped'
})

adapter.on('trackChange', (track) => {
  console.log('Now playing:', track.title)
})

adapter.on('positionUpdate', (position) => {
  console.log('Position:', position.current, '/', position.duration)
})

// Play a queue
await adapter.playQueue(tracks, 0) // Play from first track
```

### Integration with React

```typescript
import { useEffect, useState } from 'react'
import { WasmPlaybackAdapter } from '@soul-player/playback-web'

function usePlayback() {
  const [adapter, setAdapter] = useState<WasmPlaybackAdapter | null>(null)

  useEffect(() => {
    WasmPlaybackAdapter.create().then(setAdapter)
    return () => adapter?.cleanup()
  }, [])

  return adapter
}

function App() {
  const adapter = usePlayback()

  const handlePlay = async () => {
    await adapter?.play()
  }

  const handlePause = async () => {
    await adapter?.pause()
  }

  return (
    <div>
      <button onClick={handlePlay}>Play</button>
      <button onClick={handlePause}>Pause</button>
    </div>
  )
}
```

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│  Application Layer (React/Vue/etc.)                     │
│  └── Playback Provider (manages adapter lifecycle)      │
├─────────────────────────────────────────────────────────┤
│  @soul-player/playback-web                              │
│  ├── WasmPlaybackAdapter (core orchestration)           │
│  │   ├── Automatic event emission                       │
│  │   ├── Queue state management                         │
│  │   └── Error handling                                 │
│  ├── WebAudioPlayer (Web Audio API wrapper)             │
│  │   └── Audio buffer playback                          │
│  └── WASM Bindings (Rust ↔ JS bridge)                   │
│      └── soul-playback WASM module                      │
└─────────────────────────────────────────────────────────┘
```

## API Overview

### WasmPlaybackAdapter

Core playback orchestration class.

**Methods:**
- `static create(): Promise<WasmPlaybackAdapter>` - Initialize adapter with WASM
- `playQueue(tracks: Track[], startIndex: number): Promise<void>` - Play a queue of tracks
- `play(): Promise<void>` - Resume playback
- `pause(): Promise<void>` - Pause playback
- `next(): Promise<void>` - Skip to next track
- `previous(): Promise<void>` - Go to previous track
- `seek(position: number): Promise<void>` - Seek to position (seconds)
- `setVolume(volume: number): Promise<void>` - Set volume (0-100)
- `setShuffle(enabled: boolean): Promise<void>` - Toggle shuffle mode
- `setRepeat(mode: RepeatMode): Promise<void>` - Set repeat mode
- `getQueue(): QueueState` - Get current queue state
- `cleanup(): void` - Clean up resources

**Events:**
- `stateChange` - Playback state changed (playing/paused/stopped)
- `trackChange` - Current track changed
- `positionUpdate` - Playback position updated (emitted every 250ms)
- `volumeChange` - Volume changed
- `queueChange` - Queue modified (shuffle, next, previous, etc.)
- `error` - Error occurred

### WebAudioPlayer

Low-level Web Audio API wrapper.

**Methods:**
- `loadTrack(audioData: ArrayBuffer): Promise<void>` - Load audio data
- `play(): void` - Start playback
- `pause(): void` - Pause playback
- `seek(position: number): void` - Seek to position
- `setVolume(volume: number): void` - Set volume (0-1 range)
- `getCurrentTime(): number` - Get current playback position
- `getDuration(): number` - Get track duration

## Development

```bash
# Type checking
yarn workspace @soul-player/playback-web type-check

# Linting
yarn workspace @soul-player/playback-web lint

# Testing
yarn workspace @soul-player/playback-web test

# Watch mode
yarn workspace @soul-player/playback-web test:watch

# Coverage
yarn workspace @soul-player/playback-web test:coverage
```

## Testing

The library uses Vitest for unit testing with comprehensive coverage.

### Test Coverage (Phase 4 ✅ COMPLETE)

- **Total Tests**: 94 passing
- **Overall Coverage**: 75%
- **WebAudioPlayer**: 95% coverage (33 tests)
- **WasmPlaybackAdapter**: 74% coverage (61 tests)

### Running Tests

```bash
# Run all tests
yarn test

# Watch mode
yarn test:watch

# Coverage report
yarn test:coverage
```

### Manual Testing Checklist

For comprehensive manual testing on the marketing demo, see:
- [Testing Checklist](../../docs/TESTING_CHECKLIST_WEB_PLAYBACK.md)

### Example Test

```typescript
import { describe, it, expect } from 'vitest'
import { WasmPlaybackAdapter } from '@soul-player/playback-web'

describe('WasmPlaybackAdapter', () => {
  it('should initialize successfully', async () => {
    const adapter = new WasmPlaybackAdapter()
    await adapter.initialize()
    expect(adapter.getState()).toBe('stopped')
  })

  it('should emit events when playing a queue', async () => {
    const adapter = new WasmPlaybackAdapter()
    await adapter.initialize()

    const events: string[] = []
    adapter.on('trackChange', () => events.push('trackChange'))
    adapter.on('stateChange', () => events.push('stateChange'))

    adapter.loadPlaylist(mockTracks)
    await adapter.play()

    expect(events).toContain('trackChange')
    expect(events).toContain('stateChange')
  })
})
```

## Roadmap

This library is being developed in phases:

- ✅ **Phase 1**: Extract core playback code from marketing demo
- ✅ **Phase 2**: Implement automatic event emission
- ✅ **Phase 3**: Create reusable WebPlaybackProvider for React
- ✅ **Phase 4**: Add comprehensive test coverage (94 tests, 75% coverage)
- 🚧 **Phase 5**: Documentation and examples (in progress)
- 🔜 **Phase 6**: Use in production web player application

See [WEB_PLAYBACK_REFACTOR_ROADMAP.md](../../docs/WEB_PLAYBACK_REFACTOR_ROADMAP.md) for details.

## Contributing

This library follows the Soul Player project conventions:

- **TypeScript**: Strict mode enabled
- **Linting**: ESLint with project rules
- **Testing**: Vitest with good coverage (50-60%)
- **Documentation**: JSDoc comments for all public APIs
- **Code Style**: Prettier formatting

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for general contribution guidelines.

## License

MIT License - see [LICENSE](../../LICENSE) for details.

---

**Note**: This library is currently in active development as part of the Web Playback Refactor initiative. APIs may change between versions until a stable 1.0.0 release.
