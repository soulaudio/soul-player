/**
 * Theme Manager - handles theme loading, saving, import/export
 */

import type { Theme, ThemeExport, ThemeValidationResult } from './types';
import { builtInThemes, defaultTheme } from './themes';
import { validateTheme } from './validators';
import { defaultInteractions } from './defaultInteractions';
import { debug } from '../utils/debug';

const STORAGE_KEY_CURRENT_THEME = 'soul-player-current-theme';
const STORAGE_KEY_CUSTOM_THEMES = 'soul-player-custom-themes';

/**
 * Optional file backend for persistent theme storage.
 * On desktop, this is backed by Tauri FS commands writing to the app data dir.
 * Writes are fire-and-forget; localStorage acts as the runtime cache.
 */
export interface ThemeFileBackend {
  saveTheme(theme: Theme): void;
  deleteTheme(themeId: string): void;
}

/**
 * ThemeManager class - manages all theme operations
 */
export class ThemeManager {
  private currentTheme: Theme;
  private customThemes: Theme[] = [];
  private fileBackend?: ThemeFileBackend;

  constructor() {
    this.currentTheme = defaultTheme;
    this.loadFromStorage();
  }

  /**
   * Attach a file backend for persistent disk storage.
   * Called once at app startup (desktop only) after async file reads complete.
   */
  setFileBackend(backend: ThemeFileBackend): void {
    this.fileBackend = backend;
  }

  /**
   * Seed custom themes loaded from disk (called at desktop startup).
   * Replaces any localStorage-cached customs with the authoritative file list.
   */
  seedCustomThemes(themes: Theme[]): void {
    this.customThemes = themes;
    this.saveCustomThemesToStorage();
    // Re-validate current theme is still available
    if (!this.getThemeById(this.currentTheme.id)) {
      this.currentTheme = defaultTheme;
      this.saveCurrentThemeToStorage();
      this.applyTheme(this.currentTheme);
    }
  }

  /**
   * Get the current active theme
   */
  getCurrentTheme(): Theme {
    return this.currentTheme;
  }

  /**
   * Get all available themes (built-in + custom)
   */
  getAllThemes(): Theme[] {
    return [...builtInThemes, ...this.customThemes];
  }

  /**
   * Get a theme by ID
   */
  getThemeById(id: string): Theme | undefined {
    return this.getAllThemes().find((theme) => theme.id === id);
  }

  /**
   * Set the current theme by ID
   * Returns true if successful, false if theme not found
   */
  setCurrentTheme(themeId: string): boolean {
    const theme = this.getThemeById(themeId);
    if (!theme) {
      return false;
    }

    this.currentTheme = theme;
    this.saveCurrentThemeToStorage();
    this.applyTheme(theme);
    return true;
  }

  /**
   * Apply theme to the DOM
   */
  private applyTheme(theme: Theme): void {
    // Set data-theme attribute on html element
    document.documentElement.setAttribute('data-theme', theme.id);

    // Apply CSS variables for colors
    const root = document.documentElement.style;
    Object.entries(theme.colors).forEach(([key, value]) => {
      root.setProperty(`--${key}`, value);
    });

    // Apply gradients if defined
    if (theme.gradients) {
      Object.entries(theme.gradients).forEach(([key, value]) => {
        if (value) {
          root.setProperty(`--gradient-${key}`, value);
        }
      });
    }

    // Apply typography if defined
    if (theme.typography) {
      if (theme.typography.fontFamily) {
        root.setProperty(
          '--font-sans',
          theme.typography.fontFamily.sans.join(', ')
        );
        root.setProperty(
          '--font-mono',
          theme.typography.fontFamily.mono.join(', ')
        );
      }

      if (theme.typography.fontSize?.base) {
        root.setProperty('--font-size-base', theme.typography.fontSize.base);
      }
    }

    // Apply interaction settings (with fallback to defaults)
    const interactions = theme.interactions || defaultInteractions;
    root.setProperty('--hover-text-opacity', interactions.hover.textOpacity.toString());
    root.setProperty('--hover-button-opacity', interactions.hover.buttonOpacity.toString());
    root.setProperty('--hover-bg-opacity', interactions.hover.bgOpacity.toString());
    root.setProperty('--selected-bg-opacity', interactions.selected.bgOpacity.toString());
    root.setProperty('--disabled-opacity', interactions.disabled.opacity.toString());
    root.setProperty('--transition-duration', interactions.transition.duration);
  }

  /**
   * Import a theme from JSON
   * Validates the theme before importing
   */
  importTheme(themeJson: string): ThemeValidationResult & { theme?: Theme } {
    try {
      const parsed = JSON.parse(themeJson);
      const validation = validateTheme(parsed);

      if (!validation.valid) {
        return { ...validation, theme: undefined };
      }

      const theme = parsed as Theme;

      // Check if theme with this ID already exists
      const existingTheme = this.getThemeById(theme.id);
      if (existingTheme) {
        if (existingTheme.isBuiltIn) {
          return {
            valid: false,
            errors: [
              `Cannot import theme: A built-in theme with ID "${theme.id}" already exists`,
            ],
            warnings: [],
            theme: undefined,
          };
        }

        // Replace existing custom theme
        this.customThemes = this.customThemes.filter((t) => t.id !== theme.id);
      }

      // Add to custom themes
      this.customThemes.push(theme);
      this.saveCustomThemesToStorage();
      this.fileBackend?.saveTheme(theme);

      return {
        ...validation,
        theme,
      };
    } catch (error) {
      return {
        valid: false,
        errors: [
          `Failed to parse theme JSON: ${error instanceof Error ? error.message : 'Unknown error'}`,
        ],
        warnings: [],
        theme: undefined,
      };
    }
  }

  /**
   * Export a theme to JSON
   */
  exportTheme(themeId: string): string | null {
    const theme = this.getThemeById(themeId);
    if (!theme) {
      return null;
    }

    const themeExport: ThemeExport = {
      ...theme,
      exportedAt: new Date().toISOString(),
      exportedBy: 'Soul Player',
    };

    return JSON.stringify(themeExport, null, 2);
  }

  /**
   * Delete a custom theme
   * Returns true if successful, false if theme is built-in or not found
   */
  deleteTheme(themeId: string): boolean {
    const theme = this.getThemeById(themeId);
    if (!theme || theme.isBuiltIn) {
      return false;
    }

    this.customThemes = this.customThemes.filter((t) => t.id !== themeId);
    this.saveCustomThemesToStorage();
    this.fileBackend?.deleteTheme(themeId);

    // If we deleted the current theme, switch to default
    if (this.currentTheme.id === themeId) {
      this.setCurrentTheme(defaultTheme.id);
    }

    return true;
  }

  /**
   * Load themes and current selection from localStorage
   */
  private loadFromStorage(): void {
    if (typeof window === 'undefined') {
      return;
    }

    // Load custom themes
    try {
      const customThemesJson = localStorage.getItem(STORAGE_KEY_CUSTOM_THEMES);
      if (customThemesJson) {
        this.customThemes = JSON.parse(customThemesJson);
      }
    } catch (error) {
      debug.error('Failed to load custom themes from storage:', error);
    }

    // Load current theme
    try {
      const currentThemeId = localStorage.getItem(STORAGE_KEY_CURRENT_THEME);
      if (currentThemeId) {
        const theme = this.getThemeById(currentThemeId);
        if (theme) {
          this.currentTheme = theme;
          this.applyTheme(theme);
        }
      } else {
        // Apply default theme on first load
        this.applyTheme(this.currentTheme);
      }
    } catch (error) {
      debug.error('Failed to load current theme from storage:', error);
      this.applyTheme(this.currentTheme);
    }
  }

  /**
   * Save current theme ID to localStorage
   */
  private saveCurrentThemeToStorage(): void {
    if (typeof window === 'undefined') {
      return;
    }

    try {
      localStorage.setItem(STORAGE_KEY_CURRENT_THEME, this.currentTheme.id);
    } catch (error) {
      debug.error('Failed to save current theme to storage:', error);
    }
  }

  /**
   * Save custom themes to localStorage
   */
  private saveCustomThemesToStorage(): void {
    if (typeof window === 'undefined') {
      return;
    }

    try {
      localStorage.setItem(
        STORAGE_KEY_CUSTOM_THEMES,
        JSON.stringify(this.customThemes)
      );
    } catch (error) {
      debug.error('Failed to save custom themes to storage:', error);
    }
  }

  /**
   * Preview a theme temporarily without saving
   * Returns a function to restore the previous theme
   */
  previewTheme(themeId: string): (() => void) | null {
    const theme = this.getThemeById(themeId);
    if (!theme) {
      return null;
    }

    const previousTheme = this.currentTheme;
    this.applyTheme(theme);

    // Return restore function
    return () => {
      this.applyTheme(previousTheme);
    };
  }
}

/**
 * Singleton instance
 */
export const themeManager = new ThemeManager();
