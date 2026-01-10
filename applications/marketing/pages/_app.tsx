import type { AppProps } from 'next/app'
import '@/styles/globals.css'
import dynamic from 'next/dynamic'

// ThemeProvider reads from localStorage, so it must be client-only to avoid hydration mismatch
const ThemeProvider = dynamic(
  () => import('@soul-player/shared').then((mod) => mod.ThemeProvider),
  { ssr: false }
)

export default function App({ Component, pageProps }: AppProps) {
  return (
    <ThemeProvider>
      <Component {...pageProps} />
    </ThemeProvider>
  )
}
