import { ReactNode } from 'react';

interface MainLayoutProps {
  children: ReactNode;
}

export function MainLayout({ children }: MainLayoutProps) {
  return (
    <div className="flex flex-col h-screen bg-background text-foreground">
      {/* Navigation Sidebar */}
      <div className="flex flex-1 overflow-hidden">
        <aside className="w-64 border-r bg-muted/40 p-4">
          <nav className="space-y-2">
            <a href="/" className="block px-3 py-2 rounded-lg hover:bg-accent">
              Library
            </a>
            <a href="/playlists" className="block px-3 py-2 rounded-lg hover:bg-accent">
              Playlists
            </a>
            <a href="/settings" className="block px-3 py-2 rounded-lg hover:bg-accent">
              Settings
            </a>
          </nav>
        </aside>

        {/* Main Content */}
        <main className="flex-1 overflow-auto p-6">
          {children}
        </main>
      </div>

      {/* Player Bar (bottom) */}
      <div className="border-t bg-card p-4">
        <div className="flex items-center justify-between">
          <div className="text-sm">
            <div className="font-medium">No track playing</div>
            <div className="text-muted-foreground text-xs">Soul Player</div>
          </div>

          <div className="flex items-center gap-4">
            <button className="p-2 hover:bg-accent rounded-full">
              <span>⏮</span>
            </button>
            <button className="p-3 hover:bg-accent rounded-full bg-primary text-primary-foreground">
              <span>▶</span>
            </button>
            <button className="p-2 hover:bg-accent rounded-full">
              <span>⏭</span>
            </button>
          </div>

          <div className="w-32 text-sm text-muted-foreground">
            0:00 / 0:00
          </div>
        </div>
      </div>
    </div>
  );
}
