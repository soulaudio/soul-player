import { useTranslation } from 'react-i18next';

export function PlaybackSettingsPage() {
  const { t } = useTranslation();

  return (
    <div className="max-w-2xl">
      <h1 className="text-2xl font-bold mb-2">{t('settings.sections.playback')}</h1>
      <p className="text-muted-foreground mb-6">
        Configure playback behavior and features
      </p>

      <div className="bg-muted/50 border border-border rounded-lg p-6 text-center">
        <p className="text-sm text-muted-foreground">
          Playback settings coming soon. Configure crossfade, gapless playback, and more.
        </p>
      </div>
    </div>
  );
}
