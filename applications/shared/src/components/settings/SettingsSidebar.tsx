// Settings sidebar navigation

import { useTranslation } from 'react-i18next';
import { useLocation, Link } from 'react-router-dom';
import {
  Volume2,
  Palette,
  Music,
  Zap,
  Info,
  Keyboard,
} from 'lucide-react';

interface NavItem {
  id: string;
  labelKey: string;
  path: string;
  icon: React.ComponentType<{ className?: string }>;
}

const navigationItems: NavItem[] = [
  {
    id: 'audio',
    labelKey: 'settings.sections.audio',
    path: '/settings/audio',
    icon: Volume2,
  },
  {
    id: 'library',
    labelKey: 'settings.sections.library',
    path: '/settings/library',
    icon: Music,
  },
  {
    id: 'playback',
    labelKey: 'settings.sections.playback',
    path: '/settings/playback',
    icon: Zap,
  },
  {
    id: 'appearance',
    labelKey: 'settings.sections.appearance',
    path: '/settings/appearance',
    icon: Palette,
  },
  {
    id: 'shortcuts',
    labelKey: 'settings.sections.shortcuts',
    path: '/settings/shortcuts',
    icon: Keyboard,
  },
  {
    id: 'about',
    labelKey: 'settings.sections.about',
    path: '/settings/about',
    icon: Info,
  },
];

export function SettingsSidebar() {
  const { t } = useTranslation();
  const location = useLocation();

  return (
    <nav className="p-4 h-full flex flex-col">
      <h2 className="text-lg font-semibold mb-6 px-3">{t('settings.title')}</h2>

      <ul className="space-y-1 flex-1">
        {navigationItems.map((item) => {
          const Icon = item.icon;
          const isActive = location.pathname === item.path;

          return (
            <li key={item.id}>
              <Link
                to={item.path}
                className={`
                  flex items-center gap-3 px-3 py-2 rounded-lg text-sm
                  transition-colors duration-150
                  ${
                    isActive
                      ? 'bg-primary text-primary-foreground font-medium'
                      : 'text-muted-foreground hover:bg-muted hover:text-foreground'
                  }
                `}
              >
                <Icon className="w-4 h-4 flex-shrink-0" />
                <span>{t(item.labelKey)}</span>
              </Link>
            </li>
          );
        })}
      </ul>

      {/* Footer hint */}
      <div className="text-xs text-muted-foreground px-3 pt-4 border-t">
        <p>{t('settings.hint', 'Changes are saved automatically')}</p>
      </div>
    </nav>
  );
}
