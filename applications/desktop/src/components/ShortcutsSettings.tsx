/**
 * Keyboard shortcuts configuration UI
 *
 * Allows users to view and customize keyboard shortcuts.
 * These are app-level shortcuts that only work when the app is focused
 * and don't trigger when typing in input fields.
 */

import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { Kbd } from '@soul-player/shared';
import { notifyShortcutsChanged } from '../hooks/useKeyboardShortcuts';

interface GlobalShortcut {
  action: string;
  accelerator: string;
  enabled: boolean;
  is_default: boolean;
}

// Map backend action names to translation keys
const ACTION_LABELS: Record<string, string> = {
  play_pause: 'playPause',
  next: 'next',
  previous: 'previous',
  volume_up: 'volumeUp',
  volume_down: 'volumeDown',
  mute: 'mute',
  toggle_shuffle: 'toggleShuffle',
  toggle_repeat: 'toggleRepeat',
};

// Parse accelerator string to Kbd keys array
function acceleratorToKbdKeys(accelerator: string): string[] {
  const keys: string[] = [];
  const parts = accelerator.split('+');

  for (const part of parts) {
    const normalized = part.trim().toLowerCase();
    if (normalized === 'commandorcontrol' || normalized === 'cmdorctrl') {
      keys.push('mod');
    } else if (normalized === 'command' || normalized === 'cmd') {
      keys.push('mod');
    } else if (normalized === 'control' || normalized === 'ctrl') {
      keys.push('ctrl');
    } else if (normalized === 'shift') {
      keys.push('shift');
    } else if (normalized === 'alt' || normalized === 'option') {
      keys.push('alt');
    } else if (normalized === 'space') {
      keys.push('space');
    } else if (normalized === 'right') {
      keys.push('\u2192'); // Right arrow
    } else if (normalized === 'left') {
      keys.push('\u2190'); // Left arrow
    } else if (normalized === 'up') {
      keys.push('\u2191'); // Up arrow
    } else if (normalized === 'down') {
      keys.push('\u2193'); // Down arrow
    } else {
      keys.push(part.trim().toUpperCase());
    }
  }

  return keys;
}

interface ShortcutRowProps {
  shortcut: GlobalShortcut;
  label: string;
  isEditing: boolean;
  onStartEdit: () => void;
  onSave: (newAccelerator: string) => void;
  onCancel: () => void;
}

function ShortcutRow({
  shortcut,
  label,
  isEditing,
  onStartEdit,
  onSave,
  onCancel,
}: ShortcutRowProps) {
  const { t } = useTranslation();
  const [isRecording, setIsRecording] = useState(false);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (!isRecording) return;

      e.preventDefault();
      e.stopPropagation();

      const parts: string[] = [];

      // Build modifier string
      if (e.ctrlKey || e.metaKey) {
        parts.push('CommandOrControl');
      }
      if (e.shiftKey) {
        parts.push('Shift');
      }
      if (e.altKey) {
        parts.push('Alt');
      }

      // Get the actual key
      let key = e.key;
      if (key === ' ') key = 'Space';
      else if (key === 'ArrowRight') key = 'Right';
      else if (key === 'ArrowLeft') key = 'Left';
      else if (key === 'ArrowUp') key = 'Up';
      else if (key === 'ArrowDown') key = 'Down';
      else if (key.length === 1) key = key.toUpperCase();

      // Don't allow just modifier keys
      if (['Control', 'Shift', 'Alt', 'Meta'].includes(e.key)) {
        return;
      }

      parts.push(key);
      const accelerator = parts.join('+');

      setIsRecording(false);
      onSave(accelerator);
    },
    [isRecording, onSave]
  );

  useEffect(() => {
    if (isRecording) {
      window.addEventListener('keydown', handleKeyDown);
      return () => window.removeEventListener('keydown', handleKeyDown);
    }
  }, [isRecording, handleKeyDown]);

  useEffect(() => {
    if (isEditing) {
      setIsRecording(true);
    } else {
      setIsRecording(false);
    }
  }, [isEditing]);

  const handleClick = () => {
    if (!isEditing) {
      onStartEdit();
    }
  };

  const handleBlur = () => {
    if (isEditing) {
      onCancel();
    }
  };

  return (
    <div className="flex items-center justify-between py-3 border-b border-border last:border-b-0">
      <span className="text-sm font-medium">{label}</span>
      <button
        onClick={handleClick}
        onBlur={handleBlur}
        className={`
          min-w-[120px] px-3 py-1.5 rounded-md text-sm
          transition-colors focus:outline-none
          ${
            isEditing
              ? 'bg-primary text-primary-foreground ring-2 ring-primary ring-offset-2'
              : 'bg-muted hover:bg-muted/80'
          }
        `}
      >
        {isEditing ? (
          <span className="animate-pulse">{t('settings.pressKey', 'Press keys...')}</span>
        ) : (
          <Kbd keys={acceleratorToKbdKeys(shortcut.accelerator)} size="sm" />
        )}
      </button>
    </div>
  );
}

export function ShortcutsSettings() {
  const { t } = useTranslation();
  const [shortcuts, setShortcuts] = useState<GlobalShortcut[]>([]);
  const [loading, setLoading] = useState(true);
  const [editingAction, setEditingAction] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const loadShortcuts = async () => {
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<GlobalShortcut[]>('get_global_shortcuts');
      setShortcuts(result);
    } catch (err) {
      console.error('Failed to load shortcuts:', err);
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadShortcuts();
  }, []);

  const handleSaveShortcut = async (action: string, accelerator: string) => {
    try {
      setError(null);
      await invoke('set_global_shortcut', { action, accelerator });
      setEditingAction(null);
      await loadShortcuts();
      notifyShortcutsChanged(); // Notify the keyboard handler to reload
    } catch (err) {
      console.error('Failed to save shortcut:', err);
      setError(String(err));
    }
  };

  const handleResetDefaults = async () => {
    try {
      setError(null);
      await invoke('reset_global_shortcuts');
      await loadShortcuts();
      notifyShortcutsChanged(); // Notify the keyboard handler to reload
    } catch (err) {
      console.error('Failed to reset shortcuts:', err);
      setError(String(err));
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center py-8">
        <div className="animate-spin h-6 w-6 border-2 border-primary border-t-transparent rounded-full" />
      </div>
    );
  }

  return (
    <div className="max-w-2xl">
      <div className="mb-6">
        <h2 className="text-2xl font-semibold mb-2">{t('settings.shortcuts')}</h2>
        <p className="text-sm text-muted-foreground">
          {t(
            'settings.shortcutsDescription',
            'Configure keyboard shortcuts for playback control. Shortcuts only work when the app is focused and are disabled when typing in text fields.'
          )}
        </p>
      </div>

      {error && (
        <div className="mb-4 p-3 bg-destructive/10 text-destructive rounded-lg text-sm">
          {error}
        </div>
      )}

      <div className="bg-card rounded-lg border border-border p-4 mb-6">
        {shortcuts.map((shortcut) => (
          <ShortcutRow
            key={shortcut.action}
            shortcut={shortcut}
            label={t(`shortcuts.${ACTION_LABELS[shortcut.action] || shortcut.action}`)}
            isEditing={editingAction === shortcut.action}
            onStartEdit={() => setEditingAction(shortcut.action)}
            onSave={(accelerator) => handleSaveShortcut(shortcut.action, accelerator)}
            onCancel={() => setEditingAction(null)}
          />
        ))}

        {shortcuts.length === 0 && (
          <p className="text-sm text-muted-foreground py-4 text-center">
            {t('settings.noShortcuts', 'No shortcuts configured')}
          </p>
        )}
      </div>

      <button
        onClick={handleResetDefaults}
        className="px-4 py-2 border border-border rounded-lg hover:bg-muted transition-colors text-sm"
      >
        {t('settings.resetShortcuts')}
      </button>
    </div>
  );
}
