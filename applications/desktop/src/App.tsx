import { useState, useEffect } from 'react';
import { Routes, Route } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import { MainLayout } from './layouts/MainLayout';
import { HomePage } from './pages/HomePage';
import { LibraryPage } from './pages/LibraryPage';
import { NowPlayingPage } from './pages/NowPlayingPage';
import { SearchPage } from './pages/SearchPage';
import { SettingsPage } from './pages/SettingsPage';
import { OnboardingPage } from './pages/OnboardingPage';
import { ArtistPage } from './pages/ArtistPage';
import { AlbumPage } from './pages/AlbumPage';
import { GenrePage } from './pages/GenrePage';
import { PlaylistPage } from './pages/PlaylistPage';
import { FileDropHandler } from './components/FileDropHandler';

function App() {
  const [showOnboarding, setShowOnboarding] = useState<boolean | null>(null);

  useEffect(() => {
    // Check if onboarding is needed
    invoke<boolean>('check_onboarding_needed')
      .then(setShowOnboarding)
      .catch(() => setShowOnboarding(false)); // On error, skip onboarding
  }, []);

  // Show nothing while checking
  if (showOnboarding === null) {
    return null;
  }

  // Show onboarding if needed
  if (showOnboarding) {
    return <OnboardingPage onComplete={() => setShowOnboarding(false)} />;
  }

  // Normal app with file drop handler
  return (
    <FileDropHandler>
      <MainLayout>
        <Routes>
          <Route path="/" element={<HomePage />} />
          <Route path="/library" element={<LibraryPage />} />
          <Route path="/artists/:id" element={<ArtistPage />} />
          <Route path="/albums/:id" element={<AlbumPage />} />
          <Route path="/genres/:id" element={<GenrePage />} />
          <Route path="/playlists/:id" element={<PlaylistPage />} />
          <Route path="/now-playing" element={<NowPlayingPage />} />
          <Route path="/search" element={<SearchPage />} />
          <Route path="/settings" element={<SettingsPage />} />
        </Routes>
      </MainLayout>
    </FileDropHandler>
  );
}

export default App;
