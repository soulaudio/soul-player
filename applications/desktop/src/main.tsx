import React from 'react';
import ReactDOM from 'react-dom/client';
import { BrowserRouter } from 'react-router-dom';
import { ThemeProvider } from '@soul-player/shared/theme';
import { initI18n, PlatformProvider, QueryClient, QueryClientProvider } from '@soul-player/shared';
import { SettingsProvider } from './contexts/SettingsContext';
import { TauriPlayerCommandsProvider } from './providers/TauriPlayerCommandsProvider';
import { TauriBackendProvider } from './providers/TauriBackendProvider';
import App from './App';
import './index.css';

// Initialize i18n from shared package
initI18n();

// Create TanStack Query client
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 60 * 5, // 5 minutes
      gcTime: 1000 * 60 * 30, // 30 minutes (formerly cacheTime)
      refetchOnWindowFocus: false,
    },
  },
});

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
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
          <TauriPlayerCommandsProvider>
            <TauriBackendProvider>
              <SettingsProvider>
                <App />
              </SettingsProvider>
            </TauriBackendProvider>
          </TauriPlayerCommandsProvider>
        </PlatformProvider>
      </ThemeProvider>
    </BrowserRouter>
    </QueryClientProvider>
  </React.StrictMode>
);
