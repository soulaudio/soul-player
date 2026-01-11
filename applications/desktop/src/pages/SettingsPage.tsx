/**
 * Desktop SettingsPage - wraps shared SettingsPage with Tauri handlers
 */

import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { SettingsPage as SharedSettingsPage, type SettingsHandlers } from '@soul-player/shared';
import { useSettings } from '../contexts/SettingsContext';
import { ShortcutsSettings } from '../components/ShortcutsSettings';

export function SettingsPage() {
  const { i18n } = useTranslation();
  const { showKeyboardShortcuts, setShowKeyboardShortcuts } = useSettings();
  const [autoUpdate, setAutoUpdate] = useState(true);
  const [silentUpdate, setSilentUpdate] = useState(false);
  const [checking, setChecking] = useState(false);

  // Load settings from backend
  useEffect(() => {
    loadSettings();
  }, []);

  const loadSettings = async () => {
    try {
      // Load auto-update settings
      const autoUpdateSetting = await invoke<string | null>('get_user_setting', {
        key: 'app.auto_update_enabled'
      });
      if (autoUpdateSetting !== null) {
        setAutoUpdate(JSON.parse(autoUpdateSetting));
      }

      const silentUpdateSetting = await invoke<string | null>('get_user_setting', {
        key: 'app.auto_update_silent'
      });
      if (silentUpdateSetting !== null) {
        setSilentUpdate(JSON.parse(silentUpdateSetting));
      }

      // Load and set language
      const localeSetting = await invoke<string | null>('get_user_setting', {
        key: 'ui.locale'
      });
      if (localeSetting !== null) {
        const locale = JSON.parse(localeSetting);
        i18n.changeLanguage(locale);
      }
    } catch (error) {
      console.error('Failed to load settings:', error);
    }
  };

  const handleLanguageChange = useCallback(async (locale: string) => {
    try {
      await invoke('set_user_setting', {
        key: 'ui.locale',
        value: JSON.stringify(locale)
      });
      i18n.changeLanguage(locale);
    } catch (error) {
      console.error('Failed to save language:', error);
    }
  }, [i18n]);

  const handleAutoUpdateChange = useCallback(async (enabled: boolean) => {
    try {
      await invoke('set_user_setting', {
        key: 'app.auto_update_enabled',
        value: JSON.stringify(enabled)
      });
      setAutoUpdate(enabled);
    } catch (error) {
      console.error('Failed to save auto-update setting:', error);
    }
  }, []);

  const handleSilentUpdateChange = useCallback(async (enabled: boolean) => {
    try {
      await invoke('set_user_setting', {
        key: 'app.auto_update_silent',
        value: JSON.stringify(enabled)
      });
      setSilentUpdate(enabled);
    } catch (error) {
      console.error('Failed to save silent update setting:', error);
    }
  }, []);

  const checkForUpdates = useCallback(async () => {
    setChecking(true);
    try {
      const update = await invoke<any>('check_for_updates');
      if (update) {
        alert(`Update available: ${update.version}\n\n${update.body}`);
      } else {
        alert('You are on the latest version!');
      }
    } catch (error) {
      console.error('Failed to check for updates:', error);
      alert('Failed to check for updates');
    } finally {
      setChecking(false);
    }
  }, []);

  const handlers: SettingsHandlers = {
    loadSettings,
    handleLanguageChange,
    handleAutoUpdateChange,
    handleSilentUpdateChange,
    checkForUpdates,
    showKeyboardShortcuts,
    setShowKeyboardShortcuts,
    autoUpdate,
    silentUpdate,
    checking,
  };

  return (
    <SharedSettingsPage
      handlers={handlers}
      ShortcutsSettingsComponent={ShortcutsSettings}
    />
  );
}
