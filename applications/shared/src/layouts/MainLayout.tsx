import { ReactNode, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { LeftSidebar } from '../components/LeftSidebar';
import { NowPlayingFloating } from '../components/NowPlayingFloating';
import { SidebarStateProvider } from '../contexts/SidebarStateContext';
import { useSidebarState } from '../contexts/SidebarStateContext';
import { useScrollVisibility } from '../contexts/ScrollVisibilityContext';

// Inner component — must live inside SidebarStateProvider to read context
function MainContent({ children, showHeader }: { children: ReactNode; showHeader: boolean }) {
  const { isCollapsed } = useSidebarState();
  return (
    <main className="flex-1 overflow-hidden">
      <div className={`h-full pl-6 pr-0 transition-all duration-300 ${showHeader ? 'pt-6' : 'pt-0'} ${isCollapsed ? 'pb-[72px]' : ''}`}>
        {children}
      </div>
    </main>
  );
}

interface MainLayoutProps {
  children: ReactNode;
  /** Callback when the "Add to Playlist" button is clicked in the sidebar */
  onAddToPlaylist?: () => void;
}

export function MainLayout({ children, onAddToPlaylist }: MainLayoutProps) {
  const navigate = useNavigate();

  let showHeader = true;
  try {
    const context = useScrollVisibility();
    showHeader = context.showHeader;
  } catch {
    showHeader = true;
  }

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === '1') {
        e.preventDefault();
        navigate('/');
      }
      if ((e.metaKey || e.ctrlKey) && e.key === '2') {
        e.preventDefault();
        navigate('/albums');
      }
      if ((e.metaKey || e.ctrlKey) && e.key === 'l') {
        e.preventDefault();
        navigate('/albums');
      }
      if ((e.metaKey || e.ctrlKey) && e.key === 'h') {
        e.preventDefault();
        navigate('/');
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [navigate]);

  return (
    <SidebarStateProvider>
      <div className="flex h-full bg-background text-foreground">
        {/* Left Sidebar — collapses to CollapsedSidebarStrip when isCollapsed */}
        <LeftSidebar onAddToPlaylist={onAddToPlaylist} />

        {/* Main Content Area — adds bottom padding when player bar is visible */}
        <MainContent showHeader={showHeader}>{children}</MainContent>

        {/* Now Playing Floating Bar — visible when sidebar is collapsed + track playing */}
        <NowPlayingFloating />
      </div>
    </SidebarStateProvider>
  );
}
