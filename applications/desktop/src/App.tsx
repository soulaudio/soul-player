import { Routes, Route } from 'react-router-dom';
import { MainLayout } from './layouts/MainLayout';
import { LibraryPage } from './pages/LibraryPage';
import { PlaylistsPage } from './pages/PlaylistsPage';
import { ArtistsPage } from './pages/ArtistsPage';
import { AlbumsPage } from './pages/AlbumsPage';
import { GenresPage } from './pages/GenresPage';
import { SearchPage } from './pages/SearchPage';
import { SettingsPage } from './pages/SettingsPage';

function App() {
  return (
    <MainLayout>
      <Routes>
        <Route path="/" element={<LibraryPage />} />
        <Route path="/playlists" element={<PlaylistsPage />} />
        <Route path="/artists" element={<ArtistsPage />} />
        <Route path="/albums" element={<AlbumsPage />} />
        <Route path="/genres" element={<GenresPage />} />
        <Route path="/search" element={<SearchPage />} />
        <Route path="/settings" element={<SettingsPage />} />
      </Routes>
    </MainLayout>
  );
}

export default App;
