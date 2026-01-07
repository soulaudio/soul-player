import { createContext, useContext, useState, useEffect, ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface SettingsContextValue {
  showKeyboardShortcuts: boolean;
  setShowKeyboardShortcuts: (show: boolean) => void;
}

const SettingsContext = createContext<SettingsContextValue | undefined>(undefined);

export function SettingsProvider({ children }: { children: ReactNode }) {
  const [showKeyboardShortcuts, setShowKeyboardShortcutsState] = useState(true);

  // Load settings on mount
  useEffect(() => {
    loadSettings();
  }, []);

  const loadSettings = async () => {
    try {
      const setting = await invoke<string | null>('get_user_setting', {
        key: 'ui.show_keyboard_shortcuts',
      });
      if (setting !== null) {
        setShowKeyboardShortcutsState(JSON.parse(setting));
      }
    } catch (error) {
      console.error('Failed to load keyboard shortcuts setting:', error);
    }
  };

  const setShowKeyboardShortcuts = async (show: boolean) => {
    try {
      await invoke('set_user_setting', {
        key: 'ui.show_keyboard_shortcuts',
        value: JSON.stringify(show),
      });
      setShowKeyboardShortcutsState(show);
    } catch (error) {
      console.error('Failed to save keyboard shortcuts setting:', error);
    }
  };

  return (
    <SettingsContext.Provider
      value={{
        showKeyboardShortcuts,
        setShowKeyboardShortcuts,
      }}
    >
      {children}
    </SettingsContext.Provider>
  );
}

export function useSettings() {
  const context = useContext(SettingsContext);
  if (!context) {
    throw new Error('useSettings must be used within a SettingsProvider');
  }
  return context;
}
