/**
 * ScrollVisibilityContext - Shares scroll-based header visibility state
 * Used to sync search bar and window controls auto-hide behavior
 */

import { createContext, useContext, useState, ReactNode } from 'react'

interface ScrollVisibilityContextValue {
  showHeader: boolean
  setShowHeader: (show: boolean) => void
}

const ScrollVisibilityContext = createContext<ScrollVisibilityContextValue | undefined>(undefined)

export function ScrollVisibilityProvider({ children }: { children: ReactNode }) {
  const [showHeader, setShowHeader] = useState(true)

  return (
    <ScrollVisibilityContext.Provider value={{ showHeader, setShowHeader }}>
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
