import { useState, useEffect } from 'react'
import { useBackend } from '../contexts/BackendContext'
import { debug } from '../utils/debug'

export type TrackNumberDisplay = 'hide' | 'show' | 'vinyl'

export const TRACK_NUMBER_DISPLAY_EVENT = 'track-number-display-changed'
export const TRACK_NUMBER_DISPLAY_KEY = 'ui.track_number_display'
export const TRACK_NUMBER_DISPLAY_DEFAULT: TrackNumberDisplay = 'show'

export function useTrackNumberDisplay(): TrackNumberDisplay {
  const backend = useBackend()
  const [mode, setMode] = useState<TrackNumberDisplay>(TRACK_NUMBER_DISPLAY_DEFAULT)

  useEffect(() => {
    backend.getUserSetting(TRACK_NUMBER_DISPLAY_KEY)
      .then(val => {
        if (val === 'hide' || val === 'show' || val === 'vinyl') {
          setMode(val)
        }
      })
      .catch(err => debug.error('[useTrackNumberDisplay] Failed to load setting:', err))
  }, [backend])

  useEffect(() => {
    const handler = (e: Event) => {
      const { mode } = (e as CustomEvent<{ mode: TrackNumberDisplay }>).detail
      setMode(mode)
    }
    window.addEventListener(TRACK_NUMBER_DISPLAY_EVENT, handler)
    return () => window.removeEventListener(TRACK_NUMBER_DISPLAY_EVENT, handler)
  }, [])

  return mode
}
