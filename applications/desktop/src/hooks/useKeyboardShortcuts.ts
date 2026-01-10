/**
 * App-level keyboard shortcuts hook
 *
 * This hook provides keyboard shortcuts that:
 * 1. Only work when the app window is focused (not OS-level global)
 * 2. Respect input fields (don't fire when typing in textarea/input)
 * 3. Are customizable via the settings page
 *
 * The shortcuts are stored in the database and loaded on mount.
 */

import { useEffect, useCallback, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { usePlayerStore } from '@soul-player/shared';

interface GlobalShortcut {
  action: string;
  accelerator: string;
  enabled: boolean;
  is_default: boolean;
}

type ShortcutAction =
  | 'play_pause'
  | 'next'
  | 'previous'
  | 'volume_up'
  | 'volume_down'
  | 'mute'
  | 'toggle_shuffle'
  | 'toggle_repeat';

/**
 * Check if the currently focused element is an editable field
 * where we should NOT trigger shortcuts
 */
function isEditableElement(element: Element | null): boolean {
  if (!element) return false;

  const tagName = element.tagName.toLowerCase();

  // Check for input elements (text, search, etc.)
  if (tagName === 'input') {
    const inputType = (element as HTMLInputElement).type.toLowerCase();
    // These input types should block shortcuts
    const textInputTypes = [
      'text',
      'search',
      'email',
      'password',
      'url',
      'tel',
      'number',
    ];
    return textInputTypes.includes(inputType);
  }

  // Textarea always blocks shortcuts
  if (tagName === 'textarea') return true;

  // Check for contenteditable elements
  if (element.getAttribute('contenteditable') === 'true') return true;

  // Check for elements with role="textbox"
  if (element.getAttribute('role') === 'textbox') return true;

  return false;
}

/**
 * Parse an accelerator string (e.g., "CommandOrControl+Space") and check if it matches the keyboard event
 */
function matchesAccelerator(event: KeyboardEvent, accelerator: string): boolean {
  const parts = accelerator.split('+').map((p) => p.trim().toLowerCase());
  const key = parts[parts.length - 1];
  const modifiers = parts.slice(0, -1);

  // Check modifiers
  const needsCtrl = modifiers.some((m) =>
    ['ctrl', 'control', 'commandorcontrol', 'cmdorctrl'].includes(m)
  );
  const needsMeta = modifiers.some((m) =>
    ['cmd', 'command', 'commandorcontrol', 'cmdorctrl', 'meta'].includes(m)
  );
  const needsAlt = modifiers.some((m) => ['alt', 'option'].includes(m));
  const needsShift = modifiers.includes('shift');

  // CommandOrControl means Ctrl on Windows/Linux, Cmd on macOS
  const isMac = navigator.platform.toLowerCase().includes('mac');
  const ctrlOrCmdPressed = isMac ? event.metaKey : event.ctrlKey;

  if (needsCtrl || needsMeta) {
    if (!ctrlOrCmdPressed) return false;
  }
  if (needsAlt && !event.altKey) return false;
  if (needsShift && !event.shiftKey) return false;

  // Check the actual key
  let eventKey = event.key.toLowerCase();

  // Normalize arrow keys
  if (eventKey === 'arrowleft') eventKey = 'left';
  if (eventKey === 'arrowright') eventKey = 'right';
  if (eventKey === 'arrowup') eventKey = 'up';
  if (eventKey === 'arrowdown') eventKey = 'down';
  if (eventKey === ' ') eventKey = 'space';

  return eventKey === key.toLowerCase();
}

export function useKeyboardShortcuts() {
  const [shortcuts, setShortcuts] = useState<GlobalShortcut[]>([]);

  // Load shortcuts from database
  useEffect(() => {
    const loadShortcuts = async () => {
      try {
        const result = await invoke<GlobalShortcut[]>('get_global_shortcuts');
        setShortcuts(result);
      } catch (error) {
        console.error('[useKeyboardShortcuts] Failed to load shortcuts:', error);
      }
    };

    loadShortcuts();
  }, []);

  // Execute a shortcut action
  const executeAction = useCallback(async (action: ShortcutAction) => {
    const { isPlaying, volume } = usePlayerStore.getState();

    try {
      switch (action) {
        case 'play_pause':
          if (isPlaying) {
            await invoke('pause_playback');
          } else {
            await invoke('resume_playback');
          }
          break;

        case 'next':
          await invoke('next_track');
          break;

        case 'previous':
          await invoke('previous_track');
          break;

        case 'volume_up': {
          const newVolume = Math.min(100, Math.round(volume * 100) + 5);
          await invoke('set_volume', { volume: newVolume });
          break;
        }

        case 'volume_down': {
          const newVolume = Math.max(0, Math.round(volume * 100) - 5);
          await invoke('set_volume', { volume: newVolume });
          break;
        }

        case 'mute': {
          const { previousVolume } = usePlayerStore.getState();
          if (volume > 0) {
            usePlayerStore.setState({ previousVolume: volume });
            await invoke('set_volume', { volume: 0 });
          } else {
            await invoke('set_volume', { volume: Math.round((previousVolume || 0.5) * 100) });
          }
          break;
        }

        case 'toggle_shuffle':
          // TODO: Implement toggle shuffle
          console.log('[useKeyboardShortcuts] Toggle shuffle not implemented');
          break;

        case 'toggle_repeat':
          // TODO: Implement toggle repeat
          console.log('[useKeyboardShortcuts] Toggle repeat not implemented');
          break;
      }
    } catch (error) {
      console.error('[useKeyboardShortcuts] Failed to execute action:', action, error);
    }
  }, []);

  // Handle keyboard events
  useEffect(() => {
    if (shortcuts.length === 0) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      // Don't trigger shortcuts when typing in input fields
      if (isEditableElement(document.activeElement)) {
        return;
      }

      // Find matching shortcut
      for (const shortcut of shortcuts) {
        if (!shortcut.enabled) continue;

        if (matchesAccelerator(event, shortcut.accelerator)) {
          event.preventDefault();
          event.stopPropagation();
          executeAction(shortcut.action as ShortcutAction);
          return;
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [shortcuts, executeAction]);

  // Listen for reload events from ShortcutsSettings
  useEffect(() => {
    const handleReload = async () => {
      try {
        const result = await invoke<GlobalShortcut[]>('get_global_shortcuts');
        setShortcuts(result);
        console.log('[useKeyboardShortcuts] Shortcuts reloaded');
      } catch (error) {
        console.error('[useKeyboardShortcuts] Failed to reload shortcuts:', error);
      }
    };

    window.addEventListener('shortcuts-changed', handleReload);
    return () => window.removeEventListener('shortcuts-changed', handleReload);
  }, []);

  // Return function to reload shortcuts (useful after settings change)
  const reloadShortcuts = useCallback(async () => {
    try {
      const result = await invoke<GlobalShortcut[]>('get_global_shortcuts');
      setShortcuts(result);
    } catch (error) {
      console.error('[useKeyboardShortcuts] Failed to reload shortcuts:', error);
    }
  }, []);

  return { shortcuts, reloadShortcuts };
}

/**
 * Dispatch event to notify useKeyboardShortcuts hook to reload shortcuts
 * Call this after changing shortcuts in settings
 */
export function notifyShortcutsChanged() {
  window.dispatchEvent(new CustomEvent('shortcuts-changed'));
}
