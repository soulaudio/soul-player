/**
 * Desktop MainLayout - wrapper around shared MainLayout
 */
import { ReactNode, useState, useCallback } from 'react';
import { MainLayout as SharedMainLayout, usePlayerStore } from '@soul-player/shared';
import { ScanProgressIndicator } from '../components/ScanProgressIndicator';
import { AddToPlaylistDialog } from '../components/AddToPlaylistDialog';
import { WindowControls } from '../components/WindowControls';

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
    <div className="relative h-screen">
      {/* Draggable region at the very top - spans full width */}
      <div
        data-tauri-drag-region
        className="absolute top-0 left-0 right-0 h-8 z-40"
        style={{ WebkitAppRegion: 'drag' } as React.CSSProperties}
      />

      {/* Window controls - top right corner, above drag region */}
      <div className="absolute top-0 right-0 z-50">
        <WindowControls />
      </div>

      {/* Main layout - full height, sidebar stretches to top */}
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
