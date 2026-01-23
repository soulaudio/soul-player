import { Routes, Route, Navigate } from 'react-router-dom';
import {
  ThemeProvider,
  PlatformProvider,
  MockBackendProvider,
  ServerBackendProvider,
  DemoStorage,
  HomePage,
  LibraryPage,
  AlbumsPage,
  ArtistsPage,
  PlaylistsPage,
  AlbumPage,
  ArtistPage,
  PlaylistPage,
  SettingsPage,
  MainLayout,
  useBackend,
} from '@soul-player/shared';
import { useState, useEffect } from 'react';
import { AuthProvider, useAuth } from './providers/AuthProvider';
import { WebPlayerCommandsProvider } from './providers/WebPlayerCommandsProvider';
import { LoginPage } from './pages/LoginPage';

type AppMode = 'demo' | 'server';

// Demo storage singleton
const demoStorage = new DemoStorage();

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
          console.error('Failed to load home.enabled setting:', err);
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

function DemoModeApp() {
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    // Load demo data
    demoStorage
      .loadFromJson('/demo-data.json')
      .then(() => setIsLoading(false))
      .catch((err) => {
        console.error('Failed to load demo data:', err);
        setError('Failed to load demo data');
        setIsLoading(false);
      });
  }, []);

  if (isLoading) {
    return (
      <div className="flex items-center justify-center min-h-screen bg-background">
        <div className="text-muted-foreground">Loading demo...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center min-h-screen bg-background">
        <div className="text-center">
          <div className="text-destructive">{error}</div>
          <button
            onClick={() => window.location.reload()}
            className="mt-4 px-4 py-2 bg-primary text-primary-foreground rounded"
          >
            Retry
          </button>
        </div>
      </div>
    );
  }

  return (
    <MockBackendProvider storage={demoStorage}>
      <WebPlayerCommandsProvider>
        <MainLayout>
          <Routes>
            <Route path="/" element={<HomeRoute />} />
            <Route path="/library" element={<LibraryPage />} />
            <Route path="/albums" element={<AlbumsPage />} />
            <Route path="/albums/:id" element={<AlbumPage />} />
            <Route path="/artists" element={<ArtistsPage />} />
            <Route path="/artists/:id" element={<ArtistPage />} />
            <Route path="/playlists" element={<PlaylistsPage />} />
            <Route path="/playlists/:id" element={<PlaylistPage />} />
            <Route path="/settings" element={<SettingsPage />} />
          </Routes>
        </MainLayout>
      </WebPlayerCommandsProvider>
    </MockBackendProvider>
  );
}

function ServerModeApp() {
  const { isAuthenticated, isLoading, token } = useAuth();
  const apiBase = import.meta.env.VITE_API_URL || '/api';

  if (isLoading) {
    return (
      <div className="flex items-center justify-center min-h-screen bg-background">
        <div className="text-muted-foreground">Loading...</div>
      </div>
    );
  }

  if (!isAuthenticated) {
    return <Navigate to="/login" replace />;
  }

  return (
    <ServerBackendProvider apiBase={apiBase} authToken={token}>
      <WebPlayerCommandsProvider>
        <MainLayout>
          <Routes>
            <Route path="/" element={<HomeRoute />} />
            <Route path="/library" element={<LibraryPage />} />
            <Route path="/albums" element={<AlbumsPage />} />
            <Route path="/albums/:id" element={<AlbumPage />} />
            <Route path="/artists" element={<ArtistsPage />} />
            <Route path="/artists/:id" element={<ArtistPage />} />
            <Route path="/playlists" element={<PlaylistsPage />} />
            <Route path="/playlists/:id" element={<PlaylistPage />} />
            <Route path="/settings" element={<SettingsPage />} />
          </Routes>
        </MainLayout>
      </WebPlayerCommandsProvider>
    </ServerBackendProvider>
  );
}

function ModeSelector({ onSelect }: { onSelect: (mode: AppMode) => void }) {
  return (
    <div className="flex items-center justify-center min-h-screen bg-background">
      <div className="text-center space-y-6 max-w-md">
        <h1 className="text-3xl font-bold">Welcome to Soul Player</h1>
        <p className="text-muted-foreground">
          Choose how you want to experience the app
        </p>
        <div className="space-y-4">
          <button
            onClick={() => onSelect('demo')}
            className="w-full px-6 py-4 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition"
          >
            <div className="font-semibold">Try Demo</div>
            <div className="text-sm opacity-80">
              Explore the app with sample music
            </div>
          </button>
          <button
            onClick={() => onSelect('server')}
            className="w-full px-6 py-4 border border-border rounded-lg hover:bg-accent transition"
          >
            <div className="font-semibold">Sign In</div>
            <div className="text-sm text-muted-foreground">
              Connect to your Soul Player server
            </div>
          </button>
        </div>
      </div>
    </div>
  );
}

function App() {
  const [mode, setMode] = useState<AppMode | null>(null);

  // Check for stored mode preference
  useEffect(() => {
    const stored = localStorage.getItem('soul-player-mode');
    if (stored === 'demo' || stored === 'server') {
      setMode(stored);
    }
  }, []);

  const selectMode = (selectedMode: AppMode) => {
    localStorage.setItem('soul-player-mode', selectedMode);
    setMode(selectedMode);
  };

  return (
    <ThemeProvider>
      <PlatformProvider
        platform="web"
        features={{
          // Library features
          canDeleteTracks: mode === 'server',
          canCreatePlaylists: mode === 'server',
          hasFilters: mode === 'server',
          hasHealthCheck: mode === 'server',
          hasVirtualization: false,
          hasTrackMenu: mode === 'server',
          hasPlaybackContext: mode === 'server',
          // Settings features
          hasLibrarySettings: mode === 'server',
          hasAudioSettings: false,
          hasShortcutSettings: false,
          hasUpdateSettings: false,
          hasLanguageSettings: mode === 'server',
          hasThemeImportExport: false,
          // Audio features
          hasRealAudioDevices: false,
          hasRealDeviceSelection: false,
        }}
      >
        {mode === null && <ModeSelector onSelect={selectMode} />}
        {mode === 'demo' && (
          <Routes>
            <Route path="/*" element={<DemoModeApp />} />
          </Routes>
        )}
        {mode === 'server' && (
          <AuthProvider>
            <Routes>
              <Route path="/login" element={<LoginPage />} />
              <Route path="/*" element={<ServerModeApp />} />
            </Routes>
          </AuthProvider>
        )}
      </PlatformProvider>
    </ThemeProvider>
  );
}

export default App;
