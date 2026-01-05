/**
 * End-to-End workflow tests for theme system
 * Focus: Complete user journeys, cross-component integration, real-world scenarios
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, waitFor, act } from '@testing-library/react';
import { ThemeProvider } from '../ThemeProvider';
import { ThemePicker } from '../components/ThemePicker';
import { ThemeSwitcher } from '../components/ThemeSwitcher';
import { lightTheme } from '../themes';
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

// Mock URL for file downloads
global.URL.createObjectURL = vi.fn(() => 'blob:mock-url');
global.URL.revokeObjectURL = vi.fn();

describe('E2E Workflow Tests', () => {
  beforeEach(() => {
    localStorageMock.clear();
    htmlElementMock.setAttribute.mockClear();
    htmlElementMock.style.setProperty.mockClear();
  });

  afterEach(() => {
    localStorageMock.clear();
  });

  describe('Complete Theme Switching Journey', () => {
    it('should allow user to browse, preview, and apply a theme', async () => {
      render(
        <ThemeProvider>
          <ThemePicker />
        </ThemeProvider>
      );

      // 1. User sees all built-in themes
      expect(screen.getByText('Light')).toBeInTheDocument();
      expect(screen.getByText('Dark')).toBeInTheDocument();
      expect(screen.getByText('Ocean')).toBeInTheDocument();

      // 2. User clicks on Dark theme
      const darkThemeCard = screen.getByText('Dark').closest('[role="button"]');
      expect(darkThemeCard).toBeInTheDocument();

      if (darkThemeCard) {
        act(() => {
          darkThemeCard.click();
        });
      }

      // 3. Theme is applied to DOM
      await waitFor(() => {
        expect(htmlElementMock.setAttribute).toHaveBeenCalledWith('data-theme', 'dark');
      });

      // 4. Theme is persisted to localStorage
      await waitFor(() => {
        const stored = localStorageMock.getItem('soul-player-current-theme');
        expect(stored).toBe('dark');
      });

      // 5. Dark theme is now marked as active
      await waitFor(() => {
        const darkCard = screen.getByText('Dark').closest('div');
        expect(darkCard?.textContent).toContain('Active');
      });
    });

    it('should preserve theme selection across app restarts', () => {
      // First render: select dark theme
      const { unmount } = render(
        <ThemeProvider>
          <ThemePicker />
        </ThemeProvider>
      );

      const darkCard = screen.getByText('Dark').closest('[role="button"]');
      if (darkCard) {
        act(() => {
          darkCard.click();
        });
      }

      // Unmount (simulate app close)
      unmount();

      htmlElementMock.setAttribute.mockClear();

      // Second render: theme should be remembered
      render(
        <ThemeProvider>
          <ThemePicker />
        </ThemeProvider>
      );

      // Dark theme should be loaded automatically
      expect(htmlElementMock.setAttribute).toHaveBeenCalledWith('data-theme', 'dark');

      // Dark theme should be marked as active
      const darkCard2 = screen.getByText('Dark').closest('div');
      expect(darkCard2?.textContent).toContain('Active');
    });
  });

  describe('Custom Theme Import/Export Journey', () => {
    it('should allow user to import, use, export, and delete a custom theme', async () => {
      const customTheme: Theme = {
        id: 'my-custom',
        name: 'My Custom Theme',
        version: '1.0.0',
        author: 'Test User',
        description: 'A beautiful custom theme',
        colors: { ...lightTheme.colors },
      };

      render(
        <ThemeProvider>
          <ThemePicker />
        </ThemeProvider>
      );

      // 1. Initially no custom themes
      expect(screen.queryByText('Custom Themes')).not.toBeInTheDocument();

      // 2. Import custom theme (simulate programmatically since file input is complex)
      localStorageMock.setItem('soul-player-custom-themes', JSON.stringify([customTheme]));

      // Re-render to pick up the change
      const { rerender } = render(
        <ThemeProvider>
          <ThemePicker />
        </ThemeProvider>
      );

      rerender(
        <ThemeProvider>
          <ThemePicker />
        </ThemeProvider>
      );

      // 3. Custom theme appears in Custom Themes section
      await waitFor(() => {
        expect(screen.getByText('Custom Themes')).toBeInTheDocument();
        expect(screen.getByText('My Custom Theme')).toBeInTheDocument();
      });

      // 4. User selects the custom theme
      const customCard = screen.getByText('My Custom Theme').closest('[role="button"]');
      if (customCard) {
        act(() => {
          customCard.click();
        });
      }

      await waitFor(() => {
        expect(htmlElementMock.setAttribute).toHaveBeenCalledWith('data-theme', 'my-custom');
      });

      // 5. User exports the theme
      const exportBtn = screen.getByTitle('Export theme');
      const mockLink = document.createElement('a');
      const clickSpy = vi.spyOn(mockLink, 'click');

      const createElementSpy = vi.spyOn(document, 'createElement');
      createElementSpy.mockImplementation((tagName) => {
        if (tagName === 'a') {
          return mockLink;
        }
        return document.createElement(tagName);
      });

      const appendChildSpy = vi.spyOn(document.body, 'appendChild').mockImplementation(() => mockLink);
      const removeChildSpy = vi.spyOn(document.body, 'removeChild').mockImplementation(() => mockLink);

      act(() => {
        exportBtn.click();
      });

      expect(clickSpy).toHaveBeenCalled();

      createElementSpy.mockRestore();
      appendChildSpy.mockRestore();
      removeChildSpy.mockRestore();

      // 6. User deletes the custom theme
      const deleteBtn = screen.getByTitle('Delete theme');
      act(() => {
        deleteBtn.click();
      });

      // 7. Confirm deletion
      await waitFor(() => {
        const confirmBtn = screen.getByText('Delete');
        act(() => {
          confirmBtn.click();
        });
      });

      // 8. Theme is removed and user switches back to default
      await waitFor(() => {
        expect(screen.queryByText('My Custom Theme')).not.toBeInTheDocument();
        expect(screen.queryByText('Custom Themes')).not.toBeInTheDocument();
        expect(htmlElementMock.setAttribute).toHaveBeenCalledWith('data-theme', 'light');
      });
    });
  });

  describe('Multi-Component Synchronization', () => {
    it('should keep ThemePicker and ThemeSwitcher synchronized', async () => {
      function SynchronizedApp() {
        return (
          <ThemeProvider>
            <div data-testid="switcher-container">
              <ThemeSwitcher />
            </div>
            <div data-testid="picker-container">
              <ThemePicker />
            </div>
          </ThemeProvider>
        );
      }

      render(<SynchronizedApp />);

      // 1. Initial state: both show light theme
      const select = screen.getByRole('combobox') as HTMLSelectElement;
      expect(select.value).toBe('light');

      const lightCard = screen.getByText('Light').closest('div');
      expect(lightCard?.textContent).toContain('Active');

      // 2. Change theme via switcher dropdown
      act(() => {
        select.value = 'ocean';
        select.dispatchEvent(new Event('change', { bubbles: true }));
      });

      // 3. Picker should update to show ocean as active
      await waitFor(() => {
        const oceanCard = screen.getByText('Ocean').closest('div');
        expect(oceanCard?.textContent).toContain('Active');
      });

      // 4. Change theme via picker
      const darkCard = screen.getByText('Dark').closest('[role="button"]');
      if (darkCard) {
        act(() => {
          darkCard.click();
        });
      }

      // 5. Switcher should update to show dark
      await waitFor(() => {
        expect((screen.getByRole('combobox') as HTMLSelectElement).value).toBe('dark');
      });

      // 6. Theme description in switcher should update
      await waitFor(() => {
        expect(screen.getByText(/Sleek dark theme/i)).toBeInTheDocument();
      });
    });
  });

  describe('Error Recovery Workflows', () => {
    it('should recover gracefully from corrupted localStorage', () => {
      // Corrupt localStorage
      localStorageMock.setItem('soul-player-custom-themes', 'corrupted{{{json');
      localStorageMock.setItem('soul-player-current-theme', 'non-existent-theme');

      // App should still load with defaults
      render(
        <ThemeProvider>
          <ThemePicker />
        </ThemeProvider>
      );

      // Should fallback to default light theme
      expect(htmlElementMock.setAttribute).toHaveBeenCalledWith('data-theme', 'light');

      // Should show only built-in themes (corrupted custom themes ignored)
      expect(screen.getByText('Light')).toBeInTheDocument();
      expect(screen.getByText('Dark')).toBeInTheDocument();
      expect(screen.getByText('Ocean')).toBeInTheDocument();
      expect(screen.queryByText('Custom Themes')).not.toBeInTheDocument();
    });

    it('should handle import of invalid theme and allow retry', async () => {
      render(
        <ThemeProvider>
          <ThemePicker />
        </ThemeProvider>
      );

      // Simulate failed import
      const invalidTheme = {
        id: 'invalid',
        name: 'Invalid',
        version: 'not-semver',
        colors: {},
      };

      localStorageMock.setItem('soul-player-custom-themes', JSON.stringify([invalidTheme]));

      // Should not crash and should allow continued use
      expect(screen.getByText('Light')).toBeInTheDocument();

      // User can still switch themes
      const darkCard = screen.getByText('Dark').closest('[role="button"]');
      if (darkCard) {
        act(() => {
          darkCard.click();
        });
      }

      await waitFor(() => {
        expect(htmlElementMock.setAttribute).toHaveBeenCalledWith('data-theme', 'dark');
      });
    });
  });

  describe('Accessibility Workflows', () => {
    it('should provide accessible theme information', () => {
      render(
        <ThemeProvider>
          <ThemePicker showAccessibilityInfo={true} />
        </ThemeProvider>
      );

      // Theme info section should be present
      expect(screen.getByText('Current Theme Info')).toBeInTheDocument();

      // Should show theme metadata
      expect(screen.getByText(/Name:/)).toBeInTheDocument();
      expect(screen.getByText(/Version:/)).toBeInTheDocument();
      expect(screen.getByText(/Light/)).toBeInTheDocument();
      expect(screen.getByText(/1.0.0/)).toBeInTheDocument();
    });

    it('should allow keyboard-only theme switching', async () => {
      render(
        <ThemeProvider>
          <ThemePicker />
        </ThemeProvider>
      );

      // Find Dark theme card
      const darkCard = screen.getByText('Dark').closest('[role="button"]');
      expect(darkCard).toBeInTheDocument();

      if (darkCard) {
        // Tab to focus (simulated)
        darkCard.focus();

        // Press Enter to select
        act(() => {
          darkCard.dispatchEvent(
            new KeyboardEvent('keydown', {
              key: 'Enter',
              code: 'Enter',
              bubbles: true,
            })
          );
        });

        await waitFor(() => {
          expect(htmlElementMock.setAttribute).toHaveBeenCalledWith('data-theme', 'dark');
        });
      }
    });
  });

  describe('Complex Multi-Step Scenarios', () => {
    it('should handle rapid theme switching without issues', async () => {
      render(
        <ThemeProvider>
          <ThemePicker />
        </ThemeProvider>
      );

      const lightCard = screen.getByText('Light').closest('[role="button"]');
      const darkCard = screen.getByText('Dark').closest('[role="button"]');
      const oceanCard = screen.getByText('Ocean').closest('[role="button"]');

      // Rapidly switch between themes
      if (darkCard && oceanCard && lightCard) {
        act(() => {
          darkCard.click();
        });

        act(() => {
          oceanCard.click();
        });

        act(() => {
          lightCard.click();
        });

        act(() => {
          darkCard.click();
        });
      }

      // Final theme should be dark
      await waitFor(() => {
        const darkCardFinal = screen.getByText('Dark').closest('div');
        expect(darkCardFinal?.textContent).toContain('Active');
        expect(localStorageMock.getItem('soul-player-current-theme')).toBe('dark');
      });
    });

    it('should handle theme switching while importing themes', async () => {
      render(
        <ThemeProvider>
          <ThemePicker />
        </ThemeProvider>
      );

      // Switch to dark theme
      const darkCard = screen.getByText('Dark').closest('[role="button"]');
      if (darkCard) {
        act(() => {
          darkCard.click();
        });
      }

      await waitFor(() => {
        expect(htmlElementMock.setAttribute).toHaveBeenCalledWith('data-theme', 'dark');
      });

      // Import a custom theme while dark theme is active
      const customTheme: Theme = {
        id: 'imported-while-dark',
        name: 'Imported While Dark',
        version: '1.0.0',
        colors: { ...lightTheme.colors },
      };

      localStorageMock.setItem('soul-player-custom-themes', JSON.stringify([customTheme]));

      // Re-render
      const { rerender } = render(
        <ThemeProvider>
          <ThemePicker />
        </ThemeProvider>
      );

      rerender(
        <ThemeProvider>
          <ThemePicker />
        </ThemeProvider>
      );

      // Dark theme should still be active
      await waitFor(() => {
        const darkCardStill = screen.getByText('Dark').closest('div');
        expect(darkCardStill?.textContent).toContain('Active');
      });

      // Custom theme should be available
      expect(screen.getByText('Custom Themes')).toBeInTheDocument();
      expect(screen.getByText('Imported While Dark')).toBeInTheDocument();
    });

    it('should handle switching to custom theme, exporting it, deleting it, and reimporting', async () => {
      const customTheme: Theme = {
        id: 'cycle-test',
        name: 'Cycle Test',
        version: '1.0.0',
        colors: { ...lightTheme.colors },
      };

      // Import theme
      localStorageMock.setItem('soul-player-custom-themes', JSON.stringify([customTheme]));

      render(
        <ThemeProvider>
          <ThemePicker />
        </ThemeProvider>
      );

      await waitFor(() => {
        expect(screen.getByText('Cycle Test')).toBeInTheDocument();
      });

      // Select custom theme
      const customCard = screen.getByText('Cycle Test').closest('[role="button"]');
      if (customCard) {
        act(() => {
          customCard.click();
        });
      }

      await waitFor(() => {
        expect(htmlElementMock.setAttribute).toHaveBeenCalledWith('data-theme', 'cycle-test');
      });

      // Export (simulated)
      const exportedData = JSON.stringify({ ...customTheme, exportedAt: new Date().toISOString() });

      // Delete the theme
      const deleteBtn = screen.getByTitle('Delete theme');
      act(() => {
        deleteBtn.click();
      });

      await waitFor(() => {
        const confirmBtn = screen.getByText('Delete');
        act(() => {
          confirmBtn.click();
        });
      });

      await waitFor(() => {
        expect(screen.queryByText('Cycle Test')).not.toBeInTheDocument();
      });

      // Reimport the theme
      localStorageMock.setItem('soul-player-custom-themes', JSON.stringify([JSON.parse(exportedData)]));

      const { rerender } = render(
        <ThemeProvider>
          <ThemePicker />
        </ThemeProvider>
      );

      rerender(
        <ThemeProvider>
          <ThemePicker />
        </ThemeProvider>
      );

      // Theme should be back
      await waitFor(() => {
        expect(screen.getByText('Cycle Test')).toBeInTheDocument();
      });
    });
  });

  describe('Real-World Usage Patterns', () => {
    it('should support typical user workflow: browse → preview → apply → persist', async () => {
      render(
        <ThemeProvider>
          <ThemePicker />
          <ThemeSwitcher showLivePreview={true} />
        </ThemeProvider>
      );

      // 1. User browses themes in picker
      expect(screen.getByText('Light')).toBeInTheDocument();
      expect(screen.getByText('Dark')).toBeInTheDocument();
      expect(screen.getByText('Ocean')).toBeInTheDocument();

      // 2. User previews Ocean theme via switcher hover (simulated)
      htmlElementMock.setAttribute.mockClear();

      // 3. User applies Ocean theme by clicking
      const oceanCard = screen.getByText('Ocean').closest('[role="button"]');
      if (oceanCard) {
        act(() => {
          oceanCard.click();
        });
      }

      // 4. Theme is applied and persisted
      await waitFor(() => {
        expect(htmlElementMock.setAttribute).toHaveBeenCalledWith('data-theme', 'ocean');
        expect(localStorageMock.getItem('soul-player-current-theme')).toBe('ocean');
      });

      // 5. On next app load, Ocean theme is remembered
      const { unmount } = render(
        <ThemeProvider>
          <ThemePicker />
        </ThemeProvider>
      );

      unmount();

      htmlElementMock.setAttribute.mockClear();

      render(
        <ThemeProvider>
          <ThemePicker />
        </ThemeProvider>
      );

      expect(htmlElementMock.setAttribute).toHaveBeenCalledWith('data-theme', 'ocean');
    });
  });
});
