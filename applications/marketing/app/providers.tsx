'use client'

import dynamic from 'next/dynamic'

// Completely skip SSR for ThemeProvider - it reads localStorage
const SharedThemeProvider = dynamic(
  () => import('@soul-player/shared').then((mod) => mod.ThemeProvider),
  { ssr: false }
)

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  return <SharedThemeProvider>{children}</SharedThemeProvider>
}
