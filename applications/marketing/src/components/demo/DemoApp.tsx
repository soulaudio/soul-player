'use client'

import { useEffect, useState } from 'react'
import { MemoryRouter, Routes, Route } from 'react-router-dom'
import { MainLayout } from '@soul-player/shared'
import { DemoPlayerCommandsProvider } from '@/providers/DemoPlayerCommandsProvider'
import { MockSettingsProvider } from './MockContexts'
import { HomePage } from './HomePage'
import { LibraryPage } from './LibraryPage'
import { NowPlayingPage } from './NowPlayingPage'
import { SettingsPage } from './SettingsPage'
import { initializeDemoStorage } from '@/lib/demo/storage'

/**
 * Demo version of the Soul Player app for marketing showcase
 * Uses real playback with demo data loaded from JSON
 * Fixed dimensions (1200x750) - will be scaled by DemoScaler
 */
export function DemoApp() {
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  // Initialize demo storage on mount
  useEffect(() => {
    initializeDemoStorage('/demo-data.json')
      .then(() => {
        setIsLoading(false)
      })
      .catch((err) => {
        console.error('Failed to initialize demo storage:', err)
        setError('Failed to load demo data')
        setIsLoading(false)
      })
  }, [])

  if (isLoading) {
    return (
      <div
        data-demo-container
        data-theme="dark"
        className="flex items-center justify-center bg-background text-foreground"
        style={{ width: 1200, height: 750 }}
      >
        <div className="text-center">
          <div className="text-lg font-medium">Loading demo...</div>
          <div className="text-sm text-muted-foreground mt-2">Preparing music player</div>
        </div>
      </div>
    )
  }

  if (error) {
    return (
      <div
        data-demo-container
        data-theme="dark"
        className="flex items-center justify-center bg-background text-foreground"
        style={{ width: 1200, height: 750 }}
      >
        <div className="text-center">
          <div className="text-lg font-medium text-destructive">Error</div>
          <div className="text-sm text-muted-foreground mt-2">{error}</div>
        </div>
      </div>
    )
  }

  return (
    <div
      data-demo-container
      data-theme="dark"
      className="bg-background text-foreground flex flex-col"
      style={{ width: 1200, height: 750 }}
    >
      <MemoryRouter initialEntries={['/']}>
        <DemoPlayerCommandsProvider>
          <MockSettingsProvider>
            <MainLayout showKeyboardShortcuts={false}>
              <Routes>
                <Route path="/" element={<HomePage />} />
                <Route path="/library" element={<LibraryPage />} />
                <Route path="/now-playing" element={<NowPlayingPage />} />
                <Route path="/settings" element={<SettingsPage />} />
                <Route path="/search" element={<div className="text-center py-20 text-muted-foreground">Search Page (Demo)</div>} />
              </Routes>
            </MainLayout>
          </MockSettingsProvider>
        </DemoPlayerCommandsProvider>
      </MemoryRouter>
    </div>
  )
}
