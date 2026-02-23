/**
 * ThemePreview component - visual preview of a theme
 */

import { useTranslation } from 'react-i18next';
import type { Theme } from '../types';

interface ThemePreviewProps {
  /** Theme to preview */
  theme: Theme;
  /** Whether this theme is currently active */
  isActive?: boolean;
  /** Click handler */
  onClick?: () => void;
  /** Custom className */
  className?: string;
}

/**
 * ThemePreview - Visual preview card showing a theme's colors
 * Useful for theme galleries or selection screens
 */
export function ThemePreview({
  theme,
  isActive = false,
  onClick,
  className = '',
}: ThemePreviewProps) {
  const { t } = useTranslation();
  return (
    <div
      className={`theme-preview cursor-pointer rounded-lg border-2 p-4 transition-all hover:shadow-lg ${
        isActive ? 'border-primary shadow-md' : 'border-border'
      } ${className}`}
      onClick={onClick}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          onClick?.();
        }
      }}
    >
      {/* Theme name and info */}
      <div className="mb-3">
        <h3 className="font-semibold text-foreground">
          {theme.name}
          {isActive && (
            <span className="ml-2 text-xs text-primary">({t('theme.active')})</span>
          )}
        </h3>
        {theme.description && (
          <p className="text-xs text-muted-foreground mt-1">
            {theme.description}
          </p>
        )}
      </div>

      {/* Color swatches */}
      <div className="grid grid-cols-6 gap-2 mb-3">
        <ColorSwatch
          color={theme.colors.background}
          label={t('theme.colors.background')}
        />
        <ColorSwatch
          color={theme.colors.foreground}
          label={t('theme.colors.foreground')}
        />
        <ColorSwatch color={theme.colors.primary}     label={t('theme.colors.primary')}     />
        <ColorSwatch color={theme.colors.secondary}   label={t('theme.colors.secondary')}   />
        <ColorSwatch color={theme.colors.accent}      label={t('theme.colors.accent')}      />
        <ColorSwatch color={theme.colors.destructive} label={t('theme.colors.destructive')} />
      </div>

      {/* Gradient preview if available */}
      {theme.gradients?.hero && (
        <div className="mt-3">
          <div
            className="h-8 rounded"
            style={{ background: theme.gradients.hero }}
          />
        </div>
      )}

      {/* Metadata */}
      <div className="mt-3 text-xs text-muted-foreground flex justify-between">
        <span>v{theme.version}</span>
        {theme.isBuiltIn ? (
          <span className="text-primary">{t('theme.builtIn')}</span>
        ) : (
          <span>{t('theme.custom')}</span>
        )}
      </div>
    </div>
  );
}

interface ColorSwatchProps {
  color: string;
  label: string;
  tooltip?: string;
}

/**
 * ColorSwatch - Individual color swatch display
 */
function ColorSwatch({ color, label, tooltip }: ColorSwatchProps) {
  return (
    <div
      className="aspect-square rounded border border-border"
      style={{ backgroundColor: `hsl(${color})` }}
      title={tooltip || label}
      aria-label={label}
    />
  );
}
