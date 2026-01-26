/**
 * PlaybackContextProvider - provides shared playback context data to all MediaCards
 *
 * CRITICAL PERFORMANCE FIX:
 * Before this provider, every MediaCard fetched recent contexts independently,
 * causing 50+ database queries per track change (one per card on screen).
 *
 * This provider fetches contexts ONCE and shares them with all cards,
 * reducing 50+ queries to 1 query per track change.
 *
 * Pattern:
 * - Fetch recent contexts once at app level
 * - Store in React context
 * - Provide lookup function: isActiveContext(type, id)
 * - Auto-refresh on currentTrack change
 */

import { createContext, useContext, useState, useEffect, useCallback, ReactNode } from 'react'
import { useBackend, type PlaybackContext } from './BackendContext'
import { useCurrentTrack } from '../stores/player'
import { debug } from '../utils/debug'

// =============================================================================
// Context Interface
// =============================================================================

export interface PlaybackContextData {
  /**
   * Check if an entity (album/artist/playlist) is the current active playback context
   * @param type - Entity type (album, artist, playlist)
   * @param id - Entity ID
   * @returns true if this entity is the current playback context
   */
  isActiveContext: (type: 'album' | 'artist' | 'playlist', id: number | string) => boolean

  /**
   * Recent playback contexts (for debugging or advanced use)
   */
  contexts: PlaybackContext[]

  /**
   * Loading state
   */
  isLoading: boolean

  /**
   * Manually refresh contexts (bypasses debounce)
   * Use this when you need immediate context updates (e.g., after starting playback)
   */
  refreshContexts?: () => void
}

const PlaybackContextContext = createContext<PlaybackContextData | null>(null)

export function usePlaybackContext(): PlaybackContextData {
  const context = useContext(PlaybackContextContext)
  if (!context) {
    throw new Error('usePlaybackContext must be used within PlaybackContextProvider')
  }
  return context
}

// =============================================================================
// Provider
// =============================================================================

interface PlaybackContextProviderProps {
  children: ReactNode
}

export function PlaybackContextProvider({ children }: PlaybackContextProviderProps) {
  const backend = useBackend()
  const currentTrack = useCurrentTrack()
  const [contexts, setContexts] = useState<PlaybackContext[]>([])
  const [isLoading, setIsLoading] = useState(true)

  // Shared fetch function
  const fetchContexts = useCallback(async () => {
    try {
      debug.log('[PlaybackContextProvider] Fetching recent contexts...')
      const recentContexts = await backend.getRecentContexts(10)
      setContexts(recentContexts)
      setIsLoading(false)
      debug.log(`[PlaybackContextProvider] Loaded ${recentContexts.length} contexts`)
    } catch (error) {
      debug.error('[PlaybackContextProvider] Failed to fetch contexts:', error)
      setContexts([])
      setIsLoading(false)
    }
  }, [backend])

  // Fetch recent contexts (limit to 10 - we only need the most recent for active context check)
  // PERFORMANCE: Debounce to avoid excessive queries on rapid track changes (e.g., skipping)
  useEffect(() => {
    let isCancelled = false

    // Debounce: wait 500ms after track change before fetching
    const timer = setTimeout(() => {
      if (!isCancelled) {
        fetchContexts()
      }
    }, 500) // 500ms debounce - batches rapid track skips into a single query

    return () => {
      isCancelled = true
      clearTimeout(timer)
    }
  }, [backend, currentTrack, fetchContexts]) // Re-fetch when track changes

  // Lookup function: check if entity is active context
  const isActiveContext = useCallback((type: 'album' | 'artist' | 'playlist', id: number | string): boolean => {
    if (contexts.length === 0) return false

    const mostRecent = contexts[0]
    return mostRecent?.contextType === type && mostRecent?.contextId === String(id)
  }, [contexts])

  const value: PlaybackContextData = {
    isActiveContext,
    contexts,
    isLoading,
    refreshContexts: fetchContexts,
  }

  return (
    <PlaybackContextContext.Provider value={value}>
      {children}
    </PlaybackContextContext.Provider>
  )
}
