'use client'

/**
 * DemoInitializer - Sets up initial demo state
 * - Selects a random track on load (paused)
 * - Must be rendered inside all providers
 */

import { useEffect, useRef } from 'react'
import { usePlayerStore } from '@soul-player/shared'
import { getDemoStorage } from '@/lib/demo/storage'
import type { DemoTrack } from '@/lib/demo/types'

// Track type matching the player store's expected type (from types/index.ts)
interface PlayerTrack {
  id: number
  title: string
  artist: string
  album: string
  albumId?: number
  duration: number
  filePath: string
  trackNumber?: number
  coverArtPath?: string
  addedAt: string
}

function demoTrackToPlayerTrack(dt: DemoTrack, index: number): PlayerTrack {
  return {
    id: parseInt(dt.id, 10) || index,
    title: dt.title,
    artist: dt.artist,
    album: dt.album || 'Unknown Album',
    duration: dt.duration,
    filePath: dt.path,
    trackNumber: dt.trackNumber,
    coverArtPath: dt.coverUrl,
    addedAt: new Date().toISOString(),
  }
}

export function DemoInitializer({ children }: { children: React.ReactNode }) {
  const initialized = useRef(false)
  const setCurrentTrack = usePlayerStore((state) => state.setCurrentTrack)
  const setQueue = usePlayerStore((state) => state.setQueue)
  const setDuration = usePlayerStore((state) => state.setDuration)

  useEffect(() => {
    // Only initialize once
    if (initialized.current) return
    initialized.current = true

    const storage = getDemoStorage()
    const tracks = storage.getAllTracks()

    if (tracks.length === 0) return

    // Pick a random track
    const randomIndex = Math.floor(Math.random() * tracks.length)
    const demoTrack = tracks[randomIndex]

    // Convert to Track type for player store
    const track = demoTrackToPlayerTrack(demoTrack, randomIndex)

    // Set the current track (paused by default since we don't set isPlaying)
    setCurrentTrack(track)
    setDuration(demoTrack.duration)

    // Also set a queue with a few random tracks
    const shuffledTracks = [...tracks].sort(() => Math.random() - 0.5)
    const queueTracks = shuffledTracks
      .slice(0, Math.min(10, tracks.length))
      .map((t, i) => demoTrackToPlayerTrack(t, i))

    setQueue(queueTracks)
  }, [setCurrentTrack, setQueue, setDuration])

  return <>{children}</>
}
