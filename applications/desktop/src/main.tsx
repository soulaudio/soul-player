import React from 'react';
import ReactDOM from 'react-dom/client';
import { BrowserRouter } from 'react-router-dom';
import { ThemeProvider } from '@soul-player/shared/theme';
import { SettingsProvider } from './contexts/SettingsContext';
import { TauriPlayerCommandsProvider } from './providers/TauriPlayerCommandsProvider';
import App from './App';
import './index.css';
import './i18n'; // Initialize i18n

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <BrowserRouter>
      <ThemeProvider>
        <TauriPlayerCommandsProvider>
          <SettingsProvider>
            <App />
          </SettingsProvider>
        </TauriPlayerCommandsProvider>
      </ThemeProvider>
    </BrowserRouter>
  </React.StrictMode>
);
