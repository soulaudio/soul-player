'use client';

import { ReactNode, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { LeftSidebar } from '../components/LeftSidebar';

interface MainLayoutProps {
  children: ReactNode;
  /** Callback when the "Add to Playlist" button is clicked in the sidebar */
  onAddToPlaylist?: () => void;
}

export function MainLayout({ children, onAddToPlaylist }: MainLayoutProps) {
  const navigate = useNavigate();

  // Keyboard shortcuts for navigation (playback shortcuts handled by global shortcuts system)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Cmd/Ctrl + 1 for home
      if ((e.metaKey || e.ctrlKey) && e.key === '1') {
        e.preventDefault();
        navigate('/');
      }
      // Cmd/Ctrl + 2 for library
      if ((e.metaKey || e.ctrlKey) && e.key === '2') {
        e.preventDefault();
        navigate('/library');
      }
      // Cmd/Ctrl + L for library
      if ((e.metaKey || e.ctrlKey) && e.key === 'l') {
        e.preventDefault();
        navigate('/library');
      }
      // Cmd/Ctrl + H for home
      if ((e.metaKey || e.ctrlKey) && e.key === 'h') {
        e.preventDefault();
        navigate('/');
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [navigate]);

  return (
    <div className="flex h-full bg-background text-foreground">
      {/* Left Sidebar - full height, always visible */}
      <LeftSidebar onAddToPlaylist={onAddToPlaylist} />

      {/* Main Content Area */}
      <main className="flex-1 overflow-auto">
        <div className="h-full p-6">
          {children}
        </div>
      </main>
    </div>
  );
}
