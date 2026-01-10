/**
 * Bridge between WASM PlaybackManager and shared Zustand store
 * Makes demo work exactly like desktop using shared Rust logic
 */

import { usePlayerStore } from '@soul-player/shared/stores/player'
import { WasmPlaybackAdapter } from './wasm-playback-adapter'
import { PlaybackState, RepeatMode as DemoRepeatMode, ShuffleMode as DemoShuffleMode, type QueueTrack } from './types'
import type { Track } from '@soul-player/shared/types'
import { getDemoStorage } from './storage'

// Global manager instance
let managerInstance: WasmPlaybackAdapter | null = null
let initPromise: Promise<void> | null = null

export async function getManager(): Promise<WasmPlaybackAdapter> {
  if (!managerInstance) {
    managerInstance = new WasmPlaybackAdapter()
    initPromise = managerInstance.initialize().then(() => {
      setupBridge()
    })
  }

  if (initPromise) {
    await initPromise
  }

  return managerInstance
}

export function getManagerSync(): WasmPlaybackAdapter | null {
  return managerInstance
}

function setupBridge() {
  const manager = managerInstance!

  console.log('[Bridge] Setting up event bridge')

  // Bridge demo events to shared store
  manager.on('stateChange', (state: PlaybackState) => {
    console.log('[Bridge] State change:', state)
    usePlayerStore.setState({ isPlaying: state === PlaybackState.Playing })
  })

  manager.on('trackChange', (track: QueueTrack | null) => {
    if (track) {
      // Convert demo QueueTrack to shared Track format
      // track.id is string from WASM, convert to number for shared interface
      const trackId = Number(track.id);

      // Look up cover URL from demo storage (WASM doesn't store it)
      const storage = getDemoStorage();
      const demoTrack = storage.getTrackById(track.id);
      const coverUrl = demoTrack?.coverUrl || undefined;

      console.log('[Bridge] Track changed:', {
        id: track.id,
        convertedId: trackId,
        title: track.title,
        coverUrl: coverUrl
      });

      const sharedTrack: Track = {
        id: trackId,
        title: track.title,
        artist: track.artist,
        album: track.album || '',
        duration: Math.floor(track.duration_secs),  // Use correct field name
        filePath: track.path,
        coverArtPath: coverUrl,  // Look up from storage
        addedAt: new Date().toISOString(),
      };

      usePlayerStore.setState({ currentTrack: sharedTrack, duration: track.duration_secs });
    } else {
      console.log('[Bridge] Track cleared');
      usePlayerStore.setState({ currentTrack: null, duration: 0 });
    }
  })

  manager.on('positionUpdate', (position: number) => {
    const duration = manager.getDuration()
    if (duration > 0) {
      const progress = (position / duration) * 100
      // console.log('[Bridge] Position update:', { position, duration, progress }) // Too verbose
      usePlayerStore.setState({ progress })
    }
  })

  manager.on('volumeChange', (volume: number) => {
    console.log('[Bridge] Volume change:', volume, '-> store:', volume / 100)
    usePlayerStore.setState({ volume: volume / 100 }) // 0-100 to 0-1
  })

  manager.on('shuffleChange', (mode: DemoShuffleMode) => {
    usePlayerStore.setState({ shuffleEnabled: mode !== DemoShuffleMode.Off })
  })

  manager.on('repeatChange', (mode: DemoRepeatMode) => {
    const modeMap: Record<DemoRepeatMode, 'off' | 'all' | 'one'> = {
      [DemoRepeatMode.Off]: 'off',
      [DemoRepeatMode.All]: 'all',
      [DemoRepeatMode.One]: 'one',
    }
    usePlayerStore.setState({ repeatMode: modeMap[mode] })
  })
}

// Initialize bridge on first import (async)
if (typeof window !== 'undefined') {
  getManager().catch(err => {
    console.error('[Bridge] Failed to initialize WASM playback manager:', err)
  })
}
