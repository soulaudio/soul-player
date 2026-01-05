import { Routes, Route } from 'react-router-dom';
import { MobileLayout } from './layouts/MobileLayout';
import { LibraryPage } from './pages/LibraryPage';
import { PlaylistsPage } from './pages/PlaylistsPage';
import { NowPlayingPage } from './pages/NowPlayingPage';
import { SettingsPage } from './pages/SettingsPage';

function App() {
  return (
    <MobileLayout>
      <Routes>
        <Route path="/" element={<LibraryPage />} />
        <Route path="/playlists" element={<PlaylistsPage />} />
        <Route path="/now-playing" element={<NowPlayingPage />} />
        <Route path="/settings" element={<SettingsPage />} />
      </Routes>
    </MobileLayout>
  );
}

export default App;
