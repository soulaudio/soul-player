'use client'

import { useEffect, useRef, useState } from 'react'
import { MemoryRouter, Routes, Route } from 'react-router-dom'
import {
  AlbumsPage,
  initI18n,
  PlatformProvider,
  MockBackendProvider,
  DemoStorage,
  QueryClient,
  QueryClientProvider,
  ScrollVisibilityProvider,
  PlaybackSessionProvider,
  WebPlaybackProvider,
} from '@soul-player/shared'
import { DemoModeWrapper } from '../DemoModeWrapper'
import { DemoScaler } from '../demo/DemoScaler'
import { Lock, HardDrive, ShieldCheck } from 'lucide-react'

// Initialize i18n
initI18n()

// Create QueryClient instance
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 1,
      staleTime: 1000 * 60 * 5,
      gcTime: 1000 * 60 * 10,
    },
  },
})

// Singleton demo storage instance
const demoStorage = new DemoStorage()

/**
 * 3D rotating showcase component for "Actually YOUR Music" feature
 * Displays a real music library interface with privacy indicators
 */
export function LocalFirstShowcase() {
  const [isLoading, setIsLoading] = useState(true)
  const [rotateX, setRotateX] = useState(0)
  const [rotateY, setRotateY] = useState(0)
  const showcaseRef = useRef<HTMLDivElement>(null)

  // Initialize demo storage
  useEffect(() => {
    demoStorage
      .loadFromJson('/demo-data.json')
      .then(() => setIsLoading(false))
      .catch((err) => {
        console.error('Failed to initialize demo storage:', err)
        setIsLoading(false)
      })
  }, [])

  // Handle scroll-based 3D rotation
  useEffect(() => {
    const handleScroll = () => {
      if (!showcaseRef.current) return

      const rect = showcaseRef.current.getBoundingClientRect()
      const viewportHeight = window.innerHeight

      // Only rotate when element is in viewport
      if (rect.top < viewportHeight && rect.bottom > 0) {
        // Calculate scroll progress (0 to 1) through the viewport
        const progress = Math.max(0, Math.min(1,
          (viewportHeight - rect.top) / (viewportHeight + rect.height)
        ))

        // Rotate from initial angle to flat as user scrolls
        const maxRotateX = 15 // Start with 15deg tilt
        const maxRotateY = -8 // Slight Y-axis rotation

        setRotateX(maxRotateX * (1 - progress))
        setRotateY(maxRotateY * (1 - progress))
      }
    }

    handleScroll() // Initial calculation
    window.addEventListener('scroll', handleScroll, { passive: true })
    return () => window.removeEventListener('scroll', handleScroll)
  }, [])

  return (
    <section className="py-24 bg-background relative overflow-hidden">
      {/* Background gradient */}
      <div
        className="absolute inset-0 opacity-30"
        style={{
          background: 'radial-gradient(ellipse 80% 60% at 50% 50%, hsl(var(--primary) / 0.2) 0%, transparent 70%)',
        }}
      />

      <div className="container mx-auto px-6 relative z-10">
        {/* Section header */}
        <div className="text-center mb-16">
          <h2 className="text-4xl sm:text-5xl font-serif font-bold mb-4 text-foreground">
            Actually <span className="text-primary">YOUR</span> Music
          </h2>
          <p className="text-xl text-muted-foreground max-w-2xl mx-auto">
            Your files, your device, your privacy. No cloud required.
          </p>
        </div>

        {/* 3D Showcase */}
        <div
          ref={showcaseRef}
          className="max-w-5xl mx-auto mb-16"
          style={{
            perspective: '2000px',
          }}
        >
          <div
            className="relative transition-transform duration-700 ease-out"
            style={{
              transform: `rotateX(${rotateX}deg) rotateY(${rotateY}deg)`,
              transformStyle: 'preserve-3d',
            }}
          >
            {/* Glow effect around edges */}
            <div
              className="absolute -inset-4 rounded-2xl opacity-50 blur-2xl transition-opacity duration-700"
              style={{
                background: `linear-gradient(135deg,
                  hsl(var(--primary) / 0.3) 0%,
                  hsl(var(--primary) / 0.15) 50%,
                  hsl(var(--accent) / 0.2) 100%)`,
              }}
            />

            {/* Main container */}
            <div className="relative rounded-xl sm:rounded-2xl overflow-hidden border-2 shadow-2xl backdrop-blur-sm"
              style={{
                borderColor: 'hsl(var(--border))',
                backgroundColor: 'hsl(var(--card) / 0.5)',
              }}
            >
              <DemoModeWrapper interactive={false} className="w-full aspect-[16/10]">
                <DemoScaler designWidth={1200} designHeight={750} minScale={0.25}>
                  <QueryClientProvider client={queryClient}>
                    <div
                      data-demo-container
                      data-theme="dark"
                      className="bg-background text-foreground flex flex-col overflow-hidden"
                      style={{ width: 1200, height: 750 }}
                    >
                      {isLoading ? (
                        <div className="flex items-center justify-center w-full h-full">
                          <div className="text-center">
                            <div className="text-lg font-medium">Loading library...</div>
                            <div className="text-sm text-muted-foreground mt-2">Preparing showcase</div>
                          </div>
                        </div>
                      ) : (
                        <MemoryRouter initialEntries={['/albums']}>
                          <PlatformProvider
                            platform="web"
                            features={{
                              canDeleteTracks: false,
                              canCreatePlaylists: false,
                              hasFilters: false,
                              hasHealthCheck: false,
                              hasVirtualization: false,
                              hasTrackMenu: false,
                              hasPlaybackContext: true,
                              hasLibrarySettings: false,
                              hasAudioSettings: false,
                              hasShortcutSettings: false,
                              hasUpdateSettings: false,
                              hasLanguageSettings: false,
                              hasThemeImportExport: false,
                              hasRealAudioDevices: false,
                              hasRealDeviceSelection: false,
                            }}
                          >
                            <MockBackendProvider storage={demoStorage} version="0.1.0">
                              <PlaybackSessionProvider>
                                <WebPlaybackProvider storage={demoStorage}>
                                  <ScrollVisibilityProvider>
                                    <div className="flex-1 min-h-0 h-full p-6">
                                      <Routes>
                                        <Route path="/albums" element={<AlbumsPage />} />
                                      </Routes>
                                    </div>
                                  </ScrollVisibilityProvider>
                                </WebPlaybackProvider>
                              </PlaybackSessionProvider>
                            </MockBackendProvider>
                          </PlatformProvider>
                        </MemoryRouter>
                      )}
                    </div>
                  </QueryClientProvider>
                </DemoScaler>
              </DemoModeWrapper>

              {/* Privacy overlay indicators - semi-transparent */}
              <div className="absolute inset-0 pointer-events-none">
                {/* Top-left: Interactive demo badge */}
                <div className="absolute top-6 left-6 bg-background/90 backdrop-blur-md border border-border rounded-lg px-4 py-2 shadow-lg">
                  <div className="flex items-center gap-2 text-sm">
                    <HardDrive className="w-4 h-4 text-primary" />
                    <span className="font-medium text-foreground">Interactive Demo</span>
                    <span className="text-xs text-muted-foreground ml-2">Click to play</span>
                  </div>
                </div>

                {/* Top-right: Privacy badge */}
                <div className="absolute top-6 right-6 bg-background/90 backdrop-blur-md border border-border rounded-lg px-4 py-2 shadow-lg">
                  <div className="flex items-center gap-2 text-sm">
                    <Lock className="w-4 h-4 text-green-500" />
                    <span className="font-medium text-foreground">100% Private</span>
                  </div>
                </div>

                {/* Bottom-right: No cloud indicator */}
                <div className="absolute bottom-6 right-6 bg-background/90 backdrop-blur-md border border-border rounded-lg px-4 py-2 shadow-lg">
                  <div className="flex items-center gap-2 text-sm">
                    <ShieldCheck className="w-4 h-4 text-blue-500" />
                    <span className="font-medium text-foreground">No Cloud Sync</span>
                    <span className="text-xs text-muted-foreground ml-2">Zero tracking</span>
                  </div>
                </div>

                {/* File path indicators - subtle overlay */}
                <div className="absolute bottom-6 left-6 bg-background/80 backdrop-blur-sm border border-border/50 rounded px-3 py-1.5 shadow-md">
                  <div className="text-xs font-mono text-muted-foreground">
                    /Users/you/Music/Artist/Album/01-track.flac
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* Feature highlights */}
        <div className="grid md:grid-cols-3 gap-8 max-w-4xl mx-auto">
          <div className="text-center">
            <div className="w-12 h-12 mx-auto mb-4 rounded-full bg-primary/10 flex items-center justify-center">
              <HardDrive className="w-6 h-6 text-primary" />
            </div>
            <h3 className="text-lg font-bold mb-2 text-foreground">Your Files Only</h3>
            <p className="text-sm text-muted-foreground">
              All music stays on your device. No uploads, no cloud storage, no middleman.
            </p>
          </div>

          <div className="text-center">
            <div className="w-12 h-12 mx-auto mb-4 rounded-full bg-green-500/10 flex items-center justify-center">
              <Lock className="w-6 h-6 text-green-500" />
            </div>
            <h3 className="text-lg font-bold mb-2 text-foreground">Zero Tracking</h3>
            <p className="text-sm text-muted-foreground">
              No analytics, no telemetry, no data collection. What you play is your business.
            </p>
          </div>

          <div className="text-center">
            <div className="w-12 h-12 mx-auto mb-4 rounded-full bg-blue-500/10 flex items-center justify-center">
              <ShieldCheck className="w-6 h-6 text-blue-500" />
            </div>
            <h3 className="text-lg font-bold mb-2 text-foreground">Always Accessible</h3>
            <p className="text-sm text-muted-foreground">
              No internet? No problem. Your library works offline, always.
            </p>
          </div>
        </div>
      </div>
    </section>
  )
}
