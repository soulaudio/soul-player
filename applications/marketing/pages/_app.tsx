import type { AppProps } from 'next/app'
import '@/styles/globals.css'
import { useEffect } from 'react'
import { themeManager } from '@soul-player/shared'

export default function App({ Component, pageProps }: AppProps) {
  useEffect(() => {
    // Initialize theme manager - it will load saved theme from localStorage
    // or apply the default theme
    themeManager.setCurrentTheme(themeManager.getCurrentTheme().id)
  }, [])

  return <Component {...pageProps} />
}
