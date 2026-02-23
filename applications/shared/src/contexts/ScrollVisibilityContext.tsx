/**
 * ScrollVisibilityContext - Shares scroll-based header visibility state
 * Used to sync search bar and window controls auto-hide behavior.
 * Also exposes scrollContainerRef so VirtualizedGrid can share the same
 * scroll element as LibraryPageLayout (avoids nested scroll containers).
 */

import { createContext, useContext, useState, useMemo, useRef, ReactNode, RefObject } from 'react'

interface ScrollVisibilityContextValue {
  showHeader: boolean
  setShowHeader: (show: boolean) => void
  /** Ref to the active page's scroll container, set by LibraryPageLayout */
  scrollContainerRef: RefObject<HTMLDivElement>
}

const ScrollVisibilityContext = createContext<ScrollVisibilityContextValue | undefined>(undefined)

export function ScrollVisibilityProvider({ children }: { children: ReactNode }) {
  const [showHeader, setShowHeader] = useState(true)
  const scrollContainerRef = useRef<HTMLDivElement>(null)

  // scrollContainerRef is a stable object — safe to include in deps array
  const value = useMemo(
    () => ({ showHeader, setShowHeader, scrollContainerRef }),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [showHeader]
  )

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
