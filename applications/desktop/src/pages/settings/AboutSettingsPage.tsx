import { useTranslation } from 'react-i18next';
import { Volume2 } from 'lucide-react';

export function AboutSettingsPage() {
  const { t } = useTranslation();

  return (
    <div className="max-w-2xl">
      <h1 className="text-2xl font-bold mb-6">{t('settings.about')}</h1>

      <div className="flex items-center gap-4 mb-6">
        <div className="w-14 h-14 bg-primary/10 rounded-xl flex items-center justify-center">
          <Volume2 className="w-7 h-7 text-primary" />
        </div>
        <div>
          <h3 className="text-lg font-semibold">{t('app.title', 'Soul Player')}</h3>
          <p className="text-sm text-muted-foreground">
            {t('settings.version')} {process.env.TAURI_VERSION || '0.1.7'}
          </p>
        </div>
      </div>

      <p className="text-sm text-muted-foreground mb-6">
        {t('settings.aboutDescription')}
      </p>

      <section className="mb-6">
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
