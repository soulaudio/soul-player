import { ReactNode, useEffect, useCallback } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { LeftSidebar } from '../components/LeftSidebar';
import { NowPlayingFloating } from '../components/NowPlayingFloating';
import { SidebarStateProvider } from '../contexts/SidebarStateContext';
import { useSidebarState } from '../contexts/SidebarStateContext';

/** Top-level routes where back button should NOT appear */
export const ROOT_PATHS = ['/', '/albums', '/artists', '/playlists', '/genres', '/tracks', '/settings'];

/**
 * Mobile navigation model:
 * - Sidebar (menu) is the "home" view, shown when mobileShowContent=false
 * - Clicking a nav item navigates AND sets mobileShowContent=true (batched by React 18)
 * - Back button sets mobileShowContent=false to return to menu
 * - key={location.key} on content ensures clean remounts — no stale page content
 */

// Inner component — must live inside SidebarStateProvider to read context
function MainContent({ children }: { children: ReactNode }) {
  const { isMobile, mobileShowContent, setMobileShowContent } = useSidebarState();
  const navigate = useNavigate();
  const location = useLocation();

  const canGoBack = !ROOT_PATHS.includes(location.pathname);

  const handleBack = useCallback(() => {
    if (isMobile) {
      setMobileShowContent(false);
    } else {
      navigate(-1);
    }
  }, [isMobile, setMobileShowContent, navigate]);

  // Backspace keybind for back navigation
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key !== 'Backspace') return;
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === 'INPUT' || tag === 'TEXTAREA' || (e.target as HTMLElement)?.isContentEditable) return;
      if (isMobile || canGoBack) {
        e.preventDefault();
        handleBack();
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [isMobile, canGoBack, handleBack]);

  // Listen for mobile-back event from title bar
  useEffect(() => {
    const onMobileBack = () => handleBack();
    window.addEventListener('mobile-back', onMobileBack);
    return () => window.removeEventListener('mobile-back', onMobileBack);
  }, [handleBack]);

  // Sync mobile content state to DOM so title bar can read it
  useEffect(() => {
    if (isMobile && mobileShowContent) {
      document.documentElement.setAttribute('data-mobile-content', 'true');
    } else {
      document.documentElement.removeAttribute('data-mobile-content');
    }
    return () => document.documentElement.removeAttribute('data-mobile-content');
  }, [isMobile, mobileShowContent]);

  // Mobile: full-screen content — hide when menu is showing
  if (isMobile) {
    return (
      <main className={`flex-1 flex flex-col overflow-hidden ${mobileShowContent ? '' : 'hidden'}`}>
        <div className="flex-1 overflow-hidden">
          {children}
        </div>
      </main>
    );
  }

  // Desktop
  return (
    <main className="flex-1 overflow-hidden">
      <div className="h-full pl-6 pr-0">
        {children}
      </div>
    </main>
  );
}

interface MainLayoutProps {
  children: ReactNode;
  onAddToPlaylist?: () => void;
}

export function MainLayout({ children, onAddToPlaylist }: MainLayoutProps) {
  const navigate = useNavigate();


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
        <LeftSidebar onAddToPlaylist={onAddToPlaylist} />
        <MainContent>{children}</MainContent>
        <NowPlayingFloating />
      </div>
    </SidebarStateProvider>
  );
}
