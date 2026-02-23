import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { ThemePicker, useBackend, debug } from '@soul-player/shared';
import { useSettings } from '../../contexts/SettingsContext';

export function AppearanceSettingsPage() {
  const { t } = useTranslation();
  const backend = useBackend();
  const { hideWindowControls, setHideWindowControls } = useSettings();

  const [homeEnabled, setHomeEnabledState] = useState(true);
  const [hideLibrarySearch, setHideLibrarySearchState] = useState(false);

  useEffect(() => {
    Promise.all([
      backend.getUserSetting('home.enabled'),
      backend.getUserSetting('ui.hide_library_search'),
    ])
      .then(([home, hideSearch]) => {
        setHomeEnabledState(home ?? true);
        setHideLibrarySearchState(hideSearch ?? false);
      })
      .catch(err => debug.error('Failed to load appearance settings:', err));
  }, [backend]);

  const handleHomeToggle = useCallback((enabled: boolean) => {
    setHomeEnabledState(enabled);
    backend.setUserSetting('home.enabled', enabled)
      .then(() => {
        window.dispatchEvent(new CustomEvent('home-enabled-changed', { detail: { enabled } }));
      })
      .catch(err => debug.error('Failed to save home setting:', err));
  }, [backend]);

  const handleHideLibrarySearch = useCallback((hide: boolean) => {
    setHideLibrarySearchState(hide);
    backend.setUserSetting('ui.hide_library_search', hide)
      .then(() => {
        window.dispatchEvent(new CustomEvent('library-search-hidden-changed', { detail: { hide } }));
      })
      .catch(err => debug.error('Failed to save library search setting:', err));
  }, [backend]);

  return (
    <div className="max-w-2xl space-y-8">
      <div>
        <h1 className="text-2xl font-bold mb-1">{t('settings.appearance')}</h1>
      </div>

      {/* UI Options */}
      <section className="space-y-4">
        <label className="flex items-start gap-3 cursor-pointer">
          <input
            type="checkbox"
            checked={homeEnabled}
            onChange={e => handleHomeToggle(e.target.checked)}
            className="w-4 h-4 mt-0.5"
          />
          <div>
            <span className="text-sm font-medium block">{t('settings.homePageEnabled')}</span>
            <p className="text-xs text-muted-foreground mt-1">{t('settings.homePageEnabledDescription')}</p>
          </div>
        </label>

        <label className="flex items-start gap-3 cursor-pointer">
          <input
            type="checkbox"
            checked={hideLibrarySearch}
            onChange={e => handleHideLibrarySearch(e.target.checked)}
            className="w-4 h-4 mt-0.5"
          />
          <div>
            <span className="text-sm font-medium block">{t('settings.hideLibrarySearch')}</span>
            <p className="text-xs text-muted-foreground mt-1">{t('settings.hideLibrarySearchDescription')}</p>
          </div>
        </label>

        <label className="flex items-start gap-3 cursor-pointer">
          <input
            type="checkbox"
            checked={hideWindowControls}
            onChange={e => setHideWindowControls(e.target.checked)}
            className="w-4 h-4 mt-0.5"
          />
          <div>
            <span className="text-sm font-medium block">{t('settings.hideWindowControls')}</span>
            <p className="text-xs text-muted-foreground mt-1">{t('settings.hideWindowControlsDescription')}</p>
          </div>
        </label>
      </section>

      {/* Theme — always at the bottom */}
      <section>
        <h2 className="text-lg font-semibold mb-4">{t('settings.theme')}</h2>
        <ThemePicker showImportExport={true} showAccessibilityInfo={true} />
      </section>
    </div>
  );
}
