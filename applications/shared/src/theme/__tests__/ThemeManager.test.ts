/**
 * Comprehensive tests for ThemeManager
 * Focus: localStorage integration, import/export, theme switching, preview, error handling
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { ThemeManager } from '../ThemeManager';
import { lightTheme, darkTheme, oceanTheme } from '../themes';
import type { Theme } from '../types';

// Mock localStorage
const localStorageMock = (() => {
  let store: Record<string, string> = {};

  return {
    getItem: (key: string) => store[key] || null,
    setItem: (key: string, value: string) => {
      store[key] = value;
    },
    removeItem: (key: string) => {
      delete store[key];
    },
    clear: () => {
      store = {};
    },
  };
})();

Object.defineProperty(window, 'localStorage', {
  value: localStorageMock,
});

// Mock document.documentElement
const htmlElementMock = {
  setAttribute: vi.fn(),
  style: {
    setProperty: vi.fn(),
  },
};

Object.defineProperty(document, 'documentElement', {
  value: htmlElementMock,
  writable: true,
});

describe('ThemeManager', () => {
  let manager: ThemeManager;

  beforeEach(() => {
    localStorageMock.clear();
    htmlElementMock.setAttribute.mockClear();
    htmlElementMock.style.setProperty.mockClear();
    manager = new ThemeManager();
  });

  afterEach(() => {
    localStorageMock.clear();
  });

  describe('initialization', () => {
    it('should initialize with default light theme', () => {
      const currentTheme = manager.getCurrentTheme();
      expect(currentTheme.id).toBe('light');
    });

    it('should include all built-in themes', () => {
      const allThemes = manager.getAllThemes();
      expect(allThemes).toHaveLength(3); // light, dark, ocean

      const ids = allThemes.map((t) => t.id);
      expect(ids).toContain('light');
      expect(ids).toContain('dark');
      expect(ids).toContain('ocean');
    });

    it('should apply default theme to DOM on initialization', () => {
      expect(htmlElementMock.setAttribute).toHaveBeenCalledWith('data-theme', 'light');
      expect(htmlElementMock.style.setProperty).toHaveBeenCalled();
    });

    it('should load previously selected theme from localStorage', () => {
      // Set dark theme in localStorage before creating manager
      localStorageMock.setItem('soul-player-current-theme', 'dark');

      const newManager = new ThemeManager();
      const currentTheme = newManager.getCurrentTheme();

      expect(currentTheme.id).toBe('dark');
    });

    it('should load custom themes from localStorage', () => {
      const customTheme: Theme = {
        id: 'custom',
        name: 'Custom Theme',
        version: '1.0.0',
        colors: {
          ...lightTheme.colors,
        },
      };

      localStorageMock.setItem('soul-player-custom-themes', JSON.stringify([customTheme]));

      const newManager = new ThemeManager();
      const allThemes = newManager.getAllThemes();

      expect(allThemes).toHaveLength(4); // 3 built-in + 1 custom
      expect(allThemes.some((t) => t.id === 'custom')).toBe(true);
    });

    it('should fallback to default theme if localStorage has invalid theme ID', () => {
      localStorageMock.setItem('soul-player-current-theme', 'non-existent-theme');

      const newManager = new ThemeManager();
      const currentTheme = newManager.getCurrentTheme();

      expect(currentTheme.id).toBe('light'); // Should fallback to default
    });

    it('should handle corrupted localStorage gracefully', () => {
      localStorageMock.setItem('soul-player-custom-themes', 'invalid json{{{');

      // Should not throw, should just start with built-in themes
      expect(() => new ThemeManager()).not.toThrow();

      const newManager = new ThemeManager();
      const allThemes = newManager.getAllThemes();

      expect(allThemes).toHaveLength(3); // Only built-in themes
    });
  });

  describe('getThemeById', () => {
    it('should retrieve built-in theme by ID', () => {
      const theme = manager.getThemeById('dark');
      expect(theme).toBeDefined();
      expect(theme?.id).toBe('dark');
    });

    it('should return undefined for non-existent theme', () => {
      const theme = manager.getThemeById('non-existent');
      expect(theme).toBeUndefined();
    });

    it('should retrieve custom theme by ID after import', () => {
      const customTheme: Theme = {
        id: 'custom',
        name: 'Custom',
        version: '1.0.0',
        colors: { ...lightTheme.colors },
      };

      const json = JSON.stringify(customTheme);
      manager.importTheme(json);

      const retrieved = manager.getThemeById('custom');
      expect(retrieved).toBeDefined();
      expect(retrieved?.name).toBe('Custom');
    });
  });

  describe('setCurrentTheme', () => {
    beforeEach(() => {
      htmlElementMock.setAttribute.mockClear();
      htmlElementMock.style.setProperty.mockClear();
    });

    it('should change current theme successfully', () => {
      const success = manager.setCurrentTheme('dark');

      expect(success).toBe(true);
      expect(manager.getCurrentTheme().id).toBe('dark');
    });

    it('should persist theme selection to localStorage', () => {
      manager.setCurrentTheme('ocean');

      const stored = localStorageMock.getItem('soul-player-current-theme');
      expect(stored).toBe('ocean');
    });

    it('should apply theme to DOM', () => {
      manager.setCurrentTheme('dark');

      expect(htmlElementMock.setAttribute).toHaveBeenCalledWith('data-theme', 'dark');
      expect(htmlElementMock.style.setProperty).toHaveBeenCalled();
    });

    it('should apply all color variables to DOM', () => {
      manager.setCurrentTheme('ocean');

      // Check that CSS variables were set
      expect(htmlElementMock.style.setProperty).toHaveBeenCalledWith(
        '--background',
        oceanTheme.colors.background
      );
      expect(htmlElementMock.style.setProperty).toHaveBeenCalledWith(
        '--primary',
        oceanTheme.colors.primary
      );
    });

    it('should apply gradients if defined', () => {
      manager.setCurrentTheme('ocean');

      if (oceanTheme.gradients?.hero) {
        expect(htmlElementMock.style.setProperty).toHaveBeenCalledWith(
          '--gradient-hero',
          oceanTheme.gradients.hero
        );
      }
    });

    it('should apply typography if defined', () => {
      manager.setCurrentTheme('light');

      if (lightTheme.typography?.fontFamily) {
        expect(htmlElementMock.style.setProperty).toHaveBeenCalledWith(
          '--font-sans',
          lightTheme.typography.fontFamily.sans.join(', ')
        );
      }
    });

    it('should return false for non-existent theme', () => {
      const success = manager.setCurrentTheme('non-existent');

      expect(success).toBe(false);
      expect(manager.getCurrentTheme().id).toBe('light'); // Should remain unchanged
    });

    it('should not change theme or update localStorage on failure', () => {
      const initialTheme = manager.getCurrentTheme().id;
      const initialStorage = localStorageMock.getItem('soul-player-current-theme');

      manager.setCurrentTheme('non-existent');

      expect(manager.getCurrentTheme().id).toBe(initialTheme);
      expect(localStorageMock.getItem('soul-player-current-theme')).toBe(initialStorage);
    });
  });

  describe('importTheme', () => {
    it('should import valid theme successfully', () => {
      const customTheme: Theme = {
        id: 'custom',
        name: 'Custom Theme',
        version: '1.0.0',
        colors: { ...lightTheme.colors },
      };

      const json = JSON.stringify(customTheme);
      const result = manager.importTheme(json);

      expect(result.valid).toBe(true);
      expect(result.theme).toBeDefined();
      expect(result.theme?.id).toBe('custom');
      expect(result.errors).toHaveLength(0);
    });

    it('should add imported theme to available themes', () => {
      const customTheme: Theme = {
        id: 'custom',
        name: 'Custom',
        version: '1.0.0',
        colors: { ...lightTheme.colors },
      };

      manager.importTheme(JSON.stringify(customTheme));

      const allThemes = manager.getAllThemes();
      expect(allThemes.some((t) => t.id === 'custom')).toBe(true);
    });

    it('should persist imported theme to localStorage', () => {
      const customTheme: Theme = {
        id: 'custom',
        name: 'Custom',
        version: '1.0.0',
        colors: { ...lightTheme.colors },
      };

      manager.importTheme(JSON.stringify(customTheme));

      const stored = localStorageMock.getItem('soul-player-custom-themes');
      expect(stored).toBeTruthy();

      const parsed = JSON.parse(stored!);
      expect(parsed).toHaveLength(1);
      expect(parsed[0].id).toBe('custom');
    });

    it('should replace existing custom theme with same ID', () => {
      const theme1: Theme = {
        id: 'custom',
        name: 'Version 1',
        version: '1.0.0',
        colors: { ...lightTheme.colors },
      };

      const theme2: Theme = {
        id: 'custom',
        name: 'Version 2',
        version: '2.0.0',
        colors: { ...darkTheme.colors },
      };

      manager.importTheme(JSON.stringify(theme1));
      manager.importTheme(JSON.stringify(theme2));

      const allThemes = manager.getAllThemes();
      const customThemes = allThemes.filter((t) => t.id === 'custom');

      expect(customThemes).toHaveLength(1);
      expect(customThemes[0].name).toBe('Version 2');
    });

    it('should reject import if ID conflicts with built-in theme', () => {
      const conflictingTheme: Theme = {
        ...lightTheme,
        id: 'light', // Conflicts with built-in
        name: 'My Light Theme',
      };

      const result = manager.importTheme(JSON.stringify(conflictingTheme));

      expect(result.valid).toBe(false);
      expect(result.errors.some((e) => e.includes('built-in'))).toBe(true);
    });

    it('should reject invalid JSON', () => {
      const result = manager.importTheme('invalid json{{{');

      expect(result.valid).toBe(false);
      expect(result.errors.length).toBeGreaterThan(0);
      expect(result.errors[0]).toContain('parse');
    });

    it('should reject theme with invalid structure', () => {
      const invalidTheme = {
        id: 'invalid',
        name: 'Invalid',
        version: 'not-semver', // Invalid version
        colors: {}, // Missing required colors
      };

      const result = manager.importTheme(JSON.stringify(invalidTheme));

      expect(result.valid).toBe(false);
      expect(result.errors.length).toBeGreaterThan(0);
    });

    it('should return warnings for themes with accessibility issues', () => {
      const marginalTheme: Theme = {
        id: 'marginal',
        name: 'Marginal',
        version: '1.0.0',
        colors: {
          ...lightTheme.colors,
          foreground: '0 0% 40%', // Passes AA but not AAA
        },
      };

      const result = manager.importTheme(JSON.stringify(marginalTheme));

      expect(result.valid).toBe(true); // Still valid (AA is requirement)
      expect(result.warnings.length).toBeGreaterThan(0); // But has warnings
    });
  });

  describe('exportTheme', () => {
    it('should export existing theme to JSON string', () => {
      const json = manager.exportTheme('light');

      expect(json).toBeTruthy();
      expect(typeof json).toBe('string');

      const parsed = JSON.parse(json!);
      expect(parsed.id).toBe('light');
      expect(parsed.name).toBe('Light');
    });

    it('should include export metadata', () => {
      const json = manager.exportTheme('dark');
      const parsed = JSON.parse(json!);

      expect(parsed.exportedAt).toBeDefined();
      expect(parsed.exportedBy).toBe('Soul Player');
    });

    it('should export custom themes', () => {
      const customTheme: Theme = {
        id: 'custom',
        name: 'Custom',
        version: '1.0.0',
        colors: { ...lightTheme.colors },
      };

      manager.importTheme(JSON.stringify(customTheme));

      const exported = manager.exportTheme('custom');
      expect(exported).toBeTruthy();

      const parsed = JSON.parse(exported!);
      expect(parsed.id).toBe('custom');
    });

    it('should return null for non-existent theme', () => {
      const json = manager.exportTheme('non-existent');
      expect(json).toBeNull();
    });

    it('should export valid JSON that can be re-imported', () => {
      const exported = manager.exportTheme('ocean');
      expect(exported).toBeTruthy();

      // Remove from custom themes if it somehow got there
      const result = manager.importTheme(exported!);

      // Should either be valid or conflict with built-in
      expect(result.valid || result.errors.some((e) => e.includes('built-in'))).toBe(true);
    });
  });

  describe('deleteTheme', () => {
    it('should delete custom theme successfully', () => {
      const customTheme: Theme = {
        id: 'custom',
        name: 'Custom',
        version: '1.0.0',
        colors: { ...lightTheme.colors },
      };

      manager.importTheme(JSON.stringify(customTheme));

      const success = manager.deleteTheme('custom');
      expect(success).toBe(true);

      const allThemes = manager.getAllThemes();
      expect(allThemes.every((t) => t.id !== 'custom')).toBe(true);
    });

    it('should update localStorage after deletion', () => {
      const customTheme: Theme = {
        id: 'custom',
        name: 'Custom',
        version: '1.0.0',
        colors: { ...lightTheme.colors },
      };

      manager.importTheme(JSON.stringify(customTheme));
      manager.deleteTheme('custom');

      const stored = localStorageMock.getItem('soul-player-custom-themes');
      const parsed = JSON.parse(stored!);

      expect(parsed.every((t: Theme) => t.id !== 'custom')).toBe(true);
    });

    it('should not delete built-in themes', () => {
      const success = manager.deleteTheme('light');

      expect(success).toBe(false);
      expect(manager.getThemeById('light')).toBeDefined();
    });

    it('should switch to default theme if deleting current theme', () => {
      const customTheme: Theme = {
        id: 'custom',
        name: 'Custom',
        version: '1.0.0',
        colors: { ...lightTheme.colors },
      };

      manager.importTheme(JSON.stringify(customTheme));
      manager.setCurrentTheme('custom');

      expect(manager.getCurrentTheme().id).toBe('custom');

      manager.deleteTheme('custom');

      expect(manager.getCurrentTheme().id).toBe('light'); // Should revert to default
    });

    it('should return false for non-existent theme', () => {
      const success = manager.deleteTheme('non-existent');
      expect(success).toBe(false);
    });
  });

  describe('previewTheme', () => {
    beforeEach(() => {
      htmlElementMock.setAttribute.mockClear();
      htmlElementMock.style.setProperty.mockClear();
    });

    it('should apply theme temporarily for preview', () => {
      manager.setCurrentTheme('light');
      htmlElementMock.setAttribute.mockClear();

      const restore = manager.previewTheme('dark');

      expect(htmlElementMock.setAttribute).toHaveBeenCalledWith('data-theme', 'dark');
      expect(restore).toBeTruthy();
    });

    it('should return restore function', () => {
      const restore = manager.previewTheme('ocean');

      expect(typeof restore).toBe('function');
    });

    it('should restore previous theme when restore function is called', () => {
      manager.setCurrentTheme('light');
      htmlElementMock.setAttribute.mockClear();

      const restore = manager.previewTheme('dark');
      restore!();

      // Should apply light theme again
      expect(htmlElementMock.setAttribute).toHaveBeenCalledWith('data-theme', 'light');
    });

    it('should not change current theme (only visual preview)', () => {
      manager.setCurrentTheme('light');
      manager.previewTheme('dark');

      expect(manager.getCurrentTheme().id).toBe('light'); // Should still be light
    });

    it('should not save preview to localStorage', () => {
      manager.setCurrentTheme('light');
      const initialStorage = localStorageMock.getItem('soul-player-current-theme');

      manager.previewTheme('dark');

      expect(localStorageMock.getItem('soul-player-current-theme')).toBe(initialStorage);
    });

    it('should return null for non-existent theme', () => {
      const restore = manager.previewTheme('non-existent');
      expect(restore).toBeNull();
    });
  });

  describe('edge cases and error handling', () => {
    it('should handle multiple rapid theme switches', () => {
      manager.setCurrentTheme('light');
      manager.setCurrentTheme('dark');
      manager.setCurrentTheme('ocean');
      manager.setCurrentTheme('light');

      expect(manager.getCurrentTheme().id).toBe('light');
      expect(localStorageMock.getItem('soul-player-current-theme')).toBe('light');
    });

    it('should handle importing multiple themes in sequence', () => {
      const themes = ['theme1', 'theme2', 'theme3'].map((id) => ({
        id,
        name: id,
        version: '1.0.0',
        colors: { ...lightTheme.colors },
      }));

      themes.forEach((theme) => {
        const result = manager.importTheme(JSON.stringify(theme));
        expect(result.valid).toBe(true);
      });

      const allThemes = manager.getAllThemes();
      expect(allThemes).toHaveLength(6); // 3 built-in + 3 custom
    });

    it('should handle preview and delete interaction correctly', () => {
      const customTheme: Theme = {
        id: 'custom',
        name: 'Custom',
        version: '1.0.0',
        colors: { ...lightTheme.colors },
      };

      manager.importTheme(JSON.stringify(customTheme));

      const restore = manager.previewTheme('custom');
      manager.deleteTheme('custom');

      // Restore should still work (restores to previous, not the deleted theme)
      expect(() => restore!()).not.toThrow();
    });

    it('should handle large number of custom themes', () => {
      const themes = Array.from({ length: 50 }, (_, i) => ({
        id: `theme-${i}`,
        name: `Theme ${i}`,
        version: '1.0.0',
        colors: { ...lightTheme.colors },
      }));

      themes.forEach((theme) => {
        manager.importTheme(JSON.stringify(theme));
      });

      const allThemes = manager.getAllThemes();
      expect(allThemes).toHaveLength(53); // 3 built-in + 50 custom
    });

    it('should handle theme with maximum valid color values', () => {
      const extremeTheme: Theme = {
        id: 'extreme',
        name: 'Extreme',
        version: '1.0.0',
        colors: {
          background: '359 100% 100%',
          foreground: '0 0% 0%',
          card: '359 100% 100%',
          'card-foreground': '0 0% 0%',
          popover: '359 100% 100%',
          'popover-foreground': '0 0% 0%',
          primary: '359 100% 50%',
          'primary-foreground': '0 0% 100%',
          secondary: '180 100% 50%',
          'secondary-foreground': '0 0% 100%',
          muted: '0 0% 90%',
          'muted-foreground': '0 0% 10%',
          accent: '120 100% 50%',
          'accent-foreground': '0 0% 0%',
          destructive: '0 100% 50%',
          'destructive-foreground': '0 0% 100%',
          border: '0 0% 80%',
          input: '0 0% 80%',
          ring: '240 100% 50%',
        },
      };

      const result = manager.importTheme(JSON.stringify(extremeTheme));
      expect(result.valid).toBe(true);

      const success = manager.setCurrentTheme('extreme');
      expect(success).toBe(true);
    });
  });
});
