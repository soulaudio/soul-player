import { useState, useEffect } from 'react';
import { Routes, Route } from 'react-router-dom';
import { useBackend } from '@soul-player/shared';
import { MainLayout } from './layouts/MainLayout';
// Use shared pages for cross-platform parity
import {
  HomePage,
  LibraryPage,
  AlbumsPage,
  ArtistsPage,
  PlaylistsPage,
  TracksPage,
  AlbumPage,
  ArtistPage,
  NowPlayingPage,
  PlaylistPage,
} from '@soul-player/shared';
// Desktop-specific pages
import { SettingsPage } from './pages/SettingsPage';
import { OnboardingPage } from './pages/OnboardingPage';
import { GenrePage } from './pages/GenrePage';
import { FileDropHandler } from './components/FileDropHandler';

function App() {
  const [showOnboarding, setShowOnboarding] = useState<boolean | null>(null);
  const backend = useBackend();

  useEffect(() => {
    // Check if onboarding is needed
    backend.checkOnboardingNeeded()
      .then(setShowOnboarding)
      .catch(() => setShowOnboarding(false)); // On error, skip onboarding
  }, [backend]);

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
          <Route path="/albums" element={<AlbumsPage />} />
          <Route path="/albums/:id" element={<AlbumPage />} />
          <Route path="/artists" element={<ArtistsPage />} />
          <Route path="/artists/:id" element={<ArtistPage />} />
          <Route path="/playlists" element={<PlaylistsPage />} />
          <Route path="/playlists/:id" element={<PlaylistPage />} />
          <Route path="/tracks" element={<TracksPage />} />
          <Route path="/genres/:id" element={<GenrePage />} />
          <Route path="/now-playing" element={<NowPlayingPage />} />
          <Route path="/settings" element={<SettingsPage />} />
        </Routes>
      </MainLayout>
    </FileDropHandler>
  );
}

export default App;
