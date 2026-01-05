import { ThemePicker } from '@soul-player/shared/theme';

export function SettingsPage() {
  return (
    <div>
      <h1 className="text-3xl font-bold mb-6">Settings</h1>

      {/* Appearance Section */}
      <section className="mb-8">
        <h2 className="text-2xl font-semibold mb-4">Appearance</h2>
        <ThemePicker
          showImportExport={true}
          showAccessibilityInfo={true}
        />
      </section>

      {/* Future sections */}
      <section className="mb-8">
        <h2 className="text-2xl font-semibold mb-4">Audio Settings</h2>
        <p className="text-muted-foreground">
          Audio configuration coming soon...
        </p>
      </section>

      <section>
        <h2 className="text-2xl font-semibold mb-4">About</h2>
        <div className="bg-muted/40 rounded-lg p-4 space-y-2">
          <p className="text-sm">
            <span className="font-medium">Soul Player</span> - Local-first music player
          </p>
          <p className="text-xs text-muted-foreground">
            Version 0.1.0
          </p>
        </div>
      </section>
    </div>
  );
}
