import React from 'react';
import ReactDOM from 'react-dom/client';
import { BrowserRouter } from 'react-router-dom';
import { ThemeProvider } from '@soul-player/shared/theme';
import { initI18n, PlatformProvider } from '@soul-player/shared';
import { SettingsProvider } from './contexts/SettingsContext';
import { TauriPlayerCommandsProvider } from './providers/TauriPlayerCommandsProvider';
import { TauriBackendProvider } from './providers/TauriBackendProvider';
import App from './App';
import './index.css';

// Initialize i18n from shared package
initI18n();

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
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
  </React.StrictMode>
);
