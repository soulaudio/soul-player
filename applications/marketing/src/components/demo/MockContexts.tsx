'use client'

import { createContext, ReactNode, useState } from 'react'

// Mock Settings Context
interface SettingsContextValue {
  showKeyboardShortcuts: boolean
  setShowKeyboardShortcuts: (show: boolean) => void
}

const SettingsContext = createContext<SettingsContextValue>({
  showKeyboardShortcuts: true,
  setShowKeyboardShortcuts: () => {},
})

export function MockSettingsProvider({ children }: { children: ReactNode }) {
  const [showKeyboardShortcuts, setShowKeyboardShortcuts] = useState(true)

  return (
    <SettingsContext.Provider
      value={{
        showKeyboardShortcuts,
        setShowKeyboardShortcuts,
      }}
    >
      {children}
    </SettingsContext.Provider>
  )
}
