'use client'

import { useEffect, useState, useCallback } from 'react'
import { MemoryRouter, Routes, Route } from 'react-router-dom'
import {
  MainLayout,
  initI18n,
  PlatformProvider,
  HomePage,
  LibraryPage,
  AlbumsPage,
  ArtistsPage,
  PlaylistsPage,
  TracksPage,
  AlbumPage,
  ArtistPage,
  PlaylistPage,
  NowPlayingPage,
  SettingsPage,
  MockBackendProvider,
  DemoStorage,
  ScrollVisibilityProvider,
  AddToPlaylistDialog,
  useCurrentTrack,
  QueryClient,
  QueryClientProvider,
} from '@soul-player/shared'
import { DemoPlayerCommandsProvider } from '@/providers/DemoPlayerCommandsProvider'
import { MockSettingsProvider } from './MockContexts'
import { DemoInitializer } from './DemoInitializer'

// Initialize i18n for the demo
initI18n()

// Create QueryClient instance for React Query
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 1,
      staleTime: 1000 * 60 * 5, // 5 minutes
      gcTime: 1000 * 60 * 10, // 10 minutes
    },
  },
})

/**
 * Demo version of the Soul Player app for marketing showcase
 * Uses real playback with demo data loaded from JSON
 * Fixed dimensions (1200x750) - will be scaled by DemoScaler
 */
// Singleton demo storage instance
const demoStorage = new DemoStorage()

export function DemoApp() {
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [showAddToPlaylist, setShowAddToPlaylist] = useState(false)
  const currentTrack = useCurrentTrack()

  // Initialize demo storage on mount
  useEffect(() => {
    demoStorage
      .loadFromJson('/demo-data.json')
      .then(() => {
        setIsLoading(false)
      })
      .catch((err) => {
        console.error('Failed to initialize demo storage:', err)
        setError('Failed to load demo data')
        setIsLoading(false)
      })
  }, [])

  // Handle add to playlist button click
  const handleAddToPlaylist = useCallback(() => {
    if (currentTrack) {
      setShowAddToPlaylist(true)
    }
  }, [currentTrack])

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
    <QueryClientProvider client={queryClient}>
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
              // Library features - demo supports playlists now!
              canDeleteTracks: false,
              canCreatePlaylists: true,
              hasFilters: false,
              hasHealthCheck: false,
              hasVirtualization: false,
              hasTrackMenu: true,
              hasPlaybackContext: true,
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
            <DemoPlayerCommandsProvider storage={demoStorage}>
              <MockBackendProvider storage={demoStorage}>
                <MockSettingsProvider>
                  <DemoInitializer storage={demoStorage}>
                  {/* Wrapper to ensure MainLayout fills available space */}
                  <div className="flex-1 min-h-0 h-full">
                    <ScrollVisibilityProvider>
                      <MainLayout onAddToPlaylist={handleAddToPlaylist}>
                        <Routes>
                          <Route path="/" element={<HomePage />} />
                          <Route path="/library" element={<LibraryPage />} />
                          <Route path="/albums" element={<AlbumsPage />} />
                          <Route path="/albums/:id" element={<AlbumPage />} />
                          <Route path="/artists" element={<ArtistsPage />} />
                          <Route path="/artists/:id" element={<ArtistPage />} />
                          <Route path="/playlists" element={<PlaylistsPage />} />
                          <Route path="/playlists/:id" element={<PlaylistPage />} />
                          <Route path="/tracks" element={<TracksPage />} />
                          <Route path="/now-playing" element={<NowPlayingPage />} />
                          <Route path="/settings" element={<SettingsPage />} />
                          <Route path="/search" element={<div className="text-center py-20 text-muted-foreground">Search Page (Demo)</div>} />
                        </Routes>
                      </MainLayout>
                    </ScrollVisibilityProvider>
                  </div>
                </DemoInitializer>

                {/* Add to Playlist Dialog */}
                {currentTrack && showAddToPlaylist && (
                  <AddToPlaylistDialog
                    open={showAddToPlaylist}
                    onClose={() => setShowAddToPlaylist(false)}
                    trackId={currentTrack.id}
                    trackTitle={currentTrack.title}
                  />
                )}
                </MockSettingsProvider>
              </MockBackendProvider>
            </DemoPlayerCommandsProvider>
          </PlatformProvider>
        </MemoryRouter>
      </div>
    </QueryClientProvider>
  )
}
