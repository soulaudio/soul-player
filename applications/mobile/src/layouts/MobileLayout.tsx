import { ReactNode } from 'react';
import { Link, useLocation } from 'react-router-dom';

interface MobileLayoutProps {
  children: ReactNode;
}

export function MobileLayout({ children }: MobileLayoutProps) {
  const location = useLocation();

  const isActive = (path: string) => location.pathname === path;

  return (
    <div className="flex flex-col h-screen bg-background text-foreground">
      {/* Main Content */}
      <main className="flex-1 overflow-auto pb-32">
        {children}
      </main>

      {/* Mini Player Bar */}
      <div className="fixed bottom-16 left-0 right-0 border-t bg-card p-3">
        <div className="flex items-center justify-between">
          <div className="flex-1 min-w-0">
            <div className="text-sm font-medium truncate">No track playing</div>
            <div className="text-xs text-muted-foreground truncate">Soul Player</div>
          </div>
          <button className="p-2 hover:bg-accent rounded-full">
            <span className="text-2xl">▶</span>
          </button>
        </div>
      </div>

      {/* Bottom Navigation */}
      <nav className="fixed bottom-0 left-0 right-0 border-t bg-card">
        <div className="flex justify-around items-center h-16">
          <Link
            to="/"
            className={`flex flex-col items-center justify-center flex-1 h-full ${
              isActive('/') ? 'text-primary' : 'text-muted-foreground'
            }`}
          >
            <span className="text-xl mb-1">📚</span>
            <span className="text-xs">Library</span>
          </Link>

          <Link
            to="/playlists"
            className={`flex flex-col items-center justify-center flex-1 h-full ${
              isActive('/playlists') ? 'text-primary' : 'text-muted-foreground'
            }`}
          >
            <span className="text-xl mb-1">🎵</span>
            <span className="text-xs">Playlists</span>
          </Link>

          <Link
            to="/now-playing"
            className={`flex flex-col items-center justify-center flex-1 h-full ${
              isActive('/now-playing') ? 'text-primary' : 'text-muted-foreground'
            }`}
          >
            <span className="text-xl mb-1">🎧</span>
            <span className="text-xs">Playing</span>
          </Link>

          <Link
            to="/settings"
            className={`flex flex-col items-center justify-center flex-1 h-full ${
              isActive('/settings') ? 'text-primary' : 'text-muted-foreground'
            }`}
          >
            <span className="text-xl mb-1">⚙️</span>
            <span className="text-xs">Settings</span>
          </Link>
        </div>
      </nav>
    </div>
  );
}
