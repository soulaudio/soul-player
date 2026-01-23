import { useTranslation } from 'react-i18next';
import { ThemePicker } from '@soul-player/shared';

export function AppearanceSettingsPage() {
  const { t } = useTranslation();

  return (
    <div className="max-w-2xl">
      <h1 className="text-2xl font-bold mb-2">{t('settings.appearance')}</h1>
      <p className="text-muted-foreground mb-6">
        Customize the look and feel of Soul Player
      </p>

      <section>
        <h2 className="text-lg font-semibold mb-4">{t('settings.theme')}</h2>
        <ThemePicker showImportExport={true} showAccessibilityInfo={true} />
      </section>
    </div>
  );
}
