import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import i18next from 'i18next';
import { ThemePicker, useBackend, debug } from '@soul-player/shared';
import { useSettings } from '../../contexts/SettingsContext';

const LANGUAGES = [
  { code: 'en-US', label: 'English' },
  { code: 'de',    label: 'Deutsch' },
  { code: 'ja',    label: '日本語'  },
] as const;

export function AppearanceSettingsPage() {
  const { t } = useTranslation();
  const backend = useBackend();
  const { hideWindowControls, setHideWindowControls } = useSettings();

  const [homeEnabled, setHomeEnabledState] = useState(true);
  const [hideLibrarySearch, setHideLibrarySearchState] = useState(true);
  const [showLibraryGradients, setShowLibraryGradientsState] = useState(true);
  const [language, setLanguageState] = useState(i18next.language || 'en-US');

  useEffect(() => {
    Promise.all([
      backend.getUserSetting('home.enabled'),
      backend.getUserSetting('ui.hide_library_search'),
      backend.getUserSetting('ui.show_library_gradients'),
      backend.getUserSetting('ui.language'),
    ])
      .then(([home, hideSearch, showGradients, lang]) => {
        setHomeEnabledState(home ?? true);
        setHideLibrarySearchState(hideSearch ?? true);
        setShowLibraryGradientsState(showGradients ?? true);
        if (lang) setLanguageState(lang as string);
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

  const handleLanguageChange = useCallback((code: string) => {
    setLanguageState(code);
    i18next.changeLanguage(code);
    backend.setUserSetting('ui.language', code)
      .catch(err => debug.error('Failed to save language setting:', err));
  }, [backend]);

  const handleShowLibraryGradients = useCallback((show: boolean) => {
    setShowLibraryGradientsState(show);
    backend.setUserSetting('ui.show_library_gradients', show)
      .then(() => {
        window.dispatchEvent(new CustomEvent('library-gradients-changed', { detail: { show } }));
      })
      .catch(err => debug.error('Failed to save library gradients setting:', err));
  }, [backend]);

  const handleHideLibrarySearch = useCallback((autoHide: boolean) => {
    setHideLibrarySearchState(autoHide);
    backend.setUserSetting('ui.hide_library_search', autoHide)
      .then(() => {
        window.dispatchEvent(new CustomEvent('library-search-hidden-changed', { detail: { autoHide } }));
      })
      .catch(err => debug.error('Failed to save library search setting:', err));
  }, [backend]);

  return (
    <div className="max-w-2xl space-y-8">
      <div>
        <h1 className="text-2xl font-bold mb-6">{t('settings.appearance')}</h1>
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
            checked={showLibraryGradients}
            onChange={e => handleShowLibraryGradients(e.target.checked)}
            className="w-4 h-4 mt-0.5"
          />
          <div>
            <span className="text-sm font-medium block">{t('settings.showLibraryGradients')}</span>
            <p className="text-xs text-muted-foreground mt-1">{t('settings.showLibraryGradientsDescription')}</p>
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

      {/* Localization */}
      <section>
        <h2 className="text-xs font-semibold uppercase tracking-widest text-muted-foreground mb-4">{t('settings.localization')}</h2>
        <select
          value={language}
          onChange={e => handleLanguageChange(e.target.value)}
          className="px-3 py-2 text-sm rounded-lg border border-border bg-background text-foreground focus:outline-none focus:ring-2 focus:ring-primary"
        >
          {LANGUAGES.map(lang => (
            <option key={lang.code} value={lang.code}>{lang.label}</option>
          ))}
        </select>
      </section>

      {/* Theme — always at the bottom */}
      <section>
        <h2 className="text-xs font-semibold uppercase tracking-widest text-muted-foreground mb-4">{t('settings.theme')}</h2>
        <ThemePicker showImportExport={true} showAccessibilityInfo={true} />
      </section>
    </div>
  );
}
