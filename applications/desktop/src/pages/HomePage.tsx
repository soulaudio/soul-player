import { useNavigate } from 'react-router-dom';
import { usePlayerStore } from '@soul-player/shared/stores/player';
import { ArtworkImage, usePlayerCommands } from '@soul-player/shared';
import { Play, Pause, SkipBack, SkipForward, Music, ListMusic, Users, Disc3, Guitar, Sparkles, Radio } from 'lucide-react';

export function HomePage() {
  const navigate = useNavigate();
  const { currentTrack, isPlaying } = usePlayerStore();
  const { resumePlayback, pausePlayback, skipNext, skipPrevious } = usePlayerCommands();

  const hasPlayingTrack = currentTrack !== null;

  const handlePlayPause = async () => {
    if (isPlaying) {
      await pausePlayback();
    } else {
      await resumePlayback();
    }
  };

  return (
    <div className="h-full flex flex-col space-y-8">
      {/* Discovery Section - Always visible */}
      <section>
        <div className="flex items-center gap-2 mb-4">
          <Sparkles className="w-5 h-5 text-primary" />
          <h2 className="text-xl font-bold">Discovery</h2>
        </div>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <DiscoveryCard
            icon={<Radio className="w-6 h-6" />}
            title="Radio Stations"
            subtitle="Coming soon"
            disabled
          />
          <DiscoveryCard
            icon={<Sparkles className="w-6 h-6" />}
            title="Recommendations"
            subtitle="Coming soon"
            disabled
          />
          <DiscoveryCard
            icon={<Music className="w-6 h-6" />}
            title="New Releases"
            subtitle="Coming soon"
            disabled
          />
          <DiscoveryCard
            icon={<ListMusic className="w-6 h-6" />}
            title="Curated Playlists"
            subtitle="Coming soon"
            disabled
          />
        </div>
      </section>

      {/* Now Playing / Discovery Placeholder Section */}
      <section>
        {hasPlayingTrack ? (
          <>
            <div className="flex items-center gap-2 mb-4">
              <Music className="w-5 h-5 text-primary" />
              <h2 className="text-xl font-bold">Now Playing</h2>
            </div>
            <div
              onClick={() => navigate('/now-playing')}
              role="button"
              tabIndex={0}
              onKeyDown={(e) => e.key === 'Enter' && navigate('/now-playing')}
              className="w-full bg-card rounded-xl p-6 border hover:border-primary hover:bg-accent/30 transition-all text-left group cursor-pointer"
            >
              <div className="flex items-center gap-6">
                {/* Album Artwork */}
                <div className="w-32 h-32 rounded-lg overflow-hidden bg-muted flex-shrink-0">
                  <ArtworkImage
                    trackId={currentTrack.id}
                    coverArtPath={currentTrack.coverArtPath}
                    alt={currentTrack.album || currentTrack.title}
                    className="w-full h-full object-cover"
                    fallbackClassName="w-full h-full flex items-center justify-center bg-muted"
                  />
                </div>

                {/* Track Info */}
                <div className="flex-1 min-w-0">
                  <h3 className="text-2xl font-bold truncate group-hover:text-primary transition-colors">{currentTrack.title}</h3>
                  <p className="text-lg text-muted-foreground truncate">{currentTrack.artist}</p>
                  {currentTrack.album && (
                    <p className="text-sm text-muted-foreground truncate mt-1">{currentTrack.album}</p>
                  )}
                </div>

                {/* Playback Controls */}
                <div className="flex items-center gap-4" onClick={(e) => e.stopPropagation()}>
                  <button
                    onClick={skipPrevious}
                    className="p-3 rounded-full hover:bg-accent transition-colors"
                    aria-label="Previous track"
                  >
                    <SkipBack className="w-6 h-6" />
                  </button>
                  <button
                    onClick={handlePlayPause}
                    className="p-4 rounded-full bg-primary text-primary-foreground hover:bg-primary/90 transition-colors"
                    aria-label={isPlaying ? 'Pause' : 'Play'}
                  >
                    {isPlaying ? <Pause className="w-6 h-6" /> : <Play className="w-6 h-6 ml-0.5" />}
                  </button>
                  <button
                    onClick={skipNext}
                    className="p-3 rounded-full hover:bg-accent transition-colors"
                    aria-label="Next track"
                  >
                    <SkipForward className="w-6 h-6" />
                  </button>
                </div>
              </div>
            </div>
          </>
        ) : (
          <>
            <div className="flex items-center gap-2 mb-4">
              <Music className="w-5 h-5 text-muted-foreground" />
              <h2 className="text-xl font-bold text-muted-foreground">Now Playing</h2>
            </div>
            <div className="bg-card/50 rounded-xl p-8 border border-dashed">
              <div className="flex flex-col items-center justify-center text-center">
                <div className="w-16 h-16 rounded-full bg-muted flex items-center justify-center mb-4">
                  <Music className="w-8 h-8 text-muted-foreground" />
                </div>
                <h3 className="text-lg font-medium text-muted-foreground">Nothing playing</h3>
                <p className="text-sm text-muted-foreground mt-1">
                  Select a track from your library to start listening
                </p>
                <button
                  onClick={() => navigate('/library')}
                  className="mt-4 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors"
                >
                  Browse Library
                </button>
              </div>
            </div>
          </>
        )}
      </section>

      {/* Library Section - Merged categories */}
      <section>
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <Music className="w-5 h-5 text-primary" />
            <h2 className="text-xl font-bold">Library</h2>
          </div>
          <button
            onClick={() => navigate('/library')}
            className="text-sm text-primary hover:underline"
          >
            View All
          </button>
        </div>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <LibraryCard
            icon={<ListMusic className="w-6 h-6" />}
            title="Playlists"
            onClick={() => navigate('/library?tab=playlists')}
          />
          <LibraryCard
            icon={<Users className="w-6 h-6" />}
            title="Artists"
            onClick={() => navigate('/library?tab=artists')}
          />
          <LibraryCard
            icon={<Disc3 className="w-6 h-6" />}
            title="Albums"
            onClick={() => navigate('/library?tab=albums')}
          />
          <LibraryCard
            icon={<Guitar className="w-6 h-6" />}
            title="Genres"
            onClick={() => navigate('/library?tab=genres')}
          />
        </div>
      </section>
    </div>
  );
}

interface DiscoveryCardProps {
  icon: React.ReactNode;
  title: string;
  subtitle?: string;
  disabled?: boolean;
  onClick?: () => void;
}

function DiscoveryCard({ icon, title, subtitle, disabled, onClick }: DiscoveryCardProps) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className={`p-4 rounded-xl border text-left transition-all ${
        disabled
          ? 'bg-muted/50 border-dashed opacity-60 cursor-not-allowed'
          : 'bg-card hover:bg-accent hover:border-primary cursor-pointer'
      }`}
    >
      <div className="flex items-center gap-3">
        <div className={`p-2 rounded-lg ${disabled ? 'bg-muted' : 'bg-primary/10 text-primary'}`}>
          {icon}
        </div>
        <div>
          <h3 className="font-medium">{title}</h3>
          {subtitle && <p className="text-xs text-muted-foreground">{subtitle}</p>}
        </div>
      </div>
    </button>
  );
}

interface LibraryCardProps {
  icon: React.ReactNode;
  title: string;
  count?: number;
  onClick?: () => void;
}

function LibraryCard({ icon, title, count, onClick }: LibraryCardProps) {
  return (
    <button
      onClick={onClick}
      className="p-4 rounded-xl bg-card border hover:bg-accent hover:border-primary transition-all text-left"
    >
      <div className="flex items-center gap-3">
        <div className="p-2 rounded-lg bg-primary/10 text-primary">
          {icon}
        </div>
        <div>
          <h3 className="font-medium">{title}</h3>
          {count !== undefined && (
            <p className="text-xs text-muted-foreground">{count} items</p>
          )}
        </div>
      </div>
    </button>
  );
}
