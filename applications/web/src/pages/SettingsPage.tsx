import { ThemePicker } from '@soul-player/shared';
import { useAuth } from '../providers/AuthProvider';

export function SettingsPage() {
  const { user, logout } = useAuth();

  return (
    <div className="p-6 space-y-8 max-w-2xl">
      <div>
        <h1 className="text-2xl font-bold text-foreground">Settings</h1>
        <p className="text-muted-foreground mt-1">Customize your experience</p>
      </div>

      {/* Account Section */}
      <section className="space-y-4">
        <h2 className="text-lg font-semibold text-foreground">Account</h2>
        <div className="bg-card border border-border rounded-lg p-4">
          <div className="flex items-center justify-between">
            <div>
              <p className="font-medium text-foreground">Signed in as</p>
              <p className="text-sm text-muted-foreground">{user?.username || 'Unknown'}</p>
            </div>
            <button
              onClick={logout}
              className="px-4 py-2 bg-destructive text-destructive-foreground rounded-lg hover:bg-destructive/90 transition-colors"
            >
              Sign Out
            </button>
          </div>
        </div>
      </section>

      {/* Appearance Section */}
      <section className="space-y-4">
        <h2 className="text-lg font-semibold text-foreground">Appearance</h2>
        <div className="bg-card border border-border rounded-lg p-4">
          <p className="text-sm text-muted-foreground mb-4">Choose your preferred theme</p>
          <ThemePicker />
        </div>
      </section>

      {/* About Section */}
      <section className="space-y-4">
        <h2 className="text-lg font-semibold text-foreground">About</h2>
        <div className="bg-card border border-border rounded-lg p-4">
          <p className="font-medium text-foreground">Soul Player Web</p>
          <p className="text-sm text-muted-foreground mt-1">
            Cross-platform music player with multi-device sync.
          </p>
        </div>
      </section>
    </div>
  );
}
