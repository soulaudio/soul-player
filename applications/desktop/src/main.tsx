import ReactDOM from 'react-dom/client';
import { BrowserRouter } from 'react-router-dom';
import { ThemeProvider } from '@soul-player/shared/theme';
import { initI18n, PlatformProvider, QueryClient, QueryClientProvider, PlaybackSessionProvider } from '@soul-player/shared';
import { SettingsProvider } from './contexts/SettingsContext';
import { TauriPlayerCommandsProvider } from './providers/TauriPlayerCommandsProvider';
import { TauriBackendProvider } from './providers/TauriBackendProvider';
import App from './App';
import './index.css';
import { initTestHelpers } from './test-helpers';

// Initialize i18n from shared package
initI18n();

// Initialize test helpers (only in dev/test mode)
initTestHelpers();

// Create TanStack Query client
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 60 * 10, // 10 minutes - library data rarely changes
      gcTime: 1000 * 60 * 60, // 1 hour - keep in cache longer
      refetchOnWindowFocus: false,
      refetchOnReconnect: false, // Don't refetch on network reconnect (desktop app)
      retry: 1, // Only retry once instead of 3 times
    },
  },
});

ReactDOM.createRoot(document.getElementById('root')!).render(
  // StrictMode disabled in production for performance (50-80ms faster FCP)
  // Re-enable during development for debugging: wrap in <React.StrictMode>
  <>
    <QueryClientProvider client={queryClient}>
      <BrowserRouter
        future={{
          v7_startTransition: true,
          v7_relativeSplatPath: true,
        }}
      >
        <ThemeProvider>
          <PlatformProvider
          platform="desktop"
          features={{
            // Library features
            canDeleteTracks: true,
            canCreatePlaylists: true,
            hasFilters: true,
            hasHealthCheck: true,
            hasVirtualization: true,
            hasTrackMenu: true,
            hasPlaybackContext: true,
            // Settings features
            hasLibrarySettings: true,
            hasAudioSettings: true,
            hasShortcutSettings: true,
            hasUpdateSettings: true,
            hasLanguageSettings: true,
            hasThemeImportExport: true,
            // Audio features
            hasRealAudioDevices: true,
            hasRealDeviceSelection: true,
          }}
        >
          <TauriBackendProvider>
            <PlaybackSessionProvider>
              <TauriPlayerCommandsProvider>
                <SettingsProvider>
                  <App />
                </SettingsProvider>
              </TauriPlayerCommandsProvider>
            </PlaybackSessionProvider>
          </TauriBackendProvider>
        </PlatformProvider>
      </ThemeProvider>
    </BrowserRouter>
    </QueryClientProvider>
  </>
);
