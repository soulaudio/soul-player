/**
 * PlaybackSessionContext - Playback Context Tracking
 *
 * Tracks the current playback context (album/artist/playlist) while deriving
 * playback state (isPlaying, currentTrack, queue) from Zustand store.
 *
 * Architecture:
 * - Context-specific fields (contextType, contextId, etc.) stored in local state
 * - Playback state (isPlaying, currentTrack, queue) derived from Zustand store
 * - Single source of truth: Zustand store for playback, local state for context
 *
 * Purpose:
 * - Track what entity is currently playing (context: album/artist/playlist)
 * - Provide instant context checking (isActiveContext)
 * - Derive playback state from Zustand to avoid dual updates
 *
 * Updated by:
 * - WebPlaybackProvider (on playQueue for context info)
 *
 * Consumed by:
 * - MediaCard (for play/pause toggle logic)
 * - PlayerControls (for UI state)
 * - HomePage "Jump Back In" (for recent contexts)
 */

import { createContext, useContext, useState, useCallback, ReactNode, useMemo } from 'react'
import { usePlayerStore } from '../stores/player'
import type { Track } from '../types'

// =============================================================================
// Types
// =============================================================================

/**
 * Context-specific state (stored locally in this provider)
 */
interface PlaybackContextState {
  contextType: 'album' | 'artist' | 'playlist' | null
  contextId: string | null
  contextName: string | null
  startedAt: Date | null
}

/**
 * Full session state (context + derived playback state)
 */
export interface PlaybackSession {
  // Context info (what entity is playing) - stored locally
  contextType: 'album' | 'artist' | 'playlist' | null
  contextId: string | null
  contextName: string | null
  contextArtworkPath: string | null

  // Playback state - derived from Zustand store
  currentTrack: Track | null
  isPlaying: boolean
  queue: Track[]

  // Metadata
  startedAt: Date | null
}

export interface PlaybackSessionContextValue {
  /** Current playback session state (context + derived playback state) */
  session: PlaybackSession

  /**
   * Check if an entity (album/artist/playlist) is the current active playback context
   * @param type - Entity type (album, artist, playlist)
   * @param id - Entity ID
   * @returns true if this entity is currently playing
   */
  isActiveContext: (type: 'album' | 'artist' | 'playlist', id: number | string) => boolean

  /**
   * Update context state (INTERNAL - called by WebPlaybackProvider only)
   * Note: Only updates context-specific fields. Playback state is derived from Zustand.
   * @param updates - Partial context updates
   */
  updateSession: (updates: Partial<PlaybackContextState>) => void

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

const initialContextState: PlaybackContextState = {
  contextType: null,
  contextId: null,
  contextName: null,
  startedAt: null,
}

interface PlaybackSessionProviderProps {
  children: ReactNode
}

export function PlaybackSessionProvider({ children }: PlaybackSessionProviderProps) {
  // Store only context-specific fields locally
  const [contextState, setContextState] = useState<PlaybackContextState>(initialContextState)

  // Derive playback state from Zustand store (single source of truth)
  const currentTrack = usePlayerStore((state) => state.currentTrack)
  const isPlaying = usePlayerStore((state) => state.isPlaying)
  const queue = usePlayerStore((state) => state.queue)

  // Combine context state + derived playback state into full session
  const session = useMemo<PlaybackSession>(() => {
    // Derive contextArtworkPath from current track's cover art
    const contextArtworkPath = currentTrack?.coverArtPath || null

    return {
      ...contextState,
      contextArtworkPath,
      currentTrack,
      isPlaying,
      queue,
    }
  }, [contextState, currentTrack, isPlaying, queue])

  // Update context state (called by WebPlaybackProvider)
  // Note: Only updates context-specific fields. Playback state is derived from Zustand.
  const updateSession = useCallback((updates: Partial<PlaybackContextState>) => {
    setContextState((prev) => ({
      ...prev,
      ...updates,
    }))
  }, [])

  // Clear session
  const clearSession = useCallback(() => {
    setContextState(initialContextState)
  }, [])

  // Check if entity is active context
  const isActiveContext = useCallback(
    (type: 'album' | 'artist' | 'playlist', id: number | string): boolean => {
      if (!contextState.contextType || !contextState.contextId) {
        return false
      }

      return contextState.contextType === type && contextState.contextId === String(id)
    },
    [contextState.contextType, contextState.contextId]
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
