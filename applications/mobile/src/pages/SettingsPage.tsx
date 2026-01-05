import { ThemePicker } from '@soul-player/shared/theme';

export function SettingsPage() {
  return (
    <div className="p-4 pb-safe">
      <h1 className="text-2xl font-bold mb-4">Settings</h1>

      {/* Appearance Section */}
      <section className="mb-6">
        <h2 className="text-xl font-semibold mb-3">Appearance</h2>
        <ThemePicker
          showImportExport={true}
          showAccessibilityInfo={true}
        />
      </section>

      {/* Future sections */}
      <section className="mb-6">
        <h2 className="text-xl font-semibold mb-3">Audio Settings</h2>
        <p className="text-sm text-muted-foreground">
          Audio configuration coming soon...
        </p>
      </section>

      <section>
        <h2 className="text-xl font-semibold mb-3">About</h2>
        <div className="bg-muted/40 rounded-lg p-3 space-y-2">
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
