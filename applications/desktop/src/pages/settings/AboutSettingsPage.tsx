import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { Volume2 } from 'lucide-react';
import { useBackend, SITE_URL, GITHUB_URL } from '@soul-player/shared';

export function AboutSettingsPage() {
  const { t } = useTranslation();
  const backend = useBackend();
  const [version, setVersion] = useState<string>('...');

  useEffect(() => {
    backend.getVersion().then(setVersion).catch(() => setVersion('—'));
  }, [backend]);

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
  );
}
