/**
 * Comprehensive tests for ThemeProvider and useTheme hook
 * Focus: React integration, context behavior, hook usage, state management
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, waitFor, act } from '@testing-library/react';
import { ThemeProvider, useTheme } from '../ThemeProvider';
import { lightTheme, darkTheme } from '../themes';
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

// Test component that uses useTheme hook
function ThemeConsumer() {
  const { currentTheme, availableThemes, setTheme } = useTheme();

  return (
    <div>
      <div data-testid="current-theme-id">{currentTheme.id}</div>
      <div data-testid="current-theme-name">{currentTheme.name}</div>
      <div data-testid="available-count">{availableThemes.length}</div>

      <button onClick={() => setTheme('dark')} data-testid="set-dark-btn">
        Set Dark
      </button>
      <button onClick={() => setTheme('light')} data-testid="set-light-btn">
        Set Light
      </button>
    </div>
  );
}

// Test component for import/export
function ImportExportConsumer() {
  const { importTheme, exportTheme, currentTheme } = useTheme();

  const handleImport = () => {
    const customTheme: Theme = {
      id: 'custom-test',
      name: 'Custom Test',
      version: '1.0.0',
      colors: { ...lightTheme.colors },
    };

    const result = importTheme(JSON.stringify(customTheme));
    return result;
  };

  const handleExport = () => {
    const json = exportTheme(currentTheme.id);
    return json;
  };

  return (
    <div>
      <button onClick={handleImport} data-testid="import-btn">
        Import
      </button>
      <button onClick={handleExport} data-testid="export-btn">
        Export
      </button>
    </div>
  );
}

// Test component for delete
function DeleteConsumer() {
  const { deleteTheme, availableThemes } = useTheme();

  return (
    <div>
      <div data-testid="theme-count">{availableThemes.length}</div>
      <button onClick={() => deleteTheme('custom-test')} data-testid="delete-btn">
        Delete Custom
      </button>
    </div>
  );
}

// Test component for preview
function PreviewConsumer() {
  const { previewTheme } = useTheme();
  let restoreFn: (() => void) | null = null;

  const handlePreview = () => {
    restoreFn = previewTheme('ocean');
  };

  const handleRestore = () => {
    if (restoreFn) {
      restoreFn();
    }
  };

  return (
    <div>
      <button onClick={handlePreview} data-testid="preview-btn">
        Preview Ocean
      </button>
      <button onClick={handleRestore} data-testid="restore-btn">
        Restore
      </button>
    </div>
  );
}

describe('ThemeProvider', () => {
  beforeEach(() => {
    localStorageMock.clear();
    htmlElementMock.setAttribute.mockClear();
    htmlElementMock.style.setProperty.mockClear();
  });

  afterEach(() => {
    localStorageMock.clear();
  });

  describe('provider initialization', () => {
    it('should render children correctly', () => {
      render(
        <ThemeProvider>
          <div data-testid="child">Child Content</div>
        </ThemeProvider>
      );

      expect(screen.getByTestId('child')).toBeInTheDocument();
      expect(screen.getByTestId('child')).toHaveTextContent('Child Content');
    });

    it('should provide default light theme initially', () => {
      render(
        <ThemeProvider>
          <ThemeConsumer />
        </ThemeProvider>
      );

      expect(screen.getByTestId('current-theme-id')).toHaveTextContent('light');
      expect(screen.getByTestId('current-theme-name')).toHaveTextContent('Light');
    });

    it('should provide all available themes', () => {
      render(
        <ThemeProvider>
          <ThemeConsumer />
        </ThemeProvider>
      );

      // Should have 3 built-in themes
      expect(screen.getByTestId('available-count')).toHaveTextContent('3');
    });

    it('should throw error when useTheme is used outside provider', () => {
      // Suppress console.error for this test
      const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});

      expect(() => {
        render(<ThemeConsumer />);
      }).toThrow('useTheme must be used within a ThemeProvider');

      consoleError.mockRestore();
    });
  });

  describe('theme switching via hook', () => {
    it('should switch theme when setTheme is called', async () => {
      render(
        <ThemeProvider>
          <ThemeConsumer />
        </ThemeProvider>
      );

      const darkBtn = screen.getByTestId('set-dark-btn');

      act(() => {
        darkBtn.click();
      });

      await waitFor(() => {
        expect(screen.getByTestId('current-theme-id')).toHaveTextContent('dark');
      });
    });

    it('should update currentTheme in all consuming components', async () => {
      function MultipleConsumers() {
        return (
          <ThemeProvider>
            <div data-testid="consumer-1">
              <ThemeConsumer />
            </div>
            <div data-testid="consumer-2">
              <ThemeConsumer />
            </div>
          </ThemeProvider>
        );
      }

      render(<MultipleConsumers />);

      const darkBtns = screen.getAllByTestId('set-dark-btn');

      act(() => {
        darkBtns[0].click();
      });

      await waitFor(() => {
        const allThemeIds = screen.getAllByTestId('current-theme-id');
        allThemeIds.forEach((el) => {
          expect(el).toHaveTextContent('dark');
        });
      });
    });

    it('should persist theme change to localStorage', async () => {
      render(
        <ThemeProvider>
          <ThemeConsumer />
        </ThemeProvider>
      );

      const darkBtn = screen.getByTestId('set-dark-btn');

      act(() => {
        darkBtn.click();
      });

      await waitFor(() => {
        const stored = localStorageMock.getItem('soul-player-current-theme');
        expect(stored).toBe('dark');
      });
    });

    it('should apply theme to DOM when switched', async () => {
      render(
        <ThemeProvider>
          <ThemeConsumer />
        </ThemeProvider>
      );

      htmlElementMock.setAttribute.mockClear();

      const darkBtn = screen.getByTestId('set-dark-btn');

      act(() => {
        darkBtn.click();
      });

      await waitFor(() => {
        expect(htmlElementMock.setAttribute).toHaveBeenCalledWith('data-theme', 'dark');
      });
    });

    it('should handle switching to non-existent theme gracefully', async () => {
      function InvalidThemeSwitcher() {
        const { setTheme, currentTheme } = useTheme();

        return (
          <div>
            <div data-testid="current-theme-id">{currentTheme.id}</div>
            <button onClick={() => setTheme('non-existent')} data-testid="invalid-btn">
              Set Invalid
            </button>
          </div>
        );
      }

      render(
        <ThemeProvider>
          <InvalidThemeSwitcher />
        </ThemeProvider>
      );

      const initialTheme = screen.getByTestId('current-theme-id').textContent;
      const invalidBtn = screen.getByTestId('invalid-btn');

      act(() => {
        invalidBtn.click();
      });

      await waitFor(() => {
        // Theme should remain unchanged
        expect(screen.getByTestId('current-theme-id')).toHaveTextContent(initialTheme!);
      });
    });
  });

  describe('import functionality', () => {
    it('should import custom theme via hook', async () => {
      render(
        <ThemeProvider>
          <ImportExportConsumer />
          <ThemeConsumer />
        </ThemeProvider>
      );

      const importBtn = screen.getByTestId('import-btn');

      act(() => {
        importBtn.click();
      });

      await waitFor(() => {
        // Available count should increase
        expect(screen.getByTestId('available-count')).toHaveTextContent('4');
      });
    });

    it('should make imported theme available for selection', async () => {
      function ImportAndSelect() {
        const { importTheme, setTheme, currentTheme } = useTheme();

        const handleImportAndSelect = () => {
          const customTheme: Theme = {
            id: 'imported',
            name: 'Imported Theme',
            version: '1.0.0',
            colors: { ...lightTheme.colors },
          };

          importTheme(JSON.stringify(customTheme));
          setTheme('imported');
        };

        return (
          <div>
            <div data-testid="current-theme-name">{currentTheme.name}</div>
            <button onClick={handleImportAndSelect} data-testid="import-select-btn">
              Import and Select
            </button>
          </div>
        );
      }

      render(
        <ThemeProvider>
          <ImportAndSelect />
        </ThemeProvider>
      );

      const btn = screen.getByTestId('import-select-btn');

      act(() => {
        btn.click();
      });

      await waitFor(() => {
        expect(screen.getByTestId('current-theme-name')).toHaveTextContent('Imported Theme');
      });
    });

    it('should return validation errors for invalid theme', async () => {
      function ImportInvalid() {
        const { importTheme } = useTheme();
        const [result, setResult] = React.useState<any>(null);

        const handleImport = () => {
          const invalidTheme = {
            id: 'Invalid ID!',
            name: 'Invalid',
            version: '1.0', // Invalid semver
            colors: {}, // Missing colors
          };

          const res = importTheme(JSON.stringify(invalidTheme));
          setResult(res);
        };

        return (
          <div>
            <button onClick={handleImport} data-testid="import-invalid-btn">
              Import Invalid
            </button>
            {result && (
              <div data-testid="result-valid">{result.valid ? 'true' : 'false'}</div>
            )}
          </div>
        );
      }

      render(
        <ThemeProvider>
          <ImportInvalid />
        </ThemeProvider>
      );

      const btn = screen.getByTestId('import-invalid-btn');

      act(() => {
        btn.click();
      });

      await waitFor(() => {
        expect(screen.getByTestId('result-valid')).toHaveTextContent('false');
      });
    });
  });

  describe('export functionality', () => {
    it('should export current theme as JSON', async () => {
      function ExportTester() {
        const { exportTheme } = useTheme();
        const [exportedJson, setExportedJson] = React.useState<string | null>(null);

        const handleExport = () => {
          const json = exportTheme('light');
          setExportedJson(json);
        };

        return (
          <div>
            <button onClick={handleExport} data-testid="export-btn">
              Export
            </button>
            {exportedJson && <div data-testid="exported-data">Exported</div>}
          </div>
        );
      }

      render(
        <ThemeProvider>
          <ExportTester />
        </ThemeProvider>
      );

      const btn = screen.getByTestId('export-btn');

      act(() => {
        btn.click();
      });

      await waitFor(() => {
        expect(screen.getByTestId('exported-data')).toBeInTheDocument();
      });
    });

    it('should return null when exporting non-existent theme', async () => {
      function ExportNonExistent() {
        const { exportTheme } = useTheme();
        const [result, setResult] = React.useState<string | null>(undefined);

        const handleExport = () => {
          const json = exportTheme('non-existent');
          setResult(json);
        };

        return (
          <div>
            <button onClick={handleExport} data-testid="export-nonexistent-btn">
              Export Non-existent
            </button>
            {result !== undefined && (
              <div data-testid="result">{result === null ? 'null' : 'not-null'}</div>
            )}
          </div>
        );
      }

      render(
        <ThemeProvider>
          <ExportNonExistent />
        </ThemeProvider>
      );

      const btn = screen.getByTestId('export-nonexistent-btn');

      act(() => {
        btn.click();
      });

      await waitFor(() => {
        expect(screen.getByTestId('result')).toHaveTextContent('null');
      });
    });
  });

  describe('delete functionality', () => {
    it('should delete custom theme via hook', async () => {
      function ImportAndDelete() {
        const { importTheme, deleteTheme, availableThemes } = useTheme();

        const handleImport = () => {
          const customTheme: Theme = {
            id: 'deletable',
            name: 'Deletable',
            version: '1.0.0',
            colors: { ...lightTheme.colors },
          };

          importTheme(JSON.stringify(customTheme));
        };

        const handleDelete = () => {
          deleteTheme('deletable');
        };

        return (
          <div>
            <div data-testid="theme-count">{availableThemes.length}</div>
            <button onClick={handleImport} data-testid="import-btn">
              Import
            </button>
            <button onClick={handleDelete} data-testid="delete-btn">
              Delete
            </button>
          </div>
        );
      }

      render(
        <ThemeProvider>
          <ImportAndDelete />
        </ThemeProvider>
      );

      // Initial count: 3
      expect(screen.getByTestId('theme-count')).toHaveTextContent('3');

      // Import theme
      act(() => {
        screen.getByTestId('import-btn').click();
      });

      await waitFor(() => {
        expect(screen.getByTestId('theme-count')).toHaveTextContent('4');
      });

      // Delete theme
      act(() => {
        screen.getByTestId('delete-btn').click();
      });

      await waitFor(() => {
        expect(screen.getByTestId('theme-count')).toHaveTextContent('3');
      });
    });

    it('should switch to default theme when deleting current theme', async () => {
      function DeleteCurrentTheme() {
        const { importTheme, setTheme, deleteTheme, currentTheme } = useTheme();

        const handleSetup = () => {
          const customTheme: Theme = {
            id: 'temporary',
            name: 'Temporary',
            version: '1.0.0',
            colors: { ...lightTheme.colors },
          };

          importTheme(JSON.stringify(customTheme));
          setTheme('temporary');
        };

        const handleDelete = () => {
          deleteTheme('temporary');
        };

        return (
          <div>
            <div data-testid="current-theme-id">{currentTheme.id}</div>
            <button onClick={handleSetup} data-testid="setup-btn">
              Setup
            </button>
            <button onClick={handleDelete} data-testid="delete-btn">
              Delete
            </button>
          </div>
        );
      }

      render(
        <ThemeProvider>
          <DeleteCurrentTheme />
        </ThemeProvider>
      );

      // Setup custom theme and set it as current
      act(() => {
        screen.getByTestId('setup-btn').click();
      });

      await waitFor(() => {
        expect(screen.getByTestId('current-theme-id')).toHaveTextContent('temporary');
      });

      // Delete the current theme
      act(() => {
        screen.getByTestId('delete-btn').click();
      });

      await waitFor(() => {
        // Should revert to default (light)
        expect(screen.getByTestId('current-theme-id')).toHaveTextContent('light');
      });
    });

    it('should not delete built-in themes', async () => {
      function TryDeleteBuiltIn() {
        const { deleteTheme, availableThemes } = useTheme();

        return (
          <div>
            <div data-testid="theme-count">{availableThemes.length}</div>
            <button onClick={() => deleteTheme('light')} data-testid="delete-light-btn">
              Delete Light
            </button>
          </div>
        );
      }

      render(
        <ThemeProvider>
          <TryDeleteBuiltIn />
        </ThemeProvider>
      );

      const initialCount = screen.getByTestId('theme-count').textContent;

      act(() => {
        screen.getByTestId('delete-light-btn').click();
      });

      await waitFor(() => {
        // Count should remain unchanged
        expect(screen.getByTestId('theme-count')).toHaveTextContent(initialCount!);
      });
    });
  });

  describe('preview functionality', () => {
    it('should provide preview function via hook', () => {
      render(
        <ThemeProvider>
          <PreviewConsumer />
        </ThemeProvider>
      );

      expect(screen.getByTestId('preview-btn')).toBeInTheDocument();
      expect(screen.getByTestId('restore-btn')).toBeInTheDocument();
    });

    it('should apply theme visually without changing current theme', async () => {
      function PreviewWithoutChange() {
        const { previewTheme, currentTheme } = useTheme();

        const handlePreview = () => {
          previewTheme('dark');
        };

        return (
          <div>
            <div data-testid="current-theme-id">{currentTheme.id}</div>
            <button onClick={handlePreview} data-testid="preview-btn">
              Preview Dark
            </button>
          </div>
        );
      }

      render(
        <ThemeProvider>
          <PreviewWithoutChange />
        </ThemeProvider>
      );

      act(() => {
        screen.getByTestId('preview-btn').click();
      });

      await waitFor(() => {
        // Current theme should still be light (not changed)
        expect(screen.getByTestId('current-theme-id')).toHaveTextContent('light');
      });
    });
  });

  describe('multiple provider instances', () => {
    it('should work with nested providers (each independent)', () => {
      render(
        <ThemeProvider>
          <div data-testid="outer">
            <ThemeConsumer />
            <ThemeProvider>
              <div data-testid="inner">
                <ThemeConsumer />
              </div>
            </ThemeProvider>
          </div>
        </ThemeProvider>
      );

      // Both should render with default theme
      const themeIds = screen.getAllByTestId('current-theme-id');
      expect(themeIds).toHaveLength(2);
      themeIds.forEach((el) => {
        expect(el).toHaveTextContent('light');
      });
    });
  });

  describe('state synchronization', () => {
    it('should keep availableThemes synchronized across updates', async () => {
      function SynchronizationTest() {
        const { importTheme, deleteTheme, availableThemes } = useTheme();

        const handleImport = () => {
          const customTheme: Theme = {
            id: 'sync-test',
            name: 'Sync Test',
            version: '1.0.0',
            colors: { ...lightTheme.colors },
          };

          importTheme(JSON.stringify(customTheme));
        };

        const handleDelete = () => {
          deleteTheme('sync-test');
        };

        return (
          <div>
            <div data-testid="count-1">{availableThemes.length}</div>
            <div data-testid="count-2">{availableThemes.length}</div>
            <button onClick={handleImport} data-testid="import-btn">
              Import
            </button>
            <button onClick={handleDelete} data-testid="delete-btn">
              Delete
            </button>
          </div>
        );
      }

      render(
        <ThemeProvider>
          <SynchronizationTest />
        </ThemeProvider>
      );

      // Both counts should be identical
      expect(screen.getByTestId('count-1').textContent).toBe(
        screen.getByTestId('count-2').textContent
      );

      act(() => {
        screen.getByTestId('import-btn').click();
      });

      await waitFor(() => {
        // Both should update together
        expect(screen.getByTestId('count-1').textContent).toBe('4');
        expect(screen.getByTestId('count-2').textContent).toBe('4');
      });
    });
  });
});
