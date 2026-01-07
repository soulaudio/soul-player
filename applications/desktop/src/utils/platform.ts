/**
 * Platform detection utilities
 */

export type Platform = 'macos' | 'windows' | 'linux';

/**
 * Detects the current platform
 */
export function getPlatform(): Platform {
  const userAgent = window.navigator.userAgent.toLowerCase();
  const platform = window.navigator.platform.toLowerCase();

  if (platform.includes('mac') || userAgent.includes('mac')) {
    return 'macos';
  }
  if (platform.includes('win') || userAgent.includes('win')) {
    return 'windows';
  }
  return 'linux';
}

/**
 * Checks if the current platform is macOS
 */
export function isMac(): boolean {
  return getPlatform() === 'macos';
}

/**
 * Gets the primary modifier key for the current platform
 * - macOS: '⌘' (Command)
 * - Windows/Linux: 'Ctrl'
 */
export function getModifierKey(): string {
  return isMac() ? '⌘' : 'Ctrl';
}

/**
 * Gets the modifier key name for aria-labels and tooltips
 */
export function getModifierKeyName(): string {
  return isMac() ? 'Cmd' : 'Ctrl';
}

/**
 * Formats a keyboard shortcut for display
 * @param keys - Array of keys (e.g., ['mod', 'k'] or ['shift', 'n'])
 */
export function formatShortcut(keys: string[]): string {
  return keys
    .map(key => {
      if (key.toLowerCase() === 'mod') {
        return getModifierKey();
      }
      if (key.toLowerCase() === 'shift') {
        return '⇧';
      }
      if (key.toLowerCase() === 'alt') {
        return isMac() ? '⌥' : 'Alt';
      }
      if (key.toLowerCase() === 'enter') {
        return '↵';
      }
      if (key.toLowerCase() === 'backspace') {
        return '⌫';
      }
      if (key.toLowerCase() === 'delete') {
        return '⌦';
      }
      if (key.toLowerCase() === 'esc') {
        return '⎋';
      }
      if (key.toLowerCase() === 'tab') {
        return '⇥';
      }
      return key.toUpperCase();
    })
    .join(isMac() ? '' : '+');
}
