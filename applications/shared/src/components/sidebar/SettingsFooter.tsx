'use client';

import { useTranslation } from 'react-i18next';
import { useNavigate, useLocation } from 'react-router-dom';
import { Settings } from 'lucide-react';
import { cn } from '../../lib/utils';

export interface SettingsFooterProps {
  version?: string;
}

export function SettingsFooter({ version }: SettingsFooterProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const location = useLocation();

  return (
    <div className="border-t border-border px-3 py-2 flex items-center justify-between">
      <button
        onClick={() => navigate('/settings')}
        className={cn(
          'p-1.5 rounded-lg transition-opacity',
          location.pathname.startsWith('/settings')
            ? 'text-primary bg-accent/20'
            : 'text-muted-foreground hover:opacity-[var(--hover-text-opacity)] hover:bg-foreground/10'
        )}
        title={t('nav.settings')}
      >
        <Settings className="w-4 h-4" />
      </button>
      {version && (
        <div className="text-xs text-muted-foreground/60 font-mono">{version}</div>
      )}
    </div>
  );
}
