/**
 * ScrollVisibilityContext - Shares scroll-based header visibility state
 * Used to sync search bar and window controls auto-hide behavior
 */

import { createContext, useContext, useState, useMemo, ReactNode } from 'react'

interface ScrollVisibilityContextValue {
  showHeader: boolean
  setShowHeader: (show: boolean) => void
}

const ScrollVisibilityContext = createContext<ScrollVisibilityContextValue | undefined>(undefined)

export function ScrollVisibilityProvider({ children }: { children: ReactNode }) {
  const [showHeader, setShowHeader] = useState(true)

  // Memoize context value to prevent unnecessary re-renders
  // setShowHeader is stable from useState, only showHeader changes
  const value = useMemo(() => ({ showHeader, setShowHeader }), [showHeader])

  return (
    <ScrollVisibilityContext.Provider value={value}>
      {children}
    </ScrollVisibilityContext.Provider>
  )
}

export function useScrollVisibility() {
  const context = useContext(ScrollVisibilityContext)
  if (!context) {
    throw new Error('useScrollVisibility must be used within ScrollVisibilityProvider')
  }
  return context
}
