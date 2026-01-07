/**
 * Demo Settings Page - simplified version for marketing showcase
 */
'use client'

export function SettingsPage() {
  return (
    <div className="max-w-4xl mx-auto">
      <div className="mb-8">
        <h1 className="text-3xl font-bold mb-2">Settings</h1>
        <p className="text-muted-foreground">
          Configure Soul Player (Demo Mode)
        </p>
      </div>

      {/* Appearance Section */}
      <section className="mb-8 bg-card border rounded-lg p-6">
        <h2 className="text-xl font-semibold mb-4">Appearance</h2>

        <div className="space-y-4">
          <div>
            <label className="text-sm font-medium mb-2 block">Theme</label>
            <p className="text-sm text-muted-foreground">
              Theme switching available in full app. Try the theme switcher above the demo!
            </p>
          </div>
        </div>
      </section>

      {/* Audio Section */}
      <section className="mb-8 bg-card border rounded-lg p-6">
        <h2 className="text-xl font-semibold mb-4">Audio Settings</h2>

        <div className="space-y-4">
          <p className="text-sm text-muted-foreground">
            Audio configuration available in full desktop application
          </p>
        </div>
      </section>

      {/* About Section */}
      <section className="bg-card border rounded-lg p-6">
        <h2 className="text-xl font-semibold mb-4">About</h2>

        <div className="space-y-2 text-sm">
          <p><strong>Soul Player</strong> - Local-first music player</p>
          <p className="text-muted-foreground">Demo Version</p>
          <p className="text-xs text-muted-foreground mt-4">
            This is a limited demo showcasing Soul Player's interface and playback capabilities.
            Download the full app for complete features.
          </p>
        </div>
      </section>
    </div>
  );
}
