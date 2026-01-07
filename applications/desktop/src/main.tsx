import React from 'react';
import ReactDOM from 'react-dom/client';
import { BrowserRouter } from 'react-router-dom';
import { ThemeProvider } from '@soul-player/shared/theme';
import { SettingsProvider } from './contexts/SettingsContext';
import App from './App';
import './index.css';
import './i18n'; // Initialize i18n

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <BrowserRouter>
      <ThemeProvider>
        <SettingsProvider>
          <App />
        </SettingsProvider>
      </ThemeProvider>
    </BrowserRouter>
  </React.StrictMode>
);
