/**
 * Desktop MainLayout - wrapper around shared MainLayout
 *
 * Frameless Window Implementation:
 * - Uses data-tauri-drag-region for window dragging
 * - Drag regions are inset from edges (left-2, right-2) to allow OS resize handles
 * - Window is transparent mode (tauri.conf.json) for better resize performance
 * - Double-click on drag region toggles maximize/restore
 * - Window controls (minimize, maximize, close) positioned in top right
 * - Layout uses flexbox for proper content resizing
 *
 * Performance Optimizations:
 * - transparent: true in config eliminates black/white bars during resize
 * - bg-background provides solid color (no visual transparency)
 * - Drag region doesn't block corner resize handles
 *
 * References:
 * - https://github.com/tauri-apps/tauri/issues/13270 (transparent mode performance)
 * - https://github.com/tauri-apps/tauri/issues/3040 (resize handle size)
 * - https://v2.tauri.app/learn/window-customization/ (official docs)
 */
import { ReactNode, useState, useCallback, useEffect } from 'react';
import { useLocation } from 'react-router-dom';
import { MainLayout as SharedMainLayout, usePlayerStore, AddToPlaylistDialog, ScrollVisibilityProvider, useScrollVisibility } from '@soul-player/shared';
import { ScanProgressIndicator } from '../components/ScanProgressIndicator';
import { WindowControls } from '../components/WindowControls';
import { useSettings } from '../contexts/SettingsContext';
import { getCurrentWindow } from '@tauri-apps/api/window';

interface MainLayoutProps {
  children: ReactNode;
}

function MainLayoutContent({ children }: MainLayoutProps) {
  const { currentTrack } = usePlayerStore();
  const [showAddToPlaylist, setShowAddToPlaylist] = useState(false);
  const { showHeader, setShowHeader } = useScrollVisibility();
  const { hideWindowControls } = useSettings();
  const location = useLocation();
  const appWindow = getCurrentWindow();

  const handleAddToPlaylist = useCallback(() => {
    if (currentTrack) {
      setShowAddToPlaylist(true);
    }
  }, [currentTrack]);

  // Handle double-click on drag region to maximize/restore window
  const handleDragRegionDoubleClick = useCallback(async () => {
    try {
      await appWindow.toggleMaximize();
    } catch (err) {
      console.error('Failed to toggle maximize:', err);
    }
  }, [appWindow]);

  // Show header when navigating to a new page
  useEffect(() => {
    setShowHeader(true);
  }, [location.pathname, setShowHeader]);

  return (
    <div className="fixed inset-0 overflow-hidden bg-background">
      {/* Invisible drag region - inset from edges to allow OS resize handles */}
      <div
        data-tauri-drag-region
        className="absolute top-0 left-2 right-2 h-8 z-40 pointer-events-auto"
        style={{ WebkitAppRegion: 'drag' } as React.CSSProperties}
        onDoubleClick={handleDragRegionDoubleClick}
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
          onDoubleClick={handleDragRegionDoubleClick}
        />
        <div className="pointer-events-auto">
          <WindowControls />
        </div>
      </div>

      {/* Main layout - full height with proper resizing */}
      <div className="h-full w-full flex flex-col">
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
