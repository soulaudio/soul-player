import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { Volume2, Bug, ExternalLink, FileText, CheckCircle, AlertCircle } from 'lucide-react';
import { useBackend, SITE_URL, GITHUB_URL } from '@soul-player/shared';
import { invoke } from '@tauri-apps/api/core';
import { debug } from '@soul-player/shared';

export function AboutSettingsPage() {
  const { t } = useTranslation();
  const backend = useBackend();
  const [version, setVersion] = useState<string>('...');
  const [loggingEnabled, setLoggingEnabled] = useState(false);
  const [showRestartMessage, setShowRestartMessage] = useState(false);

  useEffect(() => {
    backend.getVersion().then(setVersion).catch(() => setVersion('—'));

    const loadLoggingSetting = async () => {
      try {
        const value = await invoke<string | null>('get_user_setting', { key: 'app.logging_enabled' });
        if (value !== null) setLoggingEnabled(JSON.parse(value));
      } catch (error) {
        debug.error('Failed to load logging setting:', error);
      }
    };
    loadLoggingSetting();
  }, [backend]);

  const handleLoggingToggle = async (enabled: boolean) => {
    try {
      await invoke('set_logging_enabled', { enabled });
      setLoggingEnabled(enabled);
      setShowRestartMessage(true);
      setTimeout(() => setShowRestartMessage(false), 5000);
    } catch (error) {
      debug.error('Failed to save logging setting:', error);
    }
  };

  return (
    <div className="max-w-2xl space-y-8">
      <div>
        <h1 className="text-2xl font-bold mb-6">{t('settings.about')}</h1>

        <div className="flex items-center gap-4 mb-6">
          <div className="w-14 h-14 bg-primary/10 rounded-xl flex items-center justify-center">
            <Volume2 className="w-7 h-7 text-primary" />
          </div>
          <div>
            <h3 className="text-lg font-semibold">{t('app.title', 'Soul Player')}</h3>
            <p className="text-sm text-muted-foreground">
              {t('settings.version')} {version}
            </p>
          </div>
        </div>

        <p className="text-sm text-muted-foreground mb-6">
          {t('settings.aboutDescription')}
        </p>

        <section className="mb-6">
          <h2 className="text-xs font-semibold uppercase tracking-widest text-muted-foreground mb-4">{t('settings.links')}</h2>
          <div className="space-y-2">
            <a
              href={GITHUB_URL}
              target="_blank"
              rel="noopener noreferrer"
              className="block text-sm text-primary hover:underline"
            >
              {t('settings.github')}
            </a>
            <a
              href={SITE_URL}
              target="_blank"
              rel="noopener noreferrer"
              className="block text-sm text-primary hover:underline"
            >
              {t('settings.website')}
            </a>
          </div>
        </section>
      </div>

      {/* Report a Bug */}
      <section>
        <h2 className="text-xs font-semibold uppercase tracking-widest text-muted-foreground mb-4">{t('settings.sections.reportBug')}</h2>

        <div className="bg-muted/30 rounded-lg p-6 mb-6">
          <div className="flex items-start gap-4 mb-6">
            <div className="w-12 h-12 bg-primary/10 rounded-lg flex items-center justify-center flex-shrink-0">
              <Bug className="w-6 h-6 text-primary" />
            </div>
            <div>
              <h3 className="text-base font-semibold mb-1">{t('settings.reportBugTitle')}</h3>
              <p className="text-sm text-muted-foreground">
                {t('settings.reportBugInstructions')}
              </p>
            </div>
          </div>

          <div className="space-y-4 mb-6">
            {[1, 2, 3].map((step) => (
              <div key={step} className="flex items-start gap-3">
                <div className="w-6 h-6 bg-primary rounded-full flex items-center justify-center flex-shrink-0 mt-0.5">
                  <span className="text-xs font-bold text-primary-foreground">{step}</span>
                </div>
                <div>
                  <p className="text-sm font-medium">{t(`settings.reportBugSteps.step${step}`)}</p>
                  <p className="text-xs text-muted-foreground mt-1">{t(`settings.reportBugSteps.step${step}Description`)}</p>
                </div>
              </div>
            ))}
          </div>

          <div className="pt-4 border-t border-border">
            <button
              onClick={() => window.open('https://github.com/soulaudio/soul-player/issues', '_blank', 'noopener,noreferrer')}
              className="inline-flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:opacity-[var(--hover-button-opacity)] transition-opacity duration-[var(--transition-duration)]"
            >
              <ExternalLink className="w-4 h-4" />
              {t('settings.reportBugGithubLink')}
            </button>
          </div>
        </div>

        {/* Logs - Desktop only */}
        <div className="mb-6">
          <div className="flex items-start gap-4 mb-4">
            <div className="w-12 h-12 bg-blue-500/10 rounded-lg flex items-center justify-center flex-shrink-0">
              <FileText className="w-6 h-6 text-blue-500" />
            </div>
            <div>
              <h3 className="text-base font-semibold mb-1">{t('settings.reportBugLogs')}</h3>
              <p className="text-sm text-muted-foreground">{t('settings.reportBugLogsDescription')}</p>
            </div>
          </div>

          <label className="flex items-start gap-3 cursor-pointer p-3 rounded-lg hover:bg-foreground/[var(--hover-bg-opacity)] transition-colors duration-[var(--transition-duration)] mb-3">
            <input
              type="checkbox"
              checked={loggingEnabled}
              onChange={(e) => handleLoggingToggle(e.target.checked)}
              className="w-4 h-4 mt-0.5"
            />
            <div className="flex-1">
              <div className="text-sm font-medium">{t('settings.reportBugLoggingToggleLabel')}</div>
              <p className="text-xs text-muted-foreground mt-1">{t('settings.reportBugLoggingToggleDescription')}</p>
            </div>
          </label>

          {showRestartMessage && (
            <div className="mb-3 p-3 bg-amber-500/10 border border-amber-500/20 rounded-lg flex items-start gap-2">
              <AlertCircle className="w-5 h-5 text-amber-500 flex-shrink-0 mt-0.5" />
              <p className="text-sm text-amber-700 dark:text-amber-400">
                {t('settings.reportBugLoggingRestartRequired')}
              </p>
            </div>
          )}

          <div className="bg-muted/30 rounded-lg p-4">
            <div className="space-y-3">
              {[
                { label: 'Windows', path: '%APPDATA%\\Soul Player\\logs\\soul-player.log.YYYY-MM-DD' },
                { label: 'macOS', path: '~/Library/Application Support/soul-player/logs/soul-player.log.YYYY-MM-DD' },
                { label: 'Linux', path: '~/.config/soul-player/logs/soul-player.log.YYYY-MM-DD' },
              ].map(({ label, path }) => (
                <div key={label} className="flex items-start gap-2">
                  <CheckCircle className="w-4 h-4 text-green-500 mt-0.5 flex-shrink-0" />
                  <div className="text-sm">
                    <span className="font-medium">{label}:</span>
                    <code className="ml-2 text-xs bg-muted px-2 py-1 rounded">{path}</code>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>

        {/* Helpful Resources */}
        <div>
          <h3 className="text-xs font-semibold uppercase tracking-widest text-muted-foreground mb-4">{t('settings.reportBugResourcesHeading')}</h3>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <a
              href="https://github.com/soulaudio/soul-player"
              target="_blank"
              rel="noopener noreferrer"
              className="block p-4 bg-muted/30 rounded-lg hover:bg-foreground/[var(--hover-bg-opacity)] transition-colors duration-[var(--transition-duration)]"
            >
              <div className="flex items-center gap-3">
                <ExternalLink className="w-5 h-5 text-primary" />
                <div>
                  <h4 className="font-medium text-sm">{t('settings.reportBugGithubRepoLabel')}</h4>
                  <p className="text-xs text-muted-foreground">{t('settings.reportBugGithubRepoDescription')}</p>
                </div>
              </div>
            </a>
            <a
              href="https://github.com/soulaudio/soul-player/wiki"
              target="_blank"
              rel="noopener noreferrer"
              className="block p-4 bg-muted/30 rounded-lg hover:bg-foreground/[var(--hover-bg-opacity)] transition-colors duration-[var(--transition-duration)]"
            >
              <div className="flex items-center gap-3">
                <FileText className="w-5 h-5 text-primary" />
                <div>
                  <h4 className="font-medium text-sm">{t('settings.reportBugWikiLabel')}</h4>
                  <p className="text-xs text-muted-foreground">{t('settings.reportBugWikiDescription')}</p>
                </div>
              </div>
            </a>
          </div>
        </div>
      </section>
    </div>
  );
}
