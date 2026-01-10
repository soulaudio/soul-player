import { usePlayerStore } from '@soul-player/shared';
import { Music } from 'lucide-react';

export function HomePage() {
  const { currentTrack, isPlaying } = usePlayerStore();

  return (
    <div className="p-6 space-y-8">
      <div>
        <h1 className="text-3xl font-bold text-foreground">Welcome</h1>
        <p className="text-muted-foreground mt-2">Your music, everywhere.</p>
      </div>

      {/* Now Playing Card */}
      <div className="bg-card border border-border rounded-lg p-6">
        <h2 className="text-xl font-semibold text-foreground mb-4">Now Playing</h2>

        {currentTrack ? (
          <div className="flex items-center gap-4">
            <div className="w-20 h-20 bg-muted rounded-lg flex items-center justify-center">
              {currentTrack.coverArtPath ? (
                <img
                  src={currentTrack.coverArtPath}
                  alt={currentTrack.title}
                  className="w-full h-full object-cover rounded-lg"
                />
              ) : (
                <Music className="w-8 h-8 text-muted-foreground" />
              )}
            </div>
            <div>
              <p className="font-medium text-foreground">{currentTrack.title}</p>
              <p className="text-sm text-muted-foreground">{currentTrack.artist}</p>
              {currentTrack.album && (
                <p className="text-sm text-muted-foreground">{currentTrack.album}</p>
              )}
            </div>
            {isPlaying && (
              <div className="ml-auto flex items-center gap-1">
                <span className="w-1 h-4 bg-primary rounded-full animate-pulse" />
                <span className="w-1 h-6 bg-primary rounded-full animate-pulse delay-75" />
                <span className="w-1 h-3 bg-primary rounded-full animate-pulse delay-150" />
              </div>
            )}
          </div>
        ) : (
          <p className="text-muted-foreground">No track playing. Select something from your library!</p>
        )}
      </div>

      {/* Quick Actions */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <a
          href="/library"
          className="bg-card border border-border rounded-lg p-6 hover:bg-accent transition-colors"
        >
          <h3 className="font-semibold text-foreground">Library</h3>
          <p className="text-sm text-muted-foreground mt-1">Browse your music collection</p>
        </a>

        <a
          href="/settings"
          className="bg-card border border-border rounded-lg p-6 hover:bg-accent transition-colors"
        >
          <h3 className="font-semibold text-foreground">Settings</h3>
          <p className="text-sm text-muted-foreground mt-1">Customize your experience</p>
        </a>
      </div>
    </div>
  );
}
