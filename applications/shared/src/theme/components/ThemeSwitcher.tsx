/**
 * ThemeSwitcher component - dropdown to select themes
 */

import React, { useState } from 'react';
import { useTheme } from '../useTheme';

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
  const { currentTheme, availableThemes, setTheme, previewTheme } = useTheme();
  const [restorePreview, setRestorePreview] = useState<(() => void) | null>(
    null
  );

  const handleChange = (event: React.ChangeEvent<HTMLSelectElement>) => {
    const themeId = event.target.value;

    // Clear any active preview
    if (restorePreview) {
      restorePreview();
      setRestorePreview(null);
    }

    // Set the new theme
    setTheme(themeId);
  };

  const handleMouseEnter = (themeId: string) => {
    if (!showLivePreview || themeId === currentTheme.id) {
      return;
    }

    // Clear any existing preview
    if (restorePreview) {
      restorePreview();
    }

    // Start new preview
    const restore = previewTheme(themeId);
    setRestorePreview(() => restore);
  };

  const handleMouseLeave = () => {
    if (restorePreview) {
      restorePreview();
      setRestorePreview(null);
    }
  };

  return (
    <div className={`theme-switcher ${className}`}>
      <label
        htmlFor="theme-select"
        className="block text-sm font-medium text-foreground mb-2"
      >
        Theme
      </label>
      <select
        id="theme-select"
        value={currentTheme.id}
        onChange={handleChange}
        onMouseLeave={handleMouseLeave}
        className="w-full px-3 py-2 bg-background border border-border rounded-md text-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:border-transparent"
      >
        {availableThemes.map((theme) => (
          <option
            key={theme.id}
            value={theme.id}
            onMouseEnter={() => handleMouseEnter(theme.id)}
          >
            {theme.name}
            {theme.isBuiltIn ? '' : ' (Custom)'}
          </option>
        ))}
      </select>
      {currentTheme.description && (
        <p className="mt-2 text-sm text-muted-foreground">
          {currentTheme.description}
        </p>
      )}
    </div>
  );
}
