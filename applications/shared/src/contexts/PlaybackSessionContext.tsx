/**
 * PlaybackSessionContext - Single Source of Truth for Playback State
 *
 * This context tracks the current playback session with IMMEDIATE, SYNCHRONOUS updates.
 * No debouncing, no async queries - just pure state management.
 *
 * Purpose:
 * - Track what's currently playing (context: album/artist/playlist)
 * - Track playback state (isPlaying, currentTrack, queue)
 * - Provide instant context checking (isActiveContext)
 *
 * Replaces:
 * - PlaybackContextProvider (had 500ms debounce causing stale context)
 * - MockBackendProvider.currentContextRef (fragmented state)
 *
 * Updated by:
 * - WebPlaybackProvider (on playQueue, and via WASM event bridge)
 *
 * Consumed by:
 * - MediaCard (for play/pause toggle logic)
 * - PlayerControls (for UI state)
 * - HomePage "Jump Back In" (for recent contexts)
 */

import { createContext, useContext, useState, useCallback, ReactNode } from 'react'
import type { Track } from '../types'

// =============================================================================
// Types
// =============================================================================

export interface PlaybackSession {
  // Context info (what entity is playing)
  contextType: 'album' | 'artist' | 'playlist' | null
  contextId: string | null
  contextName: string | null
  contextArtworkPath: string | null

  // Playback state (synced from WASM via events)
  currentTrack: Track | null
  isPlaying: boolean
  queue: Track[]

  // Metadata
  startedAt: Date | null
}

export interface PlaybackSessionContextValue {
  /** Current playback session state */
  session: PlaybackSession

  /**
   * Check if an entity (album/artist/playlist) is the current active playback context
   * @param type - Entity type (album, artist, playlist)
   * @param id - Entity ID
   * @returns true if this entity is currently playing
   */
  isActiveContext: (type: 'album' | 'artist' | 'playlist', id: number | string) => boolean

  /**
   * Update session state (INTERNAL - called by WebPlaybackProvider only)
   * @param updates - Partial session updates
   */
  updateSession: (updates: Partial<PlaybackSession>) => void

  /**
   * Clear session (INTERNAL - called on stop/cleanup)
   */
  clearSession: () => void
}

// =============================================================================
// Context
// =============================================================================

const PlaybackSessionContext = createContext<PlaybackSessionContextValue | null>(null)

export function usePlaybackSession(): PlaybackSessionContextValue {
  const context = useContext(PlaybackSessionContext)
  if (!context) {
    throw new Error('usePlaybackSession must be used within PlaybackSessionProvider')
  }
  return context
}

// =============================================================================
// Provider
// =============================================================================

const initialSession: PlaybackSession = {
  contextType: null,
  contextId: null,
  contextName: null,
  contextArtworkPath: null,
  currentTrack: null,
  isPlaying: false,
  queue: [],
  startedAt: null,
}

interface PlaybackSessionProviderProps {
  children: ReactNode
}

export function PlaybackSessionProvider({ children }: PlaybackSessionProviderProps) {
  const [session, setSession] = useState<PlaybackSession>(initialSession)

  // Update session (called by WebPlaybackProvider)
  const updateSession = useCallback((updates: Partial<PlaybackSession>) => {
    setSession((prev) => ({
      ...prev,
      ...updates,
    }))
  }, [])

  // Clear session
  const clearSession = useCallback(() => {
    setSession(initialSession)
  }, [])

  // Check if entity is active context
  const isActiveContext = useCallback(
    (type: 'album' | 'artist' | 'playlist', id: number | string): boolean => {
      if (!session.contextType || !session.contextId) {
        return false
      }

      return session.contextType === type && session.contextId === String(id)
    },
    [session.contextType, session.contextId]
  )

  const value: PlaybackSessionContextValue = {
    session,
    isActiveContext,
    updateSession,
    clearSession,
  }

  return (
    <PlaybackSessionContext.Provider value={value}>
      {children}
    </PlaybackSessionContext.Provider>
  )
}
