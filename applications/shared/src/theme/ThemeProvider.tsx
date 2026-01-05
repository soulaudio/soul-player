/**
 * React Context Provider for Theme Management
 */

import React, { createContext, useEffect, useState, useCallback } from 'react';
import type { Theme, ThemeValidationResult } from './types';
import { themeManager } from './ThemeManager';

interface ThemeContextValue {
  /** Current active theme */
  currentTheme: Theme;

  /** All available themes */
  availableThemes: Theme[];

  /** Switch to a different theme */
  setTheme: (themeId: string) => boolean;

  /** Import a theme from JSON */
  importTheme: (
    themeJson: string
  ) => ThemeValidationResult & { theme?: Theme };

  /** Export a theme to JSON */
  exportTheme: (themeId: string) => string | null;

  /** Delete a custom theme */
  deleteTheme: (themeId: string) => boolean;

  /** Preview a theme temporarily (returns restore function) */
  previewTheme: (themeId: string) => (() => void) | null;
}

const ThemeContext = createContext<ThemeContextValue | undefined>(undefined);

interface ThemeProviderProps {
  children: React.ReactNode;
}

/**
 * ThemeProvider component - wraps the app to provide theme context
 */
export function ThemeProvider({ children }: ThemeProviderProps) {
  const [currentTheme, setCurrentTheme] = useState<Theme>(
    themeManager.getCurrentTheme()
  );
  const [availableThemes, setAvailableThemes] = useState<Theme[]>(
    themeManager.getAllThemes()
  );

  // Update state when theme changes
  const handleSetTheme = useCallback((themeId: string): boolean => {
    const success = themeManager.setCurrentTheme(themeId);
    if (success) {
      setCurrentTheme(themeManager.getCurrentTheme());
    }
    return success;
  }, []);

  const handleImportTheme = useCallback(
    (themeJson: string): ThemeValidationResult & { theme?: Theme } => {
      const result = themeManager.importTheme(themeJson);
      if (result.valid && result.theme) {
        setAvailableThemes(themeManager.getAllThemes());
      }
      return result;
    },
    []
  );

  const handleDeleteTheme = useCallback((themeId: string): boolean => {
    const success = themeManager.deleteTheme(themeId);
    if (success) {
      setAvailableThemes(themeManager.getAllThemes());
      setCurrentTheme(themeManager.getCurrentTheme());
    }
    return success;
  }, []);

  const handleExportTheme = useCallback((themeId: string): string | null => {
    return themeManager.exportTheme(themeId);
  }, []);

  const handlePreviewTheme = useCallback(
    (themeId: string): (() => void) | null => {
      return themeManager.previewTheme(themeId);
    },
    []
  );

  const value: ThemeContextValue = {
    currentTheme,
    availableThemes,
    setTheme: handleSetTheme,
    importTheme: handleImportTheme,
    exportTheme: handleExportTheme,
    deleteTheme: handleDeleteTheme,
    previewTheme: handlePreviewTheme,
  };

  return (
    <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>
  );
}

/**
 * Hook to access theme context
 * Must be used within a ThemeProvider
 */
export function useTheme(): ThemeContextValue {
  const context = React.useContext(ThemeContext);
  if (context === undefined) {
    throw new Error('useTheme must be used within a ThemeProvider');
  }
  return context;
}
