import { invoke } from '@tauri-apps/api/core';
import type { ThemeFileBackend, Theme } from '@soul-player/shared/theme';
import { debug } from '@soul-player/shared';

/**
 * Loads all custom themes stored as JSON files in the app data themes/ directory.
 * Returns parsed Theme objects, skipping any files that fail to parse.
 */
export async function loadThemesFromDisk(): Promise<Theme[]> {
  try {
    const rawList = await invoke<string[]>('theme_list_custom');
    const themes: Theme[] = [];

    for (const json of rawList) {
      try {
        const theme = JSON.parse(json) as Theme;
        if (theme && typeof theme.id === 'string') {
          themes.push(theme);
        }
      } catch (e) {
        debug.warn('[ThemeAdapter] Failed to parse theme JSON from disk:', e);
      }
    }

    return themes;
  } catch (e) {
    debug.error('[ThemeAdapter] Failed to list themes from disk:', e);
    return [];
  }
}

/**
 * Tauri file backend — writes custom themes to $APPDATA/Soul Player/themes/{id}.json
 */
export const tauriThemeFileBackend: ThemeFileBackend = {
  saveTheme(theme: Theme): void {
    invoke('theme_save', {
      themeId: theme.id,
      themeJson: JSON.stringify(theme, null, 2),
    }).catch((e) => debug.error('[ThemeAdapter] Failed to save theme to disk:', e));
  },

  deleteTheme(themeId: string): void {
    invoke('theme_delete', { themeId }).catch((e) =>
      debug.error('[ThemeAdapter] Failed to delete theme from disk:', e)
    );
  },
};
