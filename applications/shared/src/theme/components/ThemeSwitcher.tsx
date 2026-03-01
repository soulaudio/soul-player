/**
 * ThemeSwitcher component - dropdown to select themes
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useTheme } from '../useTheme';
import { DropdownMenu, DropdownMenuTrigger, DropdownMenuContent, DropdownMenuItem } from '../../components/ui/dropdown-menu';

interface ThemeSwitcherProps {
  /** Show live preview on hover/focus */
  showLivePreview?: boolean;
  /** Custom className for styling */
  className?: string;
}

/**
 * ThemeSwitcher - Simple dropdown component for theme selection
 * Includes optional live preview functionality
 */
export function ThemeSwitcher({
  showLivePreview = true,
  className = '',
}: ThemeSwitcherProps) {
  const { t } = useTranslation();
  const { currentTheme, availableThemes, setTheme, previewTheme } = useTheme();
  const [restorePreview, setRestorePreview] = useState<(() => void) | null>(
    null
  );

  const handleSelect = (themeId: string) => {
    if (restorePreview) {
      restorePreview();
      setRestorePreview(null);
    }
    setTheme(themeId);
  };

  const handleItemMouseEnter = (themeId: string) => {
    if (!showLivePreview || themeId === currentTheme.id) return;
    if (restorePreview) restorePreview();
    const restore = previewTheme(themeId);
    setRestorePreview(() => restore);
  };

  const handleItemMouseLeave = () => {
    if (restorePreview) {
      restorePreview();
      setRestorePreview(null);
    }
  };

  return (
    <div className={`theme-switcher ${className}`}>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <button className="w-full flex items-center justify-between gap-2 px-3 py-2 text-sm rounded-lg border border-border bg-background text-foreground hover:bg-foreground/[var(--hover-bg-opacity)] transition-colors duration-[var(--transition-duration)]">
            <span>{currentTheme.name}{!currentTheme.isBuiltIn ? ` (${t('theme.custom')})` : ''}</span>
            <svg className="w-4 h-4 text-muted-foreground flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7" />
            </svg>
          </button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start">
          {availableThemes.map((theme) => (
            <DropdownMenuItem
              key={theme.id}
              onClick={() => handleSelect(theme.id)}
              onMouseEnter={() => handleItemMouseEnter(theme.id)}
              onMouseLeave={handleItemMouseLeave}
              className={theme.id === currentTheme.id ? 'text-primary' : ''}
            >
              {theme.name}
              {!theme.isBuiltIn && <span className="ml-1 text-muted-foreground">({t('theme.custom')})</span>}
            </DropdownMenuItem>
          ))}
        </DropdownMenuContent>
      </DropdownMenu>
      {currentTheme.description && (
        <p className="mt-2 text-sm text-muted-foreground">
          {currentTheme.description}
        </p>
      )}
    </div>
  );
}
