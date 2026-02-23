import { useTranslation } from 'react-i18next';
import { LibrarySettingsPage, DataManagementSettingsPage } from '@soul-player/shared';

export function MusicDataSettingsPage() {
  const { t } = useTranslation();

  return (
    <div className="max-w-2xl space-y-8">
      {/* Sources section */}
      <section>
        <h2 className="text-xs font-semibold uppercase tracking-widest text-muted-foreground mb-4">
          {t('settings.musicData.sourcesHeading')}
        </h2>
        <LibrarySettingsPage />
      </section>

      {/* Data section */}
      <section className="border-t border-border pt-8">
        <h2 className="text-xs font-semibold uppercase tracking-widest text-muted-foreground mb-4">
          {t('settings.musicData.dataHeading')}
        </h2>
        <DataManagementSettingsPage embedded />
      </section>
    </div>
  );
}
