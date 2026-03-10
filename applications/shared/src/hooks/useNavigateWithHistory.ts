import { useLocation, useNavigate, NavigateOptions } from 'react-router-dom'
import { useCallback } from 'react'

interface NavigateWithHistoryReturn {
  /**
   * Navigate to a new route while preserving the current location in history.
   * The current location will be available in the destination's location.state.from
   */
  navigate: (to: string, options?: NavigateOptions) => void

  /**
   * Navigate back to the previous location stored in history state.
   * Falls back to the provided default location if no history is available.
   */
  goBack: (defaultPath?: string) => void

  /**
   * Whether there is navigation history available (i.e., we came from somewhere)
   */
  hasHistory: boolean
}

/**
 * Custom hook that wraps React Router's navigate function to automatically
 * track navigation history using location state.
 *
 * Usage:
 * ```tsx
 * const { navigate, goBack } = useNavigateWithHistory()
 *
 * // When navigating to a detail page
 * navigate('/albums/123')  // Automatically saves current location
 *
 * // When navigating back
 * goBack('/albums')  // Goes to previous location or fallback
 * ```
 */
export function useNavigateWithHistory(): NavigateWithHistoryReturn {
  const location = useLocation()
  const reactRouterNavigate = useNavigate()

  // Check if we have navigation history
  const from = (location.state as any)?.from
  const hasHistory = !!(from && typeof from === 'string')

  const navigate = useCallback((to: string, options?: NavigateOptions) => {
    // Preserve any existing state from options, but add 'from' to it
    const state = {
      ...options?.state,
      from: location.pathname + location.search,
    }

    reactRouterNavigate(to, {
      ...options,
      state,
    })
  }, [location.pathname, location.search, reactRouterNavigate])

  const goBack = useCallback((defaultPath: string = '/albums') => {
    // Check if we have navigation history in location state
    const from = (location.state as any)?.from

    if (from && typeof from === 'string') {
      // Navigate back to where we came from, marking it as a back navigation
      // so scroll restoration can differentiate from fresh forward navigation
      reactRouterNavigate(from, { state: { isBack: true } })
    } else {
      reactRouterNavigate(defaultPath, { state: { isBack: true } })
    }
  }, [location.state, reactRouterNavigate])

  return { navigate, goBack, hasHistory }
}
