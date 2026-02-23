import { createContext, useContext, useState, useEffect, ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';
import i18next from 'i18next';

interface SettingsContextValue {
  showKeyboardShortcuts: boolean;
  setShowKeyboardShortcuts: (show: boolean) => void;
  hideWindowControls: boolean;
  setHideWindowControls: (hide: boolean) => void;
}

const SettingsContext = createContext<SettingsContextValue | undefined>(undefined);

export function SettingsProvider({ children }: { children: ReactNode }) {
  const [showKeyboardShortcuts, setShowKeyboardShortcutsState] = useState(true);
  const [hideWindowControls, setHideWindowControlsState] = useState(false);

  // Load settings on mount
  useEffect(() => {
    loadSettings();
  }, []);

  const loadSettings = async () => {
    // Defer to next tick to avoid blocking initial render
    await new Promise(resolve => setTimeout(resolve, 0));

    try {
      const [shortcutsSetting, windowControlsSetting, languageSetting] = await Promise.all([
        invoke<string | null>('get_user_setting', {
          key: 'ui.show_keyboard_shortcuts',
        }),
        invoke<string | null>('get_user_setting', {
          key: 'ui.hide_window_controls',
        }),
        invoke<string | null>('get_user_setting', {
          key: 'ui.language',
        }),
      ]);

      if (shortcutsSetting !== null) {
        setShowKeyboardShortcutsState(JSON.parse(shortcutsSetting));
      }
      if (windowControlsSetting !== null) {
        setHideWindowControlsState(JSON.parse(windowControlsSetting));
      }
      if (languageSetting !== null) {
        const lang = JSON.parse(languageSetting) as string;
        if (lang && lang !== i18next.language) {
          i18next.changeLanguage(lang);
        }
      }
    } catch (error) {
      console.error('Failed to load settings:', error);
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

  const setHideWindowControls = async (hide: boolean) => {
    try {
      await invoke('set_user_setting', {
        key: 'ui.hide_window_controls',
        value: JSON.stringify(hide),
      });
      setHideWindowControlsState(hide);
    } catch (error) {
      console.error('Failed to save window controls setting:', error);
    }
  };

  return (
    <SettingsContext.Provider
      value={{
        showKeyboardShortcuts,
        setShowKeyboardShortcuts,
        hideWindowControls,
        setHideWindowControls,
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
