# Playback System - Tauri Integration Guide

## Overview

The playback system is fully integrated with Tauri, providing a complete audio playback solution for the desktop application. This guide shows frontend developers how to use the playback API.

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                 Frontend (React)                     │
│  ┌──────────────┐         ┌──────────────┐          │
│  │  Commands    │────────▶│   Events     │          │
│  │  (invoke)    │         │  (listen)    │          │
│  └──────────────┘         └──────────────┘          │
└─────────────┬──────────────────┬────────────────────┘
              │                  │
              ▼                  ▼
┌─────────────────────────────────────────────────────┐
│            Tauri Backend (Rust)                      │
│  ┌──────────────────────────────────┐                │
│  │      PlaybackManager             │                │
│  │  ┌────────────┐  ┌────────────┐  │                │
│  │  │ Commands   │  │   Events   │  │                │
│  │  └────────────┘  └────────────┘  │                │
│  │         │              ▲          │                │
│  │         ▼              │          │                │
│  │  ┌────────────────────┴────┐     │                │
│  │  │   DesktopPlayback       │     │                │
│  │  │  (CPAL + PlaybackMgr)   │     │                │
│  │  └─────────────────────────┘     │                │
│  └──────────────────────────────────┘                │
└─────────────────────────────────────────────────────┘
```

## Available Commands

### Playback Control

#### play_track
Play a specific track from file path.

```typescript
import { invoke } from '@tauri-apps/api/core';

await invoke('play_track', { filePath: '/path/to/song.mp3' });
```

#### play
Start playback (resume if paused).

```typescript
await invoke('play');
```

#### pause_playback
Pause playback.

```typescript
await invoke('pause_playback');
```

#### resume_playback
Resume playback (alias for play).

```typescript
await invoke('resume_playback');
```

#### stop_playback
Stop playback and reset.

```typescript
await invoke('stop_playback');
```

#### next_track
Skip to next track in queue.

```typescript
await invoke('next_track');
```

#### previous_track
Go to previous track.

```typescript
await invoke('previous_track');
```

### Volume Control

#### set_volume
Set volume (0-100).

```typescript
await invoke('set_volume', { volume: 80 });
```

#### mute
Mute audio.

```typescript
await invoke('mute');
```

#### unmute
Unmute audio.

```typescript
await invoke('unmute');
```

### Seek

#### seek_to
Seek to position in seconds.

```typescript
await invoke('seek_to', { position: 45.5 });
```

### Shuffle & Repeat

#### set_shuffle
Set shuffle mode: "off", "random", or "smart".

```typescript
await invoke('set_shuffle', { mode: 'random' });
```

#### set_repeat
Set repeat mode: "off", "all", or "one".

```typescript
await invoke('set_repeat', { mode: 'all' });
```

### Queue Management

#### clear_queue
Clear the playback queue.

```typescript
await invoke('clear_queue');
```

## Available Events

### playback:state-changed
Emitted when playback state changes.

**Payload**: `"Stopped" | "Playing" | "Paused" | "Loading"`

```typescript
import { listen } from '@tauri-apps/api/event';

const unlisten = await listen('playback:state-changed', (event) => {
  console.log('State:', event.payload);
  // Update UI based on state
});
```

### playback:track-changed
Emitted when current track changes.

**Payload**: `QueueTrack | null`

```typescript
const unlisten = await listen('playback:track-changed', (event) => {
  const track = event.payload;
  if (track) {
    console.log('Now playing:', track.title, 'by', track.artist);
  }
});
```

### playback:position-updated
Emitted periodically during playback with current position.

**Payload**: `number` (seconds)

```typescript
const unlisten = await listen('playback:position-updated', (event) => {
  const position = event.payload;
  // Update progress bar
  updateProgressBar(position);
});
```

### playback:volume-changed
Emitted when volume changes.

**Payload**: `number` (0-100)

```typescript
const unlisten = await listen('playback:volume-changed', (event) => {
  const volume = event.payload;
  // Update volume slider
  setVolumeSlider(volume);
});
```

### playback:queue-updated
Emitted when queue is modified.

**Payload**: `void`

```typescript
const unlisten = await listen('playback:queue-updated', () => {
  // Refresh queue display
  refreshQueue();
});
```

### playback:error
Emitted when an error occurs.

**Payload**: `string` (error message)

```typescript
const unlisten = await listen('playback:error', (event) => {
  const error = event.payload;
  console.error('Playback error:', error);
  showErrorNotification(error);
});
```

## React Integration Examples

### usePlayback Hook

```typescript
import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export function usePlayback() {
  const [state, setState] = useState<'Stopped' | 'Playing' | 'Paused' | 'Loading'>('Stopped');
  const [currentTrack, setCurrentTrack] = useState<any | null>(null);
  const [position, setPosition] = useState(0);
  const [volume, setVolume] = useState(80);

  useEffect(() => {
    // Listen for events
    const unlistenState = listen('playback:state-changed', (event) => {
      setState(event.payload as any);
    });

    const unlistenTrack = listen('playback:track-changed', (event) => {
      setCurrentTrack(event.payload);
    });

    const unlistenPosition = listen('playback:position-updated', (event) => {
      setPosition(event.payload as number);
    });

    const unlistenVolume = listen('playback:volume-changed', (event) => {
      setVolume(event.payload as number);
    });

    // Cleanup
    return () => {
      Promise.all([unlistenState, unlistenTrack, unlistenPosition, unlistenVolume])
        .then((fns) => fns.forEach((fn) => fn()));
    };
  }, []);

  const play = () => invoke('play');
  const pause = () => invoke('pause_playback');
  const next = () => invoke('next_track');
  const previous = () => invoke('previous_track');
  const seek = (position: number) => invoke('seek_to', { position });
  const setVol = (vol: number) => invoke('set_volume', { volume: vol });
  const mute = () => invoke('mute');
  const unmute = () => invoke('unmute');
  const setShuffle = (mode: string) => invoke('set_shuffle', { mode });
  const setRepeat = (mode: string) => invoke('set_repeat', { mode });

  return {
    state,
    currentTrack,
    position,
    volume,
    play,
    pause,
    next,
    previous,
    seek,
    setVolume: setVol,
    mute,
    unmute,
    setShuffle,
    setRepeat,
  };
}
```

### Player Component Example

```typescript
import React from 'react';
import { usePlayback } from './hooks/usePlayback';

export function Player() {
  const {
    state,
    currentTrack,
    position,
    volume,
    play,
    pause,
    next,
    previous,
    seek,
    setVolume,
    setShuffle,
    setRepeat,
  } = usePlayback();

  const togglePlay = () => {
    if (state === 'Playing') {
      pause();
    } else {
      play();
    }
  };

  return (
    <div className="player">
      <div className="track-info">
        {currentTrack ? (
          <>
            <h3>{currentTrack.title}</h3>
            <p>{currentTrack.artist}</p>
          </>
        ) : (
          <p>No track playing</p>
        )}
      </div>

      <div className="controls">
        <button onClick={previous}>⏮️</button>
        <button onClick={togglePlay}>
          {state === 'Playing' ? '⏸️' : '▶️'}
        </button>
        <button onClick={next}>⏭️</button>
      </div>

      <div className="progress">
        <input
          type="range"
          min={0}
          max={currentTrack?.duration || 100}
          value={position}
          onChange={(e) => seek(parseFloat(e.target.value))}
        />
        <span>
          {formatTime(position)} / {formatTime(currentTrack?.duration || 0)}
        </span>
      </div>

      <div className="volume">
        <input
          type="range"
          min={0}
          max={100}
          value={volume}
          onChange={(e) => setVolume(parseInt(e.target.value))}
        />
        <span>{volume}%</span>
      </div>

      <div className="modes">
        <button onClick={() => setShuffle('random')}>🔀</button>
        <button onClick={() => setRepeat('all')}>🔁</button>
      </div>
    </div>
  );
}

function formatTime(seconds: number): string {
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins}:${secs.toString().padStart(2, '0')}`;
}
```

## Type Definitions

```typescript
// types/playback.ts

export type PlaybackState = 'Stopped' | 'Playing' | 'Paused' | 'Loading';

export type ShuffleMode = 'off' | 'random' | 'smart';

export type RepeatMode = 'off' | 'all' | 'one';

export interface QueueTrack {
  id: string;
  path: string;
  title: string;
  artist: string;
  album?: string;
  duration: number; // seconds
  track_number?: number;
  source: TrackSource;
}

export type TrackSource =
  | { type: 'playlist'; id: string }
  | { type: 'album'; id: string }
  | { type: 'artist'; id: string }
  | { type: 'manual' };
```

## Implementation Status

✅ **Completed**:
- All playback commands implemented
- Event emission system working
- Full integration with DesktopPlayback
- Sample React hooks provided

⏸️ **Pending**:
- soul-storage integration (being worked on separately)
- Queue management commands (add/remove specific tracks)
- Playlist integration

## Notes

1. **Event Polling**: Events are polled every 50ms from the background thread
2. **Thread Safety**: All playback operations are thread-safe via Arc<Mutex<>>
3. **Audio Formats**: Supports MP3, FLAC, OGG, WAV, AAC, OPUS via Symphonia
4. **Error Handling**: All errors are emitted via `playback:error` event

## Testing

To test playback integration:

1. Build the Tauri app: `cargo tauri dev`
2. Open developer console
3. Test commands:
```javascript
// Play a local file
await invoke('play_track', { filePath: '/absolute/path/to/song.mp3' });

// Control playback
await invoke('pause_playback');
await invoke('resume_playback');
await invoke('set_volume', { volume: 50 });

// Listen for events
listen('playback:state-changed', (e) => console.log('State:', e.payload));
listen('playback:track-changed', (e) => console.log('Track:', e.payload));
```

## Troubleshooting

### Audio not playing
- Check file path is absolute and exists
- Ensure audio device is available
- Check console for `playback:error` events

### Events not received
- Verify event listener is set up before commands
- Check event name spelling (they're case-sensitive)
- Ensure proper cleanup of listeners

### Volume not changing
- Volume range is 0-100 (not 0-1)
- Check if muted (use unmute command)
