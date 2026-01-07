/**
 * ThemePicker - Complete theme management UI component
 * Includes theme selection, import/export, and management features
 */

import { useState } from 'react';
import { useTheme } from '../useTheme';
import { ThemePreview } from './ThemePreview';
import type { Theme } from '../types';

interface ThemePickerProps {
  /** Show import/export buttons */
  showImportExport?: boolean;
  /** Show accessibility information */
  showAccessibilityInfo?: boolean;
  /** Custom className */
  className?: string;
}

/**
 * ThemePicker component - Full-featured theme management UI
 *
 * Features:
 * - Visual theme selection with preview cards
 * - Import custom themes from JSON
 * - Export themes to JSON files
 * - Delete custom themes
 * - Accessibility information display
 * - Live theme preview
 */
export function ThemePicker({
  showImportExport = true,
  showAccessibilityInfo = true,
  className = '',
}: ThemePickerProps) {
  const {
    currentTheme,
    availableThemes,
    setTheme,
    importTheme,
    exportTheme,
    deleteTheme,
  } = useTheme();

  const [importError, setImportError] = useState<string | null>(null);
  const [importWarnings, setImportWarnings] = useState<string[]>([]);
  const [importSuccess, setImportSuccess] = useState<string | null>(null);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState<string | null>(null);

  const builtInThemes = availableThemes.filter((t) => t.isBuiltIn);
  const customThemes = availableThemes.filter((t) => !t.isBuiltIn);

  /**
   * Handle theme import from file
   */
  const handleImport = async () => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = 'application/json,.json';
    input.multiple = false;

    input.onchange = async (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (!file) return;

      try {
        const text = await file.text();
        const result = importTheme(text);

        if (result.valid && result.theme) {
          setImportSuccess(`Theme "${result.theme.name}" imported successfully!`);
          setImportError(null);
          setImportWarnings(result.warnings);

          // Clear success message after 5 seconds
          setTimeout(() => setImportSuccess(null), 5000);
        } else {
          setImportError(result.errors.join('\n'));
          setImportWarnings([]);
          setImportSuccess(null);
        }
      } catch (error) {
        setImportError(`Failed to read file: ${error instanceof Error ? error.message : 'Unknown error'}`);
        setImportWarnings([]);
        setImportSuccess(null);
      }
    };

    input.click();
  };

  /**
   * Handle theme export to file
   */
  const handleExport = (theme: Theme) => {
    const json = exportTheme(theme.id);
    if (!json) {
      alert('Failed to export theme');
      return;
    }

    const blob = new Blob([json], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${theme.id}-theme.json`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  };

  /**
   * Handle theme deletion
   */
  const handleDelete = (themeId: string) => {
    const success = deleteTheme(themeId);
    if (success) {
      setShowDeleteConfirm(null);
    } else {
      alert('Failed to delete theme. Built-in themes cannot be deleted.');
    }
  };

  /**
   * Handle theme selection
   */
  const handleSelectTheme = (themeId: string) => {
    setTheme(themeId);
    // Clear any messages
    setImportSuccess(null);
    setImportError(null);
    setImportWarnings([]);
  };

  return (
    <div className={`theme-picker space-y-6 ${className}`}>
      {/* Import/Export Section */}
      {showImportExport && (
        <div className="border-b border-border pb-6">
          <h3 className="text-lg font-semibold mb-3">Theme Management</h3>
          <div className="flex flex-wrap gap-3">
            <button
              onClick={handleImport}
              className="px-4 py-2 bg-primary text-primary-foreground rounded-md hover:opacity-90 transition-opacity"
            >
              Import Theme
            </button>

            <button
              onClick={() => handleExport(currentTheme)}
              className="px-4 py-2 bg-secondary text-secondary-foreground rounded-md hover:opacity-90 transition-opacity"
            >
              Export Current Theme
            </button>
          </div>

          {/* Import feedback messages */}
          {importSuccess && (
            <div className="mt-3 p-3 bg-primary/10 border border-primary rounded-md">
              <p className="text-sm font-medium text-primary">{importSuccess}</p>
            </div>
          )}

          {importError && (
            <div className="mt-3 p-3 bg-destructive/10 border border-destructive rounded-md">
              <p className="text-sm font-semibold text-destructive mb-1">Import Failed:</p>
              <pre className="text-xs text-destructive whitespace-pre-wrap">{importError}</pre>
            </div>
          )}

          {importWarnings.length > 0 && (
            <div className="mt-3 p-3 bg-yellow-500/10 border border-yellow-500 rounded-md">
              <p className="text-sm font-semibold text-yellow-700 dark:text-yellow-300 mb-1">
                Warnings:
              </p>
              <ul className="text-xs text-yellow-700 dark:text-yellow-300 list-disc list-inside">
                {importWarnings.map((warning, i) => (
                  <li key={i}>{warning}</li>
                ))}
              </ul>
            </div>
          )}
        </div>
      )}

      {/* Built-in Themes */}
      <div>
        <h3 className="text-lg font-semibold mb-3">Built-in Themes</h3>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {builtInThemes.map((theme) => (
            <ThemePreview
              key={theme.id}
              theme={theme}
              isActive={currentTheme.id === theme.id}
              onClick={() => handleSelectTheme(theme.id)}
            />
          ))}
        </div>
      </div>

      {/* Custom Themes */}
      {customThemes.length > 0 && (
        <div>
          <h3 className="text-lg font-semibold mb-3">Custom Themes</h3>
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
            {customThemes.map((theme) => (
              <div key={theme.id} className="relative">
                <ThemePreview
                  theme={theme}
                  isActive={currentTheme.id === theme.id}
                  onClick={() => handleSelectTheme(theme.id)}
                />

                {/* Delete confirmation dialog */}
                {showDeleteConfirm === theme.id ? (
                  <div className="absolute inset-0 bg-background/95 rounded-lg flex flex-col items-center justify-center p-4 border-2 border-destructive">
                    <p className="text-sm font-medium mb-3 text-center">
                      Delete "{theme.name}"?
                    </p>
                    <div className="flex gap-2">
                      <button
                        onClick={() => handleDelete(theme.id)}
                        className="px-3 py-1 bg-destructive text-destructive-foreground rounded text-sm"
                      >
                        Delete
                      </button>
                      <button
                        onClick={() => setShowDeleteConfirm(null)}
                        className="px-3 py-1 bg-secondary text-secondary-foreground rounded text-sm"
                      >
                        Cancel
                      </button>
                    </div>
                  </div>
                ) : (
                  <div className="absolute top-2 right-2 flex gap-1">
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        handleExport(theme);
                      }}
                      className="p-1.5 bg-secondary/80 hover:bg-secondary text-secondary-foreground rounded text-xs"
                      title="Export theme"
                    >
                      💾
                    </button>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        setShowDeleteConfirm(theme.id);
                      }}
                      className="p-1.5 bg-destructive/80 hover:bg-destructive text-destructive-foreground rounded text-xs"
                      title="Delete theme"
                    >
                      🗑️
                    </button>
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Accessibility Information */}
      {showAccessibilityInfo && (
        <div className="border-t border-border pt-6">
          <h3 className="text-lg font-semibold mb-3">Current Theme Info</h3>
          <div className="bg-muted/40 rounded-lg p-4 space-y-2">
            <div className="flex justify-between items-center">
              <span className="text-sm font-medium">Name:</span>
              <span className="text-sm text-muted-foreground">{currentTheme.name}</span>
            </div>
            <div className="flex justify-between items-center">
              <span className="text-sm font-medium">Version:</span>
              <span className="text-sm text-muted-foreground">{currentTheme.version}</span>
            </div>
            {currentTheme.author && (
              <div className="flex justify-between items-center">
                <span className="text-sm font-medium">Author:</span>
                <span className="text-sm text-muted-foreground">{currentTheme.author}</span>
              </div>
            )}
            {currentTheme.description && (
              <div className="pt-2 border-t border-border">
                <p className="text-sm text-muted-foreground">{currentTheme.description}</p>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
