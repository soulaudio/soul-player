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

// Mock Theme Context
interface ThemeContextValue {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  currentTheme: any
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  availableThemes: any[]
  setTheme: (themeId: string) => boolean
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  importTheme: (themeJson: string) => any
  exportTheme: (themeId: string) => string | null
  deleteTheme: (themeId: string) => boolean
  previewTheme: (themeId: string) => (() => void) | null
}

const mockTheme = {
  id: 'default-dark',
  name: 'Default Dark',
  colors: {},
}

const ThemeContext = createContext<ThemeContextValue>({
  currentTheme: mockTheme,
  availableThemes: [mockTheme],
  setTheme: () => false,
  importTheme: () => ({ valid: false }),
  exportTheme: () => null,
  deleteTheme: () => false,
  previewTheme: () => null,
})

export function MockThemeProvider({ children }: { children: ReactNode }) {
  return (
    <ThemeContext.Provider
      value={{
        currentTheme: mockTheme,
        availableThemes: [mockTheme],
        setTheme: () => false,
        importTheme: () => ({ valid: false }),
        exportTheme: () => null,
        deleteTheme: () => false,
        previewTheme: () => null,
      }}
    >
      {children}
    </ThemeContext.Provider>
  )
}
