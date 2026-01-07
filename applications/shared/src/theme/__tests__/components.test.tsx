/**
 * Comprehensive component tests for theme UI components
 * Focus: User interactions, import/export workflows, visual feedback, edge cases
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
// import userEvent from '@testing-library/user-event';
import { ThemeProvider } from '../ThemeProvider';
import { ThemePicker } from '../components/ThemePicker';
import { ThemeSwitcher } from '../components/ThemeSwitcher';
import { ThemePreview } from '../components/ThemePreview';
import { lightTheme, oceanTheme } from '../themes';
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

// Mock URL.createObjectURL and revokeObjectURL for file downloads
global.URL.createObjectURL = vi.fn(() => 'blob:mock-url');
global.URL.revokeObjectURL = vi.fn();

describe('ThemePicker Component', () => {
  beforeEach(() => {
    localStorageMock.clear();
    htmlElementMock.setAttribute.mockClear();
    htmlElementMock.style.setProperty.mockClear();
  });

  afterEach(() => {
    localStorageMock.clear();
  });

  describe('rendering', () => {
    it('should render theme management section', () => {
      render(
        <ThemeProvider>
          <ThemePicker />
        </ThemeProvider>
      );

      expect(screen.getByText('Theme Management')).toBeInTheDocument();
      expect(screen.getByText('Import Theme')).toBeInTheDocument();
      expect(screen.getByText('Export Current Theme')).toBeInTheDocument();
    });

    it('should render built-in themes section', () => {
      render(
        <ThemeProvider>
          <ThemePicker />
        </ThemeProvider>
      );

      expect(screen.getByText('Built-in Themes')).toBeInTheDocument();
      expect(screen.getByText('Light')).toBeInTheDocument();
      expect(screen.getByText('Dark')).toBeInTheDocument();
      expect(screen.getByText('Ocean')).toBeInTheDocument();
    });

    it('should render current theme info section', () => {
      render(
        <ThemeProvider>
          <ThemePicker showAccessibilityInfo={true} />
        </ThemeProvider>
      );

      expect(screen.getByText('Current Theme Info')).toBeInTheDocument();
      expect(screen.getByText(/Name:/)).toBeInTheDocument();
      expect(screen.getByText(/Version:/)).toBeInTheDocument();
    });

    it('should hide import/export when showImportExport is false', () => {
      render(
        <ThemeProvider>
          <ThemePicker showImportExport={false} />
        </ThemeProvider>
      );

      expect(screen.queryByText('Theme Management')).not.toBeInTheDocument();
      expect(screen.queryByText('Import Theme')).not.toBeInTheDocument();
    });

    it('should hide accessibility info when showAccessibilityInfo is false', () => {
      render(
        <ThemeProvider>
          <ThemePicker showAccessibilityInfo={false} />
        </ThemeProvider>
      );

      expect(screen.queryByText('Current Theme Info')).not.toBeInTheDocument();
    });
  });

  describe('theme selection', () => {
    it('should select theme when clicked', async () => {
      render(
        <ThemeProvider>
          <ThemePicker />
        </ThemeProvider>
      );

      const darkThemeCard = screen.getByText('Dark').closest('[role="button"]');

      expect(darkThemeCard).toBeInTheDocument();

      if (darkThemeCard) {
        fireEvent.click(darkThemeCard);
      }

      await waitFor(() => {
        expect(htmlElementMock.setAttribute).toHaveBeenCalledWith('data-theme', 'dark');
      });
    });

    it('should mark active theme visually', () => {
      render(
        <ThemeProvider>
          <ThemePicker />
        </ThemeProvider>
      );

      // Light theme should be active by default
      const lightCard = screen.getByText('Light').closest('div');
      expect(lightCard?.textContent).toContain('Active');
    });

    it('should update active marker when theme changes', async () => {
      render(
        <ThemeProvider>
          <ThemePicker />
        </ThemeProvider>
      );

      const darkCard = screen.getByText('Dark').closest('[role="button"]');

      if (darkCard) {
        fireEvent.click(darkCard);
      }

      await waitFor(() => {
        const darkCardUpdated = screen.getByText('Dark').closest('div');
        expect(darkCardUpdated?.textContent).toContain('Active');
      });
    });
  });

  describe('import functionality', () => {
    it('should show success message on successful import', async () => {
      render(
        <ThemeProvider>
          <ThemePicker />
        </ThemeProvider>
      );

      const importBtn = screen.getByText('Import Theme');

      // Create a mock file
      const customTheme: Theme = {
        id: 'imported-test',
        name: 'Imported Test',
        version: '1.0.0',
        colors: { ...lightTheme.colors },
      };

      const file = new File([JSON.stringify(customTheme)], 'theme.json', {
        type: 'application/json',
      });

      // Create a mock file input change event
      const input = document.createElement('input');
      input.type = 'file';

      // Spy on createElement to intercept file input creation
      const createElementSpy = vi.spyOn(document, 'createElement');
      createElementSpy.mockImplementation((tagName) => {
        if (tagName === 'input') {
          return input;
        }
        return document.createElement(tagName);
      });

      // Trigger import button
      fireEvent.click(importBtn);

      // Simulate file selection
      Object.defineProperty(input, 'files', {
        value: [file],
        writable: false,
      });

      // Trigger the file read
      await act(async () => {
        if (input.onchange) {
          await input.onchange({ target: input } as any);
        }
      });

      createElementSpy.mockRestore();

      await waitFor(() => {
        expect(screen.getByText(/imported successfully/i)).toBeInTheDocument();
      });
    });

    it('should show error message on invalid JSON', async () => {
      render(
        <ThemeProvider>
          <ThemePicker />
        </ThemeProvider>
      );

      const importBtn = screen.getByText('Import Theme');

      const file = new File(['invalid json{{{'], 'theme.json', {
        type: 'application/json',
      });

      const input = document.createElement('input');
      input.type = 'file';

      const createElementSpy = vi.spyOn(document, 'createElement');
      createElementSpy.mockImplementation((tagName) => {
        if (tagName === 'input') {
          return input;
        }
        return document.createElement(tagName);
      });

      fireEvent.click(importBtn);

      Object.defineProperty(input, 'files', {
        value: [file],
        writable: false,
      });

      await act(async () => {
        if (input.onchange) {
          await input.onchange({ target: input } as any);
        }
      });

      createElementSpy.mockRestore();

      await waitFor(() => {
        expect(screen.getByText(/Import Failed/i)).toBeInTheDocument();
      });
    });

    it('should display custom themes section after import', async () => {
      render(
        <ThemeProvider>
          <ThemePicker />
        </ThemeProvider>
      );

      // Initially no custom themes section
      expect(screen.queryByText('Custom Themes')).not.toBeInTheDocument();

      // Import a theme programmatically (simpler than file input)
      const customTheme: Theme = {
        id: 'custom',
        name: 'Custom',
        version: '1.0.0',
        colors: { ...lightTheme.colors },
      };

      localStorageMock.setItem('soul-player-custom-themes', JSON.stringify([customTheme]));

      // Re-render with custom theme
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

      await waitFor(() => {
        expect(screen.getByText('Custom Themes')).toBeInTheDocument();
      });
    });
  });

  describe('export functionality', () => {
    it('should trigger download when export button is clicked', () => {
      render(
        <ThemeProvider>
          <ThemePicker />
        </ThemeProvider>
      );

      const exportBtn = screen.getByText('Export Current Theme');

      // Mock document.createElement and appendChild for download link
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

      fireEvent.click(exportBtn);

      expect(clickSpy).toHaveBeenCalled();
      expect(global.URL.createObjectURL).toHaveBeenCalled();

      createElementSpy.mockRestore();
      appendChildSpy.mockRestore();
      removeChildSpy.mockRestore();
    });
  });

  describe('delete functionality', () => {
    it('should show delete confirmation dialog', async () => {
      // Setup with a custom theme
      const customTheme: Theme = {
        id: 'deletable',
        name: 'Deletable',
        version: '1.0.0',
        colors: { ...lightTheme.colors },
      };

      localStorageMock.setItem('soul-player-custom-themes', JSON.stringify([customTheme]));

      render(
        <ThemeProvider>
          <ThemePicker />
        </ThemeProvider>
      );

      await waitFor(() => {
        expect(screen.getByText('Custom Themes')).toBeInTheDocument();
      });

      // Find and click delete button
      const deleteBtn = screen.getByTitle('Delete theme');
      fireEvent.click(deleteBtn);

      await waitFor(() => {
        expect(screen.getByText(/Delete "Deletable"/i)).toBeInTheDocument();
      });
    });

    it('should cancel deletion when cancel is clicked', async () => {
      const customTheme: Theme = {
        id: 'cancelable',
        name: 'Cancelable',
        version: '1.0.0',
        colors: { ...lightTheme.colors },
      };

      localStorageMock.setItem('soul-player-custom-themes', JSON.stringify([customTheme]));

      render(
        <ThemeProvider>
          <ThemePicker />
        </ThemeProvider>
      );

      await waitFor(() => {
        expect(screen.getByText('Cancelable')).toBeInTheDocument();
      });

      const deleteBtn = screen.getByTitle('Delete theme');
      fireEvent.click(deleteBtn);

      await waitFor(() => {
        const cancelBtn = screen.getByText('Cancel');
        fireEvent.click(cancelBtn);
      });

      await waitFor(() => {
        expect(screen.queryByText(/Delete "Cancelable"/i)).not.toBeInTheDocument();
      });
    });

    it('should remove theme when delete is confirmed', async () => {
      const customTheme: Theme = {
        id: 'removed',
        name: 'Removed',
        version: '1.0.0',
        colors: { ...lightTheme.colors },
      };

      localStorageMock.setItem('soul-player-custom-themes', JSON.stringify([customTheme]));

      render(
        <ThemeProvider>
          <ThemePicker />
        </ThemeProvider>
      );

      await waitFor(() => {
        expect(screen.getByText('Removed')).toBeInTheDocument();
      });

      const deleteBtn = screen.getByTitle('Delete theme');
      fireEvent.click(deleteBtn);

      await waitFor(() => {
        const confirmBtn = screen.getByText('Delete');
        fireEvent.click(confirmBtn);
      });

      await waitFor(() => {
        expect(screen.queryByText('Removed')).not.toBeInTheDocument();
        expect(screen.queryByText('Custom Themes')).not.toBeInTheDocument();
      });
    });
  });
});

describe('ThemeSwitcher Component', () => {
  beforeEach(() => {
    localStorageMock.clear();
  });

  describe('rendering', () => {
    it('should render dropdown with all themes', () => {
      render(
        <ThemeProvider>
          <ThemeSwitcher />
        </ThemeProvider>
      );

      const select = screen.getByRole('combobox');
      expect(select).toBeInTheDocument();

      const options = screen.getAllByRole('option');
      expect(options).toHaveLength(3); // 3 built-in themes
    });

    it('should show current theme as selected', () => {
      render(
        <ThemeProvider>
          <ThemeSwitcher />
        </ThemeProvider>
      );

      const select = screen.getByRole('combobox') as HTMLSelectElement;
      expect(select.value).toBe('light');
    });

    it('should display theme description', () => {
      render(
        <ThemeProvider>
          <ThemeSwitcher />
        </ThemeProvider>
      );

      expect(screen.getByText(lightTheme.description!)).toBeInTheDocument();
    });
  });

  describe('theme switching', () => {
    it('should change theme when option is selected', async () => {
      render(
        <ThemeProvider>
          <ThemeSwitcher />
        </ThemeProvider>
      );

      const select = screen.getByRole('combobox');

      fireEvent.change(select, { target: { value: 'dark' } });

      await waitFor(() => {
        expect(htmlElementMock.setAttribute).toHaveBeenCalledWith('data-theme', 'dark');
      });
    });

    it('should update description when theme changes', async () => {
      render(
        <ThemeProvider>
          <ThemeSwitcher />
        </ThemeProvider>
      );

      const select = screen.getByRole('combobox');

      fireEvent.change(select, { target: { value: 'ocean' } });

      await waitFor(() => {
        expect(screen.getByText(oceanTheme.description!)).toBeInTheDocument();
      });
    });
  });

  describe('live preview', () => {
    it('should preview theme on mouse enter when enabled', async () => {
      render(
        <ThemeProvider>
          <ThemeSwitcher showLivePreview={true} />
        </ThemeProvider>
      );

      const options = screen.getAllByRole('option');
      const darkOption = options.find((opt) => opt.textContent === 'Dark');

      if (darkOption) {
        fireEvent.mouseEnter(darkOption);

        await waitFor(() => {
          // Preview should be applied
          expect(htmlElementMock.setAttribute).toHaveBeenCalledWith('data-theme', 'dark');
        });
      }
    });

    it('should restore theme on mouse leave', async () => {
      render(
        <ThemeProvider>
          <ThemeSwitcher showLivePreview={true} />
        </ThemeProvider>
      );

      const select = screen.getByRole('combobox');

      // Simulate mouse enter and leave
      fireEvent.mouseEnter(select);
      fireEvent.mouseLeave(select);

      await waitFor(() => {
        // Should restore to original theme (light)
        expect(htmlElementMock.setAttribute).toHaveBeenCalledWith('data-theme', 'light');
      });
    });

    it('should not preview when showLivePreview is false', async () => {
      render(
        <ThemeProvider>
          <ThemeSwitcher showLivePreview={false} />
        </ThemeProvider>
      );

      htmlElementMock.setAttribute.mockClear();

      const options = screen.getAllByRole('option');
      const darkOption = options.find((opt) => opt.textContent === 'Dark');

      if (darkOption) {
        fireEvent.mouseEnter(darkOption);

        // Should not apply theme on hover
        expect(htmlElementMock.setAttribute).not.toHaveBeenCalled();
      }
    });
  });
});

describe('ThemePreview Component', () => {
  describe('rendering', () => {
    it('should render theme name and description', () => {
      render(<ThemePreview theme={oceanTheme} />);

      expect(screen.getByText('Ocean')).toBeInTheDocument();
      expect(screen.getByText(oceanTheme.description!)).toBeInTheDocument();
    });

    it('should render version and author info', () => {
      render(<ThemePreview theme={oceanTheme} />);

      expect(screen.getByText(/v1.0.0/)).toBeInTheDocument();
      expect(screen.getByText(/Built-in/)).toBeInTheDocument();
    });

    it('should show active indicator when theme is active', () => {
      render(<ThemePreview theme={lightTheme} isActive={true} />);

      expect(screen.getByText(/Active/)).toBeInTheDocument();
    });

    it('should not show active indicator when theme is not active', () => {
      render(<ThemePreview theme={lightTheme} isActive={false} />);

      expect(screen.queryByText(/Active/)).not.toBeInTheDocument();
    });

    it('should display color swatches', () => {
      render(<ThemePreview theme={oceanTheme} />);

      // Should have multiple color swatches (at least 6 main colors)
      const swatches = screen.getAllByLabelText(/background|foreground|primary|secondary|accent|destructive/i);
      expect(swatches.length).toBeGreaterThanOrEqual(6);
    });

    it('should display gradient preview if available', () => {
      render(<ThemePreview theme={oceanTheme} />);

      // Ocean theme has gradients - check for gradient container
      const gradients = screen.getByText('Ocean').closest('div')?.querySelector('[style*="gradient"]');
      expect(gradients).toBeInTheDocument();
    });
  });

  describe('interaction', () => {
    it('should call onClick when clicked', () => {
      const handleClick = vi.fn();

      render(<ThemePreview theme={oceanTheme} onClick={handleClick} />);

      const preview = screen.getByRole('button');
      fireEvent.click(preview);

      expect(handleClick).toHaveBeenCalledTimes(1);
    });

    it('should call onClick on Enter key', () => {
      const handleClick = vi.fn();

      render(<ThemePreview theme={oceanTheme} onClick={handleClick} />);

      const preview = screen.getByRole('button');
      fireEvent.keyDown(preview, { key: 'Enter', code: 'Enter' });

      expect(handleClick).toHaveBeenCalledTimes(1);
    });

    it('should call onClick on Space key', () => {
      const handleClick = vi.fn();

      render(<ThemePreview theme={oceanTheme} onClick={handleClick} />);

      const preview = screen.getByRole('button');
      fireEvent.keyDown(preview, { key: ' ', code: 'Space' });

      expect(handleClick).toHaveBeenCalledTimes(1);
    });

    it('should be keyboard accessible', () => {
      render(<ThemePreview theme={oceanTheme} onClick={() => {}} />);

      const preview = screen.getByRole('button');
      expect(preview).toHaveAttribute('tabIndex', '0');
    });
  });

  describe('visual states', () => {
    it('should apply active styling when isActive is true', () => {
      render(<ThemePreview theme={oceanTheme} isActive={true} />);

      const preview = screen.getByRole('button');
      expect(preview.className).toContain('border-primary');
    });

    it('should apply default styling when isActive is false', () => {
      render(<ThemePreview theme={oceanTheme} isActive={false} />);

      const preview = screen.getByRole('button');
      expect(preview.className).toContain('border-border');
    });

    it('should apply custom className', () => {
      render(<ThemePreview theme={oceanTheme} className="custom-class" />);

      const preview = screen.getByRole('button');
      expect(preview.className).toContain('custom-class');
    });
  });
});
