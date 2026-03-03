'use client';

import { useTranslation } from 'react-i18next';
import { useNavigate, useLocation } from 'react-router-dom';
import { cn } from '../../lib/utils';

interface NavItem {
  id: string;
  labelKey: string;
  path: string;
}

const navigationItems: NavItem[] = [
  { id: 'home', labelKey: 'nav.home', path: '/' },
  { id: 'albums', labelKey: 'library.tab.albums', path: '/albums' },
  { id: 'artists', labelKey: 'library.tab.artists', path: '/artists' },
  { id: 'playlists', labelKey: 'library.tab.playlists', path: '/playlists' },
  { id: 'genres', labelKey: 'nav.genres', path: '/genres' },
  { id: 'tracks', labelKey: 'library.tab.tracks', path: '/tracks' },
];

export interface NavBarProps {
  homeEnabled?: boolean;
}

export function NavBar({ homeEnabled = true }: NavBarProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const location = useLocation();

  const isActive = (path: string) => {
    if (path === '/') {
      return location.pathname === '/';
    }
    return location.pathname === path || location.pathname.startsWith(path + '/');
  };

  const visibleNavigationItems = navigationItems.filter((item) => {
    if (item.id === 'home' && !homeEnabled) {
      return false;
    }
    return true;
  });

  return (
    <nav className="p-4 pt-6 flex-shrink-0">
      <ul className="space-y-0">
        {visibleNavigationItems.map((item) => (
          <li key={item.id}>
            <button
              onClick={() => navigate(item.path)}
              data-testid={`nav-${item.id}`}
              className={cn(
                'w-full text-left px-3 py-1 text-xl font-semibold tracking-wide transition-opacity flex items-center justify-between gap-2',
                isActive(item.path)
                  ? 'text-primary'
                  : 'text-muted-foreground hover:opacity-[var(--hover-text-opacity)]'
              )}
            >
              <span>{t(item.labelKey)}</span>
              <div
                className={cn(
                  'w-2 h-2 rounded-full border transition-all flex-shrink-0',
                  isActive(item.path)
                    ? 'bg-primary border-primary'
                    : 'border-muted-foreground/30'
                )}
              />
            </button>
          </li>
        ))}
      </ul>
    </nav>
  );
}
