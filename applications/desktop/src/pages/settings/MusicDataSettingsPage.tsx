import { useTranslation } from 'react-i18next';
import { LibrarySettingsPage, DataManagementSettingsPage } from '@soul-player/shared';

export function MusicDataSettingsPage() {
  const { t } = useTranslation();

  return (
    <div className="max-w-2xl space-y-8">
      <h1 className="text-2xl font-bold mb-6">{t('settings.sections.musicData')}</h1>

      {/* Sources section */}
      <section>
        <h2 className="text-xs font-semibold uppercase tracking-widest text-muted-foreground mb-4">
          {t('settings.musicData.sourcesHeading')}
        </h2>
        <LibrarySettingsPage />
      </section>

      {/* Data section */}
      <section>
        <DataManagementSettingsPage embedded />
      </section>
    </div>
  );
}
