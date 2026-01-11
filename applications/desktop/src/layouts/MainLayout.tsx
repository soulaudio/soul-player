/**
 * Desktop MainLayout - wrapper around shared MainLayout
 */
import { ReactNode, useState, useCallback } from 'react';
import { MainLayout as SharedMainLayout, usePlayerStore } from '@soul-player/shared';
import { ScanProgressIndicator } from '../components/ScanProgressIndicator';
import { AddToPlaylistDialog } from '../components/AddToPlaylistDialog';

interface MainLayoutProps {
  children: ReactNode;
}

export function MainLayout({ children }: MainLayoutProps) {
  const { currentTrack } = usePlayerStore();
  const [showAddToPlaylist, setShowAddToPlaylist] = useState(false);

  const handleAddToPlaylist = useCallback(() => {
    if (currentTrack) {
      setShowAddToPlaylist(true);
    }
  }, [currentTrack]);

  return (
    <div className="h-screen">
      <SharedMainLayout onAddToPlaylist={handleAddToPlaylist}>
        {children}
      </SharedMainLayout>

      {/* Scan progress indicator (shows when scanning library sources) */}
      <ScanProgressIndicator position="footer" />

      {/* Add to Playlist dialog */}
      {currentTrack && (
        <AddToPlaylistDialog
          open={showAddToPlaylist}
          onClose={() => setShowAddToPlaylist(false)}
          trackId={currentTrack.id}
          trackTitle={currentTrack.title}
        />
      )}
    </div>
  );
}
