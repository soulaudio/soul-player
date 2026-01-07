import { ReactNode, useState, useEffect } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { ImportDialog } from '../components/ImportDialog';
import { SourcesDialog } from '../components/SourcesDialog';
import { PlayerFooter } from '../components/PlayerFooter';
import { QueueSidebar } from '../components/QueueSidebar';
import { Tooltip } from '../components/Tooltip';
import { Kbd } from '../components/Kbd';
import { useSettings } from '../contexts/SettingsContext';
import { ListMusic } from 'lucide-react';

interface MainLayoutProps {
  children: ReactNode;
}

interface NavTab {
  path: string;
  label: string;
  icon?: string;
}

const NAV_TABS: NavTab[] = [
  { path: '/', label: 'Library', icon: '📚' },
  { path: '/playlists', label: 'Playlists', icon: '🎵' },
  { path: '/artists', label: 'Artists', icon: '👤' },
  { path: '/albums', label: 'Albums', icon: '💿' },
  { path: '/genres', label: 'Genres', icon: '🎸' },
];

export function MainLayout({ children }: MainLayoutProps) {
  const navigate = useNavigate();
  const location = useLocation();
  const { showKeyboardShortcuts } = useSettings();
  const [showImportDialog, setShowImportDialog] = useState(false);
  const [showSourcesDialog, setShowSourcesDialog] = useState(false);
  const [showQueue, setShowQueue] = useState(false);

  // Keyboard shortcuts for navigation
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Cmd/Ctrl + K for search
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        navigate('/search');
      }
      // Cmd/Ctrl + number for quick tab switching
      if ((e.metaKey || e.ctrlKey) && e.key >= '1' && e.key <= '5') {
        e.preventDefault();
        const index = parseInt(e.key) - 1;
        if (NAV_TABS[index]) {
          navigate(NAV_TABS[index].path);
        }
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

  const isActive = (path: string) => {
    if (path === '/') {
      return location.pathname === '/';
    }
    return location.pathname.startsWith(path);
  };

  return (
    <div className="flex flex-col h-screen bg-background text-foreground">
      {/* Header */}
      <header className="border-b bg-card">
        <div className="flex items-center justify-between px-4 py-2">
          {/* Left: Home + Navigation Tabs */}
          <div className="flex items-center gap-1">
            {/* Home Button */}
            <Tooltip
              content="Home"
              shortcut={showKeyboardShortcuts ? ['mod', 'h'] : undefined}
            >
              <button
                onClick={() => navigate('/')}
                className="p-2 rounded-lg hover:bg-accent transition-colors mr-2"
                aria-label="Home"
              >
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6" />
                </svg>
              </button>
            </Tooltip>

            {/* Navigation Tabs */}
            <nav className="flex items-center gap-1">
              {NAV_TABS.map((tab, index) => (
                <Tooltip
                  key={tab.path}
                  content={tab.label}
                  shortcut={showKeyboardShortcuts ? ['mod', `${index + 1}`] : undefined}
                >
                  <button
                    onClick={() => navigate(tab.path)}
                    className={`px-3 py-1.5 rounded-lg text-sm font-medium transition-colors ${
                      isActive(tab.path)
                        ? 'bg-primary text-primary-foreground'
                        : 'hover:bg-accent'
                    }`}
                    aria-label={tab.label}
                  >
                    {tab.label}
                  </button>
                </Tooltip>
              ))}
            </nav>
          </div>

          {/* Right: Search + Action Buttons + Settings */}
          <div className="flex items-center gap-2">
            {/* Search Button */}
            <button
              onClick={() => navigate('/search')}
              className="flex items-center gap-2 px-3 py-1.5 rounded-lg hover:bg-accent transition-colors text-sm"
              aria-label="Search"
            >
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
              </svg>
              {showKeyboardShortcuts && <Kbd keys={['mod', 'k']} />}
            </button>

            {/* Divider */}
            <div className="w-px h-6 bg-border" />

            {/* Import Button */}
            <Tooltip content="Import Music">
              <button
                onClick={() => setShowImportDialog(true)}
                className="p-2 rounded-lg hover:bg-accent transition-colors"
                aria-label="Import Music"
              >
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
                </svg>
              </button>
            </Tooltip>

            {/* Sources Button */}
            <Tooltip content="Manage Sources">
              <button
                onClick={() => setShowSourcesDialog(true)}
                className="p-2 rounded-lg hover:bg-accent transition-colors"
                aria-label="Manage Sources"
              >
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2m-2-4h.01M17 16h.01" />
                </svg>
              </button>
            </Tooltip>

            {/* Queue Button */}
            <Tooltip content="Queue">
              <button
                onClick={() => setShowQueue(!showQueue)}
                className={`p-2 rounded-lg transition-colors ${
                  showQueue ? 'bg-accent text-primary' : 'hover:bg-accent'
                }`}
                aria-label="Toggle Queue"
              >
                <ListMusic className="w-5 h-5" />
              </button>
            </Tooltip>

            {/* Divider */}
            <div className="w-px h-6 bg-border" />

            {/* Settings Button */}
            <Tooltip content="Settings">
              <button
                onClick={() => navigate('/settings')}
                className="p-2 rounded-lg hover:bg-accent transition-colors"
                aria-label="Settings"
              >
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                </svg>
              </button>
            </Tooltip>
          </div>
        </div>
      </header>

      {/* Main Content with Queue Sidebar */}
      <div className="flex flex-1 overflow-hidden">
        <main className="flex-1 overflow-auto p-6">
          {children}
        </main>
        <QueueSidebar isOpen={showQueue} onClose={() => setShowQueue(false)} />
      </div>

      {/* Player Footer */}
      <PlayerFooter />

      {/* Dialogs */}
      <ImportDialog open={showImportDialog} onClose={() => setShowImportDialog(false)} />
      <SourcesDialog open={showSourcesDialog} onClose={() => setShowSourcesDialog(false)} />
    </div>
  );
}
