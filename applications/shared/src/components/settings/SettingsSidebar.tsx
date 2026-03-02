// Settings sidebar navigation — icon-only toolbar with Radix tooltips

import { useTranslation } from 'react-i18next';
import { useLocation, Link } from 'react-router-dom';
import * as Tooltip from '@radix-ui/react-tooltip';
import {
  Volume2,
  Palette,
  Info,
  Keyboard,
  Database,
} from 'lucide-react';

interface NavItem {
  id: string;
  labelKey: string;
  path: string;
  icon: React.ComponentType<{ className?: string }>;
}

const navigationItems: NavItem[] = [
  { id: 'appearance', labelKey: 'settings.sections.appearance', path: '/settings/appearance', icon: Palette  },
  { id: 'musicData',  labelKey: 'settings.sections.musicData',  path: '/settings/music-data', icon: Database },
  { id: 'audio',      labelKey: 'settings.sections.audio',      path: '/settings/audio',      icon: Volume2  },
  { id: 'shortcuts',  labelKey: 'settings.sections.shortcuts',  path: '/settings/shortcuts',  icon: Keyboard },
  { id: 'about',      labelKey: 'settings.sections.about',      path: '/settings/about',      icon: Info     },
];

export function SettingsSidebar() {
  const { t } = useTranslation();
  const location = useLocation();

  return (
    <Tooltip.Provider delayDuration={600}>
      <nav className="flex flex-col items-center gap-1 bg-muted/40 rounded-xl p-1.5">
        {navigationItems.map((item) => {
          const Icon = item.icon;
          const isActive = location.pathname === item.path;

          return (
            <Tooltip.Root key={item.id}>
              <Tooltip.Trigger asChild>
                <Link
                  to={item.path}
                  data-testid={`nav-settings-${item.id}`}
                  data-state={isActive ? 'active' : 'inactive'}
                  aria-current={isActive ? 'page' : undefined}
                  className={[
                    'p-3 rounded-lg transition-all',
                    isActive
                      ? 'text-primary-foreground bg-primary shadow-sm'
                      : 'text-muted-foreground bg-background/50 hover:bg-background hover:opacity-80',
                  ].join(' ')}
                >
                  <Icon className="w-5 h-5" />
                </Link>
              </Tooltip.Trigger>
              <Tooltip.Portal>
                <Tooltip.Content
                  side="right"
                  sideOffset={8}
                  className="z-50 px-2 py-1 text-xs rounded-md bg-popover text-popover-foreground shadow-md select-none"
                >
                  {t(item.labelKey)}
                  <Tooltip.Arrow className="fill-popover" />
                </Tooltip.Content>
              </Tooltip.Portal>
            </Tooltip.Root>
          );
        })}
      </nav>
    </Tooltip.Provider>
  );
}
