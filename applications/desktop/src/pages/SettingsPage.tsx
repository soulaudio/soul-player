import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { ThemePicker } from '@soul-player/shared/theme';
import { useSettings } from '../contexts/SettingsContext';
import { Kbd } from '@soul-player/shared';
import { AudioSettingsPage } from '@soul-player/shared/settings';

type SettingsTab = 'general' | 'audio' | 'shortcuts' | 'about';

export function SettingsPage() {
  const { t, i18n } = useTranslation();
  const { showKeyboardShortcuts, setShowKeyboardShortcuts } = useSettings();
  const [autoUpdate, setAutoUpdate] = useState(true);
  const [silentUpdate, setSilentUpdate] = useState(false);
  const [checking, setChecking] = useState(false);
  const [activeTab, setActiveTab] = useState<SettingsTab>('general');

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

  const handleLanguageChange = async (locale: string) => {
    try {
      await invoke('set_user_setting', {
        key: 'ui.locale',
        value: JSON.stringify(locale)
      });
      i18n.changeLanguage(locale);
    } catch (error) {
      console.error('Failed to save language:', error);
    }
  };

  const handleAutoUpdateChange = async (enabled: boolean) => {
    try {
      await invoke('set_user_setting', {
        key: 'app.auto_update_enabled',
        value: JSON.stringify(enabled)
      });
      setAutoUpdate(enabled);
    } catch (error) {
      console.error('Failed to save auto-update setting:', error);
    }
  };

  const handleSilentUpdateChange = async (enabled: boolean) => {
    try {
      await invoke('set_user_setting', {
        key: 'app.auto_update_silent',
        value: JSON.stringify(enabled)
      });
      setSilentUpdate(enabled);
    } catch (error) {
      console.error('Failed to save silent update setting:', error);
    }
  };

  const checkForUpdates = async () => {
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
  };

  const tabs: { id: SettingsTab; label: string }[] = [
    { id: 'general', label: t('settings.general') },
    { id: 'audio', label: t('settings.audio') },
    { id: 'shortcuts', label: t('settings.shortcuts') },
    { id: 'about', label: t('settings.about') },
  ];

  return (
    <div className="h-full flex flex-col">
      {/* Tabs Navigation */}
      <div className="border-b border-border px-6">
        <div className="flex space-x-8">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`
                py-4 px-2 border-b-2 transition-colors font-medium text-sm
                ${
                  activeTab === tab.id
                    ? 'border-primary text-foreground'
                    : 'border-transparent text-muted-foreground hover:text-foreground'
                }
              `}
            >
              {tab.label}
            </button>
          ))}
        </div>
      </div>

      {/* Tab Content */}
      <div className="flex-1 overflow-y-auto p-8">
        {activeTab === 'general' && <GeneralSettings />}
        {activeTab === 'audio' && <AudioSettingsPage />}
        {activeTab === 'shortcuts' && <ShortcutsSettings />}
        {activeTab === 'about' && <AboutSettings />}
      </div>
    </div>
  );

  // General Settings Tab Content
  function GeneralSettings() {
    return (
      <div className="max-w-4xl space-y-8">
        {/* Appearance Section */}
        <section>
          <h2 className="text-2xl font-semibold mb-4">{t('settings.appearance')}</h2>

        <div className="mb-6">
          <label className="block text-sm font-medium mb-2">{t('settings.theme')}</label>
          <ThemePicker
            showImportExport={true}
            showAccessibilityInfo={true}
          />
        </div>

        <div className="mb-6">
          <label className="block text-sm font-medium mb-2">{t('settings.language')}</label>
          <select
            value={i18n.language}
            onChange={(e) => handleLanguageChange(e.target.value)}
            className="w-full max-w-xs px-3 py-2 border rounded-lg bg-background"
          >
            <option value="en-US">English (US)</option>
            <option value="de">Deutsch</option>
            <option value="ja">日本語</option>
          </select>
        </div>

        <div>
          <label className="flex items-start space-x-3 cursor-pointer">
            <input
              type="checkbox"
              checked={showKeyboardShortcuts}
              onChange={(e) => setShowKeyboardShortcuts(e.target.checked)}
              className="w-4 h-4 mt-0.5"
            />
            <div>
              <span className="text-sm font-medium block">Show keyboard shortcuts</span>
              <p className="text-xs text-muted-foreground mt-1">
                Display keyboard shortcuts in tooltips and UI elements. For example: <Kbd keys={['mod', 'k']} size="sm" />
              </p>
            </div>
          </label>
        </div>
      </section>

        {/* Updates Section */}
        <section>
          <h2 className="text-2xl font-semibold mb-4">{t('settings.updates')}</h2>
          <div className="space-y-4">
            <label className="flex items-center space-x-3">
              <input
                type="checkbox"
                checked={autoUpdate}
                onChange={(e) => handleAutoUpdateChange(e.target.checked)}
                className="w-4 h-4"
              />
              <span className="text-sm">{t('settings.autoUpdate')}</span>
            </label>

            <label className="flex items-center space-x-3">
              <input
                type="checkbox"
                checked={silentUpdate}
                onChange={(e) => handleSilentUpdateChange(e.target.checked)}
                disabled={!autoUpdate}
                className="w-4 h-4"
              />
              <span className="text-sm">{t('settings.silentUpdate')}</span>
            </label>

            <button
              onClick={checkForUpdates}
              disabled={checking}
              className="px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 disabled:opacity-50"
            >
              {checking ? 'Checking...' : t('settings.checkNow')}
            </button>
          </div>
        </section>
      </div>
    );
  }

  // Shortcuts Settings Tab Content
  function ShortcutsSettings() {
    return (
      <div className="max-w-4xl">
        <h2 className="text-3xl font-bold mb-6">{t('settings.shortcuts')}</h2>
        <p className="text-sm text-muted-foreground mb-6">
          Configure global keyboard shortcuts for playback control.
        </p>
        <button
          className="px-4 py-2 border rounded-lg hover:bg-muted"
          onClick={() => alert('Shortcuts editor coming soon!')}
        >
          Configure Shortcuts
        </button>
      </div>
    );
  }

  // About Settings Tab Content
  function AboutSettings() {
    return (
      <div className="max-w-4xl">
        <h2 className="text-3xl font-bold mb-6">{t('settings.about')}</h2>
        <div className="bg-muted/40 rounded-lg p-6 space-y-3">
          <p className="text-lg">
            <span className="font-semibold">Soul Player</span> - Local-first music player
          </p>
          <p className="text-sm text-muted-foreground">
            {t('settings.version')} 0.1.0
          </p>
          <p className="text-sm text-muted-foreground">
            High-quality audio playback with professional audio processing pipeline
          </p>
        </div>
      </div>
    );
  }
}
