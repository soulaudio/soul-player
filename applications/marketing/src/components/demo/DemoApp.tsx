'use client'

import { useEffect, useState } from 'react'
import { MemoryRouter, Routes, Route } from 'react-router-dom'
import {
  MainLayout,
  initI18n,
  PlatformProvider,
  HomePage,
  LibraryPage,
  AlbumPage,
  ArtistPage,
  PlaylistPage,
  NowPlayingPage,
  SettingsPage,
} from '@soul-player/shared'
import { DemoPlayerCommandsProvider } from '@/providers/DemoPlayerCommandsProvider'
import { DemoBackendProvider } from '@/providers/DemoBackendProvider'
import { MockSettingsProvider } from './MockContexts'
import { DemoInitializer } from './DemoInitializer'
import { initializeDemoStorage } from '@/lib/demo/storage'

// Initialize i18n for the demo
initI18n()

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
      className="bg-background text-foreground flex flex-col overflow-hidden"
      style={{ width: 1200, height: 750 }}
    >
      <MemoryRouter initialEntries={['/']}>
        <PlatformProvider
          platform="web"
          features={{
            // Library features - disabled for web demo
            canDeleteTracks: false,
            canCreatePlaylists: false,
            hasFilters: false,
            hasHealthCheck: false,
            hasVirtualization: false,
            hasTrackMenu: false,
            hasPlaybackContext: false,
            // Settings features - disabled for web demo
            hasLibrarySettings: false,
            hasAudioSettings: false,
            hasShortcutSettings: false,
            hasUpdateSettings: false,
            hasLanguageSettings: false,
            hasThemeImportExport: false,
            // Audio features - disabled for web demo
            hasRealAudioDevices: false,
            hasRealDeviceSelection: false,
          }}
        >
          <DemoPlayerCommandsProvider>
            <DemoBackendProvider>
              <MockSettingsProvider>
                <DemoInitializer>
                  {/* Wrapper to ensure MainLayout fills available space */}
                  <div className="flex-1 min-h-0 h-full">
                    <MainLayout>
                      <Routes>
                        <Route path="/" element={<HomePage />} />
                        <Route path="/library" element={<LibraryPage />} />
                        <Route path="/albums/:id" element={<AlbumPage />} />
                        <Route path="/artists/:id" element={<ArtistPage />} />
                        <Route path="/playlists/:id" element={<PlaylistPage />} />
                        <Route path="/now-playing" element={<NowPlayingPage />} />
                        <Route path="/settings" element={<SettingsPage />} />
                        <Route path="/search" element={<div className="text-center py-20 text-muted-foreground">Search Page (Demo)</div>} />
                      </Routes>
                    </MainLayout>
                  </div>
                </DemoInitializer>
              </MockSettingsProvider>
            </DemoBackendProvider>
          </DemoPlayerCommandsProvider>
        </PlatformProvider>
      </MemoryRouter>
    </div>
  )
}
