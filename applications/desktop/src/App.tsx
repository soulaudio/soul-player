import { useState, useEffect } from 'react';
import { Routes, Route, Navigate } from 'react-router-dom';
import { Toaster } from 'sonner';
import { useBackend, clearArtworkCache, useScanCompletionInvalidation, debug } from '@soul-player/shared';
import { MainLayout } from './layouts/MainLayout';
// Use shared pages for cross-platform parity
import {
  HomePage,
  AlbumsPage,
  ArtistsPage,
  PlaylistsPage,
  TracksPage,
  AlbumPage,
  ArtistPage,
  NowPlayingPage,
  PlaylistPage,
  GenresListPage,
} from '@soul-player/shared';
// Desktop-specific pages
import { SettingsRouter } from './pages/SettingsRouter';
import { OnboardingPage } from './pages/OnboardingPage';
import { GenrePage } from './pages/GenrePage';
import { FileDropHandler } from './components/FileDropHandler';
import { UpdateSettingsProvider } from './contexts/UpdateSettingsContext';

function HomeRoute() {
  const backend = useBackend();
  const [homeEnabled, setHomeEnabled] = useState<boolean | null>(null);

  useEffect(() => {
    const loadHomeEnabled = () => {
      backend.getUserSetting('home.enabled')
        .then((value) => {
          setHomeEnabled(value ?? true);
        })
        .catch(err => {
          debug.error('Failed to load home.enabled setting:', err);
          setHomeEnabled(true); // Default to enabled on error
        });
    };

    // Load on mount
    loadHomeEnabled();

    // Listen for changes from settings page
    const handleHomeEnabledChanged = (event: Event) => {
      const customEvent = event as CustomEvent<{ enabled: boolean }>;
      setHomeEnabled(customEvent.detail.enabled);
    };

    window.addEventListener('home-enabled-changed', handleHomeEnabledChanged);

    return () => {
      window.removeEventListener('home-enabled-changed', handleHomeEnabledChanged);
    };
  }, [backend]);

  // Show loading while checking
  if (homeEnabled === null) {
    return null; // Or a loading spinner
  }

  // Redirect to albums if home is disabled
  if (!homeEnabled) {
    return <Navigate to="/albums" replace />;
  }

  // Show home page if enabled
  return <HomePage />;
}

function App() {
  const [showOnboarding, setShowOnboarding] = useState<boolean | null>(null);
  const backend = useBackend();

  // Cache invalidation on scan/import completion (replaces ScanProgressToast)
  useScanCompletionInvalidation();

  useEffect(() => {
    // Check if onboarding is needed
    debug.log('[App] Checking onboarding status...');

    // Add timeout protection (5 seconds)
    const timeoutId = setTimeout(() => {
      debug.error('[App] Onboarding check timed out after 5s, skipping onboarding');
      setShowOnboarding(false);
    }, 5000);

    backend.checkOnboardingNeeded()
      .then((needed) => {
        clearTimeout(timeoutId);
        debug.log('[App] Onboarding check result:', needed);
        setShowOnboarding(needed);
      })
      .catch((error) => {
        clearTimeout(timeoutId);
        debug.error('[App] Onboarding check failed:', error);
        setShowOnboarding(false); // On error, skip onboarding
      });

    return () => clearTimeout(timeoutId);
  }, [backend]);

  // Listen for artwork change events from backend
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let isMounted = true;

    async function setupArtworkListener() {
      try {
        // Check if we're in Tauri environment
        if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
          const { listen } = await import('@tauri-apps/api/event');
          const { invoke } = await import('@tauri-apps/api/core');

          // Only set up listener if component is still mounted
          if (!isMounted) return;

          unlisten = await listen<{ entityType: string; entityId: string }>('artwork-changed', async (event) => {
            debug.log('[App] Artwork changed:', event.payload);
            const { entityType, entityId } = event.payload;

            // Clear the cache for this entity to force reload
            if (entityType === 'album') {
              // Clear album cache
              clearArtworkCache('album', entityId);

              // Also clear cache for all tracks in this album
              try {
                const tracks = await invoke<Array<{ id: string }>>('get_album_tracks', {
                  albumId: parseInt(entityId, 10)
                });
                tracks.forEach(track => {
                  clearArtworkCache('track', track.id);
                });
                debug.log(`[App] Cleared cache for album ${entityId} and ${tracks.length} tracks`);
              } catch (error) {
                debug.error('[App] Failed to get album tracks for cache clearing:', error);
              }
            } else if (entityType === 'artist' || entityType === 'playlist') {
              clearArtworkCache(entityType, entityId);
            }
          });

          debug.log('[App] Listening for artwork-changed events');
        }
      } catch (error) {
        debug.error('[App] Failed to set up artwork listener:', error);
      }
    }

    // Await the setup to prevent race conditions and loading cursor issues
    void setupArtworkListener();

    return () => {
      isMounted = false;
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  // Show loading while checking
  if (showOnboarding === null) {
    return (
      <div className="flex items-center justify-center h-screen bg-background">
        <div className="text-center">
          <div className="animate-spin w-8 h-8 border-4 border-primary border-t-transparent rounded-full mx-auto mb-4"></div>
          <p className="text-muted-foreground">Loading...</p>
        </div>
      </div>
    );
  }

  // Show onboarding if needed
  if (showOnboarding) {
    return <OnboardingPage onComplete={() => setShowOnboarding(false)} />;
  }

  // Normal app with file drop handler
  return (
    <UpdateSettingsProvider>
    <FileDropHandler>
      <MainLayout>
        <Routes>
          <Route path="/" element={<HomeRoute />} />
          <Route path="/albums" element={<AlbumsPage />} />
          <Route path="/albums/:id" element={<AlbumPage />} />
          <Route path="/artists" element={<ArtistsPage />} />
          <Route path="/artists/:id" element={<ArtistPage />} />
          <Route path="/playlists" element={<PlaylistsPage />} />
          <Route path="/playlists/:id" element={<PlaylistPage />} />
          <Route path="/tracks" element={<TracksPage />} />
          <Route path="/genres" element={<GenresListPage />} />
          <Route path="/genres/:id" element={<GenrePage />} />
          <Route path="/now-playing" element={<NowPlayingPage />} />
          <Route path="/settings/*" element={<SettingsRouter />} />
        </Routes>
      </MainLayout>
      <Toaster richColors position="bottom-right" />
    </FileDropHandler>
    </UpdateSettingsProvider>
  );
}

export default App;
