/**
 * Desktop MainLayout - wrapper around shared MainLayout
 *
 * Platform-Specific Window Implementation:
 * - macOS: Uses native window decorations (title bar with native controls)
 * - Windows/Linux: Frameless window with custom drag region and controls
 *   - Uses data-tauri-drag-region for window dragging
 *   - Drag regions are inset from edges (left-2, right-2) to allow OS resize handles
 *   - Window is transparent mode (tauri.conf.json) for better resize performance
 *   - Double-click on drag region toggles maximize/restore
 *   - Window controls (minimize, maximize, close) positioned in top right
 *   - Layout uses flexbox for proper content resizing
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
import { useLocation, useNavigate } from 'react-router-dom';
import { MainLayout as SharedMainLayout, useCurrentTrack, AddToPlaylistDialog, ScrollVisibilityProvider, useScrollVisibility, ROOT_PATHS, debug } from '@soul-player/shared';
import { ScanProgressIndicator } from '../components/ScanProgressIndicator';
import { WindowControls } from '../components/WindowControls';
import { useSettings } from '../contexts/SettingsContext';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { platform } from '@tauri-apps/plugin-os';
import { ArrowLeft } from 'lucide-react';

/** Back button that reads sidebar width to position itself in the content area */
function BackButton({ isMobileWidth, navigate }: { isMobileWidth: boolean; navigate: ReturnType<typeof useNavigate> }) {
  const [sidebarWidth, setSidebarWidth] = useState(0);

  useEffect(() => {
    // Read sidebar width from the DOM element with data-collapsed attribute
    const update = () => {
      const sidebar = document.querySelector('[data-collapsed]') as HTMLElement | null;
      if (sidebar) {
        setSidebarWidth(sidebar.offsetWidth);
      } else {
        // Collapsed — check for the thin strip (3px)
        setSidebarWidth(3);
      }
    };
    update();
    // Watch for sidebar resize
    const observer = new MutationObserver(update);
    const resizeObserver = new ResizeObserver(update);
    const el = document.querySelector('[data-collapsed]');
    if (el) {
      observer.observe(el, { attributes: true, attributeFilter: ['style'] });
      resizeObserver.observe(el);
    }
    // Also poll briefly in case sidebar mounts late
    const timer = setTimeout(update, 200);
    return () => { observer.disconnect(); resizeObserver.disconnect(); clearTimeout(timer); };
  }, []);

  return (
    <button
      onClick={() => {
        if (isMobileWidth) {
          window.dispatchEvent(new CustomEvent('mobile-back'));
        } else {
          navigate(-1);
        }
      }}
      className="absolute top-0 z-50 flex items-center justify-center w-11 h-8 hover:bg-foreground/[var(--hover-bg-opacity)] transition-colors duration-[var(--transition-duration)] pointer-events-auto"
      style={{ left: `${sidebarWidth}px`, WebkitAppRegion: 'no-drag' } as React.CSSProperties}
      aria-label="Back"
      data-testid="back-button"
    >
      <ArrowLeft className="w-4 h-4 text-muted-foreground" />
    </button>
  );
}

interface MainLayoutProps {
  children: ReactNode;
}

function MainLayoutContent({ children }: MainLayoutProps) {
  const currentTrack = useCurrentTrack();
  const [showAddToPlaylist, setShowAddToPlaylist] = useState(false);
  const { showHeader, setShowHeader } = useScrollVisibility();
  const { hideWindowControls } = useSettings();
  const location = useLocation();
  const navigate = useNavigate();
  const appWindow = getCurrentWindow();

  // Check if we're on macOS (uses native decorations)
  const isMacOS = platform() === 'macos';

  // Track mobile content visibility reactively
  const [mobileContentVisible, setMobileContentVisible] = useState(false);
  const isMobileWidth = typeof window !== 'undefined' && window.innerWidth < 640;

  useEffect(() => {
    if (!isMobileWidth) return;
    const observer = new MutationObserver(() => {
      setMobileContentVisible(document.documentElement.hasAttribute('data-mobile-content'));
    });
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ['data-mobile-content'] });
    // Sync initial state
    setMobileContentVisible(document.documentElement.hasAttribute('data-mobile-content'));
    return () => observer.disconnect();
  }, [isMobileWidth]);

  // Back button — on detail pages (desktop) or on mobile when content is showing
  const canGoBack = isMobileWidth ? mobileContentVisible : !ROOT_PATHS.includes(location.pathname);

  const handleAddToPlaylist = useCallback(() => {
    if (currentTrack) {
      setShowAddToPlaylist(true);
    }
  }, [currentTrack]);

  // Handle double-click on drag region to maximize/restore window (Windows/Linux only)
  const handleDragRegionDoubleClick = useCallback(async () => {
    if (isMacOS) return; // Native title bar handles this on macOS

    try {
      await appWindow.toggleMaximize();
    } catch (err) {
      debug.error('Failed to toggle maximize:', err);
    }
  }, [appWindow, isMacOS]);

  // Show header when navigating to a new page
  useEffect(() => {
    setShowHeader(true);
  }, [location.pathname, setShowHeader]);

  return (
    <div className="fixed inset-0 overflow-hidden bg-background">
      {/* Custom title bar for Windows/Linux only - macOS uses native decorations */}
      {!isMacOS && (
        <>
          {/* Invisible drag region - inset from edges to allow OS resize handles */}
          <div
            data-tauri-drag-region
            className="absolute top-0 left-2 right-2 h-8 z-40 pointer-events-auto"
            style={{ WebkitAppRegion: 'drag' } as React.CSSProperties}
            onDoubleClick={handleDragRegionDoubleClick}
          />

          {/* Back button — positioned in content area (to the right of sidebar) */}
          {canGoBack && (
            <BackButton isMobileWidth={isMobileWidth} navigate={navigate} />
          )}

          {/* Window controls - auto-hide on scroll (if setting enabled), positioned over drag region */}
          <div
            className={`absolute top-0 right-0 z-50 flex items-center transition-all duration-300 pointer-events-none ${
              showHeader ? 'translate-y-0 opacity-100' : hideWindowControls ? '-translate-y-full' : 'opacity-30'
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
        </>
      )}

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
          mode="track"
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
