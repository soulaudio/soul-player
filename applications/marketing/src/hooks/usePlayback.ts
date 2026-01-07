/**
 * React hook for playback state management
 * Wraps DemoPlaybackManager with React state
 */

import { useState, useEffect, useRef, useCallback } from 'react'
import { DemoPlaybackManager } from '@/lib/demo/playback-manager'
import {
  QueueTrack,
  PlaybackState,
  RepeatMode,
  ShuffleMode
} from '@/lib/demo/types'

export interface UsePlaybackReturn {
  // State
  state: PlaybackState
  currentTrack: QueueTrack | null
  position: number
  duration: number
  queue: QueueTrack[]
  volume: number
  isMuted: boolean
  shuffle: ShuffleMode
  repeat: RepeatMode
  hasNext: boolean
  hasPrevious: boolean

  // Playback controls
  play: () => Promise<void>
  pause: () => void
  stop: () => void
  next: () => Promise<void>
  previous: () => Promise<void>
  seek: (position: number) => void
  seekPercent: (percent: number) => void

  // Queue management
  addToQueue: (track: QueueTrack) => void
  addToQueueNext: (track: QueueTrack) => void
  playNow: (track: QueueTrack) => Promise<void>
  playAlbum: (tracks: QueueTrack[]) => Promise<void>
  clearQueue: () => void

  // Settings
  setVolume: (volume: number) => void
  toggleMute: () => void
  setShuffle: (mode: ShuffleMode) => void
  setRepeat: (mode: RepeatMode) => void

  // Manager instance (for advanced usage)
  manager: DemoPlaybackManager | null
}

export function usePlayback(): UsePlaybackReturn {
  // State
  const [state, setState] = useState<PlaybackState>(PlaybackState.Stopped)
  const [currentTrack, setCurrentTrack] = useState<QueueTrack | null>(null)
  const [position, setPosition] = useState(0)
  const [duration, setDuration] = useState(0)
  const [queue, setQueue] = useState<QueueTrack[]>([])
  const [volume, setVolumeState] = useState(80)
  const [isMuted, setIsMuted] = useState(false)
  const [shuffle, setShuffleState] = useState(ShuffleMode.Off)
  const [repeat, setRepeatState] = useState(RepeatMode.Off)

  // Manager instance
  const managerRef = useRef<DemoPlaybackManager | null>(null)

  // Initialize manager
  useEffect(() => {
    const manager = new DemoPlaybackManager()
    managerRef.current = manager

    // Subscribe to events
    const unsubscribers = [
      manager.on('stateChange', (newState: PlaybackState) => {
        setState(newState)
      }),

      manager.on('trackChange', (track: QueueTrack | null) => {
        setCurrentTrack(track)
        setDuration(track ? track.duration : 0)
      }),

      manager.on('positionUpdate', (pos: number) => {
        setPosition(pos)
      }),

      manager.on('queueChange', () => {
        setQueue(manager.getQueue())
      }),

      manager.on('volumeChange', (vol: number) => {
        setVolumeState(vol)
      }),

      manager.on('muteChange', (muted: boolean) => {
        setIsMuted(muted)
      }),

      manager.on('shuffleChange', (mode: ShuffleMode) => {
        setShuffleState(mode)
      }),

      manager.on('repeatChange', (mode: RepeatMode) => {
        setRepeatState(mode)
      })
    ]

    // Cleanup on unmount
    return () => {
      unsubscribers.forEach(unsub => unsub())
      manager.destroy()
    }
  }, [])

  // Playback controls
  const play = useCallback(async () => {
    await managerRef.current?.play()
  }, [])

  const pause = useCallback(() => {
    managerRef.current?.pause()
  }, [])

  const stop = useCallback(() => {
    managerRef.current?.stop()
  }, [])

  const next = useCallback(async () => {
    await managerRef.current?.next()
  }, [])

  const previous = useCallback(async () => {
    await managerRef.current?.previous()
  }, [])

  const seek = useCallback((pos: number) => {
    managerRef.current?.seek(pos)
  }, [])

  const seekPercent = useCallback((percent: number) => {
    managerRef.current?.seekPercent(percent)
  }, [])

  // Queue management
  const addToQueue = useCallback((track: QueueTrack) => {
    managerRef.current?.addToQueueEnd(track)
  }, [])

  const addToQueueNext = useCallback((track: QueueTrack) => {
    managerRef.current?.addToQueueNext(track)
  }, [])

  const playNow = useCallback(async (track: QueueTrack) => {
    const manager = managerRef.current
    if (!manager) return

    // Clear queue and add track
    manager.clearQueue()
    manager.addToQueueNext(track)
    await manager.play()
  }, [])

  const playAlbum = useCallback(async (tracks: QueueTrack[]) => {
    const manager = managerRef.current
    if (!manager) return

    // Load playlist and start playing
    manager.clearQueue()
    manager.loadPlaylist(tracks)
    await manager.play()
  }, [])

  const clearQueue = useCallback(() => {
    managerRef.current?.clearQueue()
  }, [])

  // Settings
  const setVolume = useCallback((vol: number) => {
    managerRef.current?.setVolume(vol)
  }, [])

  const toggleMute = useCallback(() => {
    managerRef.current?.toggleMute()
  }, [])

  const setShuffle = useCallback((mode: ShuffleMode) => {
    managerRef.current?.setShuffle(mode)
  }, [])

  const setRepeat = useCallback((mode: RepeatMode) => {
    managerRef.current?.setRepeat(mode)
  }, [])

  // Computed values
  const hasNext = managerRef.current?.hasNext() ?? false
  const hasPrevious = managerRef.current?.hasPrevious() ?? false

  return {
    // State
    state,
    currentTrack,
    position,
    duration,
    queue,
    volume,
    isMuted,
    shuffle,
    repeat,
    hasNext,
    hasPrevious,

    // Controls
    play,
    pause,
    stop,
    next,
    previous,
    seek,
    seekPercent,

    // Queue
    addToQueue,
    addToQueueNext,
    playNow,
    playAlbum,
    clearQueue,

    // Settings
    setVolume,
    toggleMute,
    setShuffle,
    setRepeat,

    // Manager
    manager: managerRef.current
  }
}
