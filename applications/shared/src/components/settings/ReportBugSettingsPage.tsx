import { useTranslation } from 'react-i18next';
import { Bug, ExternalLink, FileText, CheckCircle } from 'lucide-react';
import { usePlatform } from '../../contexts/PlatformContext';

export function ReportBugSettingsPage() {
  const { t } = useTranslation();
  const { isWeb } = usePlatform();

  const handleOpenGitHub = () => {
    window.open('https://github.com/soulaudio/soul-player/issues', '_blank', 'noopener,noreferrer');
  };

  return (
    <div className="space-y-8">
      {/* Header */}
      <div>
        <h1 className="text-3xl font-bold mb-2">{t('settings.reportBugTitle')}</h1>
        <p className="text-muted-foreground">
          {t('settings.reportBugDescription')}
        </p>
      </div>

      {/* Instructions */}
      <section className="bg-muted/30 rounded-lg p-6">
        <div className="flex items-start gap-4 mb-6">
          <div className="w-12 h-12 bg-primary/10 rounded-lg flex items-center justify-center flex-shrink-0">
            <Bug className="w-6 h-6 text-primary" />
          </div>
          <div>
            <h2 className="text-lg font-semibold mb-2">{t('settings.reportBugTitle')}</h2>
            <p className="text-sm text-muted-foreground">
              {t('settings.reportBugInstructions')}
            </p>
          </div>
        </div>

        {/* Steps */}
        <div className="space-y-4">
          <div className="flex items-start gap-3">
            <div className="w-6 h-6 bg-primary rounded-full flex items-center justify-center flex-shrink-0 mt-0.5">
              <span className="text-xs font-bold text-primary-foreground">1</span>
            </div>
            <div>
              <p className="text-sm font-medium">{t('settings.reportBugSteps.step1')}</p>
              <p className="text-xs text-muted-foreground mt-1">
                Search existing issues to avoid duplicates
              </p>
            </div>
          </div>

          <div className="flex items-start gap-3">
            <div className="w-6 h-6 bg-primary rounded-full flex items-center justify-center flex-shrink-0 mt-0.5">
              <span className="text-xs font-bold text-primary-foreground">2</span>
            </div>
            <div>
              <p className="text-sm font-medium">{t('settings.reportBugSteps.step2')}</p>
              <p className="text-xs text-muted-foreground mt-1">
                Include what you expected vs. what actually happened
              </p>
            </div>
          </div>

          <div className="flex items-start gap-3">
            <div className="w-6 h-6 bg-primary rounded-full flex items-center justify-center flex-shrink-0 mt-0.5">
              <span className="text-xs font-bold text-primary-foreground">3</span>
            </div>
            <div>
              <p className="text-sm font-medium">{t('settings.reportBugSteps.step3')}</p>
              <p className="text-xs text-muted-foreground mt-1">
                OS version, Soul Player version, error messages, etc.
              </p>
            </div>
          </div>
        </div>

        {/* GitHub Button */}
        <div className="mt-6 pt-6 border-t border-border">
          <button
            onClick={handleOpenGitHub}
            className="inline-flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors"
          >
            <ExternalLink className="w-4 h-4" />
            {t('settings.reportBugGithubLink')}
          </button>
        </div>
      </section>

      {/* Logs Section - Desktop only */}
      {!isWeb && (
        <section>
          <div className="flex items-start gap-4 mb-4">
            <div className="w-12 h-12 bg-blue-500/10 rounded-lg flex items-center justify-center flex-shrink-0">
              <FileText className="w-6 h-6 text-blue-500" />
            </div>
            <div>
              <h2 className="text-lg font-semibold mb-2">{t('settings.reportBugLogs')}</h2>
              <p className="text-sm text-muted-foreground">
                {t('settings.reportBugLogsDescription')}
              </p>
            </div>
          </div>

          <div className="bg-muted/30 rounded-lg p-4">
            <div className="space-y-3">
              <div className="flex items-start gap-2">
                <CheckCircle className="w-4 h-4 text-green-500 mt-0.5 flex-shrink-0" />
                <div className="text-sm">
                  <span className="font-medium">Windows:</span>
                  <code className="ml-2 text-xs bg-muted px-2 py-1 rounded">
                    %APPDATA%\Soul Player\logs\soul-player.log.YYYY-MM-DD
                  </code>
                </div>
              </div>
              <div className="flex items-start gap-2">
                <CheckCircle className="w-4 h-4 text-green-500 mt-0.5 flex-shrink-0" />
                <div className="text-sm">
                  <span className="font-medium">macOS:</span>
                  <code className="ml-2 text-xs bg-muted px-2 py-1 rounded">
                    ~/Library/Application Support/soul-player/logs/soul-player.log.YYYY-MM-DD
                  </code>
                </div>
              </div>
              <div className="flex items-start gap-2">
                <CheckCircle className="w-4 h-4 text-green-500 mt-0.5 flex-shrink-0" />
                <div className="text-sm">
                  <span className="font-medium">Linux:</span>
                  <code className="ml-2 text-xs bg-muted px-2 py-1 rounded">
                    ~/.config/soul-player/logs/soul-player.log.YYYY-MM-DD
                  </code>
                </div>
              </div>
            </div>

            <div className="mt-4 pt-4 border-t border-border">
              <p className="text-xs text-muted-foreground">
                <strong>Tip:</strong> Run Soul Player with <code className="bg-muted px-2 py-0.5 rounded">--logs</code> flag or use{' '}
                <code className="bg-muted px-2 py-0.5 rounded">yarn dev:desktop:logs</code> to enable file logging.
              </p>
            </div>
          </div>
        </section>
      )}

      {/* Helpful Resources */}
      <section>
        <h2 className="text-lg font-semibold mb-4">Helpful Resources</h2>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <a
            href="https://github.com/soulaudio/soul-player"
            target="_blank"
            rel="noopener noreferrer"
            className="block p-4 bg-muted/30 rounded-lg hover:bg-muted/50 transition-colors"
          >
            <div className="flex items-center gap-3">
              <ExternalLink className="w-5 h-5 text-primary" />
              <div>
                <h3 className="font-medium text-sm">GitHub Repository</h3>
                <p className="text-xs text-muted-foreground">View source code and documentation</p>
              </div>
            </div>
          </a>

          <a
            href="https://github.com/soulaudio/soul-player/wiki"
            target="_blank"
            rel="noopener noreferrer"
            className="block p-4 bg-muted/30 rounded-lg hover:bg-muted/50 transition-colors"
          >
            <div className="flex items-center gap-3">
              <FileText className="w-5 h-5 text-primary" />
              <div>
                <h3 className="font-medium text-sm">Wiki & Documentation</h3>
                <p className="text-xs text-muted-foreground">Guides, FAQs, and troubleshooting</p>
              </div>
            </div>
          </a>
        </div>
      </section>
    </div>
  );
}
