'use client'

import { useEffect, useState } from 'react'
import dynamic from 'next/dynamic'

// Completely skip SSR for ThemeProvider - it reads localStorage
const SharedThemeProvider = dynamic(
  () => import('@soul-player/shared').then((mod) => mod.ThemeProvider),
  { ssr: false }
)

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  return <SharedThemeProvider>{children}</SharedThemeProvider>
}

/**
 * Client-side i18n provider that initializes react-i18next
 * Must be used in a 'use client' component
 */
export function I18nProvider({ children }: { children: React.ReactNode }) {
  const [initialized, setInitialized] = useState(false)

  useEffect(() => {
    // Dynamically import and initialize i18n to avoid SSR issues
    import('@soul-player/shared/i18n').then(({ initI18n }) => {
      initI18n()
      setInitialized(true)
    })
  }, [])

  // Render children immediately - i18n will show keys until initialized
  // This is fine for the demo since it loads quickly
  if (!initialized) {
    return <>{children}</>
  }

  return <>{children}</>
}
