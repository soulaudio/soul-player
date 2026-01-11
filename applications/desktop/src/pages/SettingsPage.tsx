import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { ThemePicker } from '@soul-player/shared/theme';
import { useSettings } from '../contexts/SettingsContext';
import { Kbd } from '@soul-player/shared';
import { AudioSettingsPage, LibrarySettingsPage, SourcesSettingsPage } from '@soul-player/shared/settings';
import { ShortcutsSettings } from '../components/ShortcutsSettings';
import {
  Settings,
  Volume2,
  Keyboard,
  Info,
  ChevronDown,
  FolderOpen,
  Cloud,
} from 'lucide-react';

type SettingsTab = 'general' | 'library' | 'sources' | 'audio' | 'shortcuts' | 'about';

interface NavItem {
  id: SettingsTab;
  labelKey: string;
  icon: React.ComponentType<{ className?: string }>;
}

const navigationItems: NavItem[] = [
  { id: 'general', labelKey: 'settings.general', icon: Settings },
  { id: 'library', labelKey: 'settings.library', icon: FolderOpen },
  { id: 'sources', labelKey: 'sources.title', icon: Cloud },
  { id: 'audio', labelKey: 'settings.audio', icon: Volume2 },
  { id: 'shortcuts', labelKey: 'settings.shortcuts', icon: Keyboard },
  { id: 'about', labelKey: 'settings.about', icon: Info },
];

export function SettingsPage() {
  const { t, i18n } = useTranslation();
  const { showKeyboardShortcuts, setShowKeyboardShortcuts } = useSettings();
  const [autoUpdate, setAutoUpdate] = useState(true);
  const [silentUpdate, setSilentUpdate] = useState(false);
  const [checking, setChecking] = useState(false);
  const [activeTab, setActiveTab] = useState<SettingsTab>('general');
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);

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

  const activeItem = navigationItems.find((item) => item.id === activeTab);

  return (
    <div className="h-full flex flex-col md:flex-row">
      {/* Desktop Sidebar - hidden on mobile */}
      <aside className="hidden md:flex w-56 flex-shrink-0 flex-col">
        <nav className="p-4 h-full flex flex-col">
          <ul className="space-y-1">
            {navigationItems.map((item) => {
              const Icon = item.icon;
              const isActive = activeTab === item.id;

              return (
                <li key={item.id}>
                  <button
                    onClick={() => setActiveTab(item.id)}
                    className={`
                      w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm
                      transition-colors duration-150 text-left
                      ${
                        isActive
                          ? 'bg-primary text-primary-foreground font-medium'
                          : 'text-muted-foreground hover:bg-muted hover:text-foreground'
                      }
                    `}
                  >
                    <Icon className="w-4 h-4 flex-shrink-0" />
                    <span>{t(item.labelKey)}</span>
                  </button>
                </li>
              );
            })}
          </ul>
        </nav>
      </aside>

      {/* Mobile Header - visible only on mobile */}
      <div className="md:hidden">
        <div className="relative">
          <button
            onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
            className="w-full flex items-center justify-between px-4 py-3 text-left"
          >
            <div className="flex items-center gap-3">
              {activeItem && <activeItem.icon className="w-5 h-5" />}
              <span className="font-medium">{activeItem ? t(activeItem.labelKey) : ''}</span>
            </div>
            <ChevronDown
              className={`w-5 h-5 text-muted-foreground transition-transform ${
                mobileMenuOpen ? 'rotate-180' : ''
              }`}
            />
          </button>

          {/* Mobile Dropdown Menu */}
          {mobileMenuOpen && (
            <div className="absolute top-full left-0 right-0 bg-background shadow-lg z-50">
              {navigationItems.map((item) => {
                const Icon = item.icon;
                const isActive = activeTab === item.id;

                return (
                  <button
                    key={item.id}
                    onClick={() => {
                      setActiveTab(item.id);
                      setMobileMenuOpen(false);
                    }}
                    className={`
                      w-full flex items-center gap-3 px-4 py-3 text-left
                      transition-colors duration-150
                      ${
                        isActive
                          ? 'bg-primary/10 text-primary font-medium'
                          : 'text-foreground hover:bg-muted'
                      }
                    `}
                  >
                    <Icon className="w-4 h-4 flex-shrink-0" />
                    <span>{t(item.labelKey)}</span>
                  </button>
                );
              })}
            </div>
          )}
        </div>
      </div>

      {/* Main Content */}
      <main className="flex-1 overflow-y-auto">
        <div className="max-w-4xl mx-auto p-4 md:p-8">
          {activeTab === 'general' && <GeneralSettings />}
          {activeTab === 'library' && <LibrarySettingsPage />}
          {activeTab === 'sources' && <SourcesSettingsPage />}
          {activeTab === 'audio' && <AudioSettingsPage />}
          {activeTab === 'shortcuts' && <ShortcutsSettings />}
          {activeTab === 'about' && <AboutSettings />}
        </div>
      </main>
    </div>
  );

  // General Settings Tab Content
  function GeneralSettings() {
    return (
      <div className="space-y-8">
        {/* Appearance Section */}
        <section>
          <h2 className="text-xl font-semibold mb-4">{t('settings.appearance')}</h2>
          <div className="space-y-6">
            <div>
              <label className="block text-sm font-medium mb-2">{t('settings.theme')}</label>
              <ThemePicker
                showImportExport={true}
                showAccessibilityInfo={true}
              />
            </div>

            <div>
              <label className="block text-sm font-medium mb-2">{t('settings.language')}</label>
              <select
                value={i18n.language}
                onChange={(e) => handleLanguageChange(e.target.value)}
                className="w-full max-w-xs px-3 py-2 rounded-lg bg-muted"
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
                  <span className="text-sm font-medium block">{t('settings.showShortcuts')}</span>
                  <p className="text-xs text-muted-foreground mt-1">
                    {t('settings.showShortcutsDescription')} <Kbd keys={['mod', 'k']} size="sm" />
                  </p>
                </div>
              </label>
            </div>
          </div>
        </section>

        {/* Updates Section */}
        <section>
          <h2 className="text-xl font-semibold mb-4">{t('settings.updates')}</h2>
          <div className="space-y-4">
            <label className="flex items-center space-x-3 cursor-pointer">
              <input
                type="checkbox"
                checked={autoUpdate}
                onChange={(e) => handleAutoUpdateChange(e.target.checked)}
                className="w-4 h-4"
              />
              <span className="text-sm">{t('settings.autoUpdate')}</span>
            </label>

            <label className="flex items-center space-x-3 cursor-pointer">
              <input
                type="checkbox"
                checked={silentUpdate}
                onChange={(e) => handleSilentUpdateChange(e.target.checked)}
                disabled={!autoUpdate}
                className="w-4 h-4 disabled:opacity-50"
              />
              <span className={`text-sm ${!autoUpdate ? 'text-muted-foreground' : ''}`}>
                {t('settings.silentUpdate')}
              </span>
            </label>

            <button
              onClick={checkForUpdates}
              disabled={checking}
              className="px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 disabled:opacity-50"
            >
              {checking ? t('settings.checking') : t('settings.checkNow')}
            </button>
          </div>
        </section>
      </div>
    );
  }


  // About Settings Tab Content
  function AboutSettings() {
    return (
      <div className="space-y-8">
        <section>
          <div className="flex items-center gap-4 mb-4">
            <div className="w-14 h-14 bg-primary/10 rounded-xl flex items-center justify-center">
              <Volume2 className="w-7 h-7 text-primary" />
            </div>
            <div>
              <h3 className="text-lg font-semibold">Soul Player</h3>
              <p className="text-sm text-muted-foreground">
                {t('settings.version')} 0.1.0
              </p>
            </div>
          </div>
          <p className="text-sm text-muted-foreground">
            {t('settings.aboutDescription')}
          </p>
        </section>

        <section>
          <h2 className="text-sm font-medium mb-3">{t('settings.links')}</h2>
          <div className="space-y-2">
            <a
              href="https://github.com/soulaudio/soul-player"
              target="_blank"
              rel="noopener noreferrer"
              className="block text-sm text-primary hover:underline"
            >
              {t('settings.github')}
            </a>
            <a
              href="https://soulplayer.app"
              target="_blank"
              rel="noopener noreferrer"
              className="block text-sm text-primary hover:underline"
            >
              {t('settings.website')}
            </a>
          </div>
        </section>
      </div>
    );
  }
}
