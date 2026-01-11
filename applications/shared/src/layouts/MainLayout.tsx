'use client';

import { ReactNode, useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { LeftSidebar } from '../components/LeftSidebar';
import { SourcesDialog } from '../components/SourcesDialog';

interface MainLayoutProps {
  children: ReactNode;
  /**
   * Optional callback when Import button is clicked
   * If not provided, Import button will be disabled (for demo)
   */
  onImport?: () => void;
}

export function MainLayout({ children, onImport }: MainLayoutProps) {
  const navigate = useNavigate();
  const [showSourcesDialog, setShowSourcesDialog] = useState(false);

  // Keyboard shortcuts for navigation (playback shortcuts handled by global shortcuts system)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Cmd/Ctrl + K for search
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        navigate('/search');
      }
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
      // Cmd/Ctrl + 3 for discovery
      if ((e.metaKey || e.ctrlKey) && e.key === '3') {
        e.preventDefault();
        navigate('/discovery');
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
      <LeftSidebar
        onImport={onImport}
        onOpenSources={() => setShowSourcesDialog(true)}
      />

      {/* Main Content Area */}
      <main className="flex-1 overflow-auto">
        <div className="container mx-auto p-6">
          {children}
        </div>
      </main>

      {/* Sources Dialog - overlay */}
      <SourcesDialog
        open={showSourcesDialog}
        onClose={() => setShowSourcesDialog(false)}
      />
    </div>
  );
}
