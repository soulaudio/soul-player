/**
 * Desktop MainLayout - wrapper around shared MainLayout
 */
import { ReactNode, useState, useCallback, useEffect } from 'react';
import { useLocation } from 'react-router-dom';
import { MainLayout as SharedMainLayout, usePlayerStore, AddToPlaylistDialog, ScrollVisibilityProvider, useScrollVisibility } from '@soul-player/shared';
import { ScanProgressIndicator } from '../components/ScanProgressIndicator';
import { WindowControls } from '../components/WindowControls';
import { useSettings } from '../contexts/SettingsContext';

interface MainLayoutProps {
  children: ReactNode;
}

function MainLayoutContent({ children }: MainLayoutProps) {
  const { currentTrack } = usePlayerStore();
  const [showAddToPlaylist, setShowAddToPlaylist] = useState(false);
  const { showHeader, setShowHeader } = useScrollVisibility();
  const { hideWindowControls } = useSettings();
  const location = useLocation();

  const handleAddToPlaylist = useCallback(() => {
    if (currentTrack) {
      setShowAddToPlaylist(true);
    }
  }, [currentTrack]);

  // Show header when navigating to a new page
  useEffect(() => {
    setShowHeader(true);
  }, [location.pathname, setShowHeader]);

  return (
    <div className="fixed inset-0 overflow-hidden">
      {/* Invisible drag region - always present at top for dragging window, doesn't take layout space */}
      <div
        data-tauri-drag-region
        className="absolute top-0 left-0 right-0 h-8 z-40 pointer-events-auto"
        style={{ WebkitAppRegion: 'drag' } as React.CSSProperties}
      />

      {/* Window controls - auto-hide on scroll (if setting enabled), positioned over drag region */}
      <div
        className={`absolute top-0 right-0 z-50 flex items-center transition-all duration-300 pointer-events-none ${
          hideWindowControls ? (showHeader ? 'translate-y-0' : '-translate-y-full') : 'translate-y-0'
        }`}
      >
        {/* Small drag region next to buttons (visible state) */}
        <div
          data-tauri-drag-region
          className="h-8 w-24 pointer-events-auto"
          style={{ WebkitAppRegion: 'drag' } as React.CSSProperties}
        />
        <div className="pointer-events-auto">
          <WindowControls />
        </div>
      </div>

      {/* Main layout - full height, sidebar stretches to top */}
      <div className="h-full w-full">
        <SharedMainLayout onAddToPlaylist={handleAddToPlaylist}>
          {children}
        </SharedMainLayout>
      </div>

      {/* Scan progress indicator (shows when scanning library sources) */}
      <ScanProgressIndicator position="footer" />

      {/* Add to Playlist dialog */}
      {currentTrack && showAddToPlaylist && (
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

export function MainLayout({ children }: MainLayoutProps) {
  return (
    <ScrollVisibilityProvider>
      <MainLayoutContent>{children}</MainLayoutContent>
    </ScrollVisibilityProvider>
  );
}
