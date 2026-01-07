/**
 * Bridge between DemoPlaybackManager and shared Zustand store
 * Makes demo work exactly like desktop
 */

import { usePlayerStore } from '@soul-player/shared/stores/player'
import { DemoPlaybackManager } from './playback-manager'
import { PlaybackState, RepeatMode as DemoRepeatMode, ShuffleMode as DemoShuffleMode } from './types'
import type { Track } from '@soul-player/shared/types'

// Global manager instance
let managerInstance: DemoPlaybackManager | null = null

export function getManager(): DemoPlaybackManager {
  if (!managerInstance) {
    managerInstance = new DemoPlaybackManager()
    setupBridge()
  }
  return managerInstance
}

function setupBridge() {
  const manager = managerInstance!
  const store = usePlayerStore.getState()

  console.log('[Bridge] Setting up event bridge')

  // Bridge demo events to shared store
  manager.on('stateChange', (state: PlaybackState) => {
    console.log('[Bridge] State change:', state)
    usePlayerStore.setState({ isPlaying: state === PlaybackState.Playing })
  })

  manager.on('trackChange', (track: any) => {
    if (track) {
      const sharedTrack: Track = {
        id: parseInt(track.id) || 0,
        title: track.title,
        artist: track.artist,
        album: track.album || '',
        duration: Math.floor(track.duration),
        filePath: track.path,
        addedAt: new Date().toISOString(),
      }
      usePlayerStore.setState({ currentTrack: sharedTrack, duration: track.duration })
    } else {
      usePlayerStore.setState({ currentTrack: null, duration: 0 })
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

// Initialize bridge on first import
if (typeof window !== 'undefined') {
  getManager()
}
