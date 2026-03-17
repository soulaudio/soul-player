import { useSidebarState } from '../contexts/SidebarStateContext';
import { useCurrentTrack, useIsPlaying, usePlayerStore } from '../stores/player';
import { ArtworkImage } from './ArtworkImage';
import { ProgressBar } from './player/ProgressBar';
import { PlaybackControls } from './sidebar/PlaybackControls';
import { usePlaybackHandlers } from '../hooks/usePlaybackHandlers';

/**
 * NowPlayingFloating — centered fixed bar shown when the sidebar is collapsed
 * and a track is playing. Reads collapse state from SidebarStateContext and
 * track data from the Zustand player store.
 *
 * Layout (wide bar, max 560px):
 *   [artwork 56×56] | Title
 *                   | Artist · Album
 *                   | ── seek bar ──  0:00 / 0:00
 *                   | ⇄  ⏮  ▶  ⏭  ↺
 */
export function NowPlayingFloating() {
  const { isCollapsed } = useSidebarState();
  const currentTrack = useCurrentTrack();
  const isPlaying = useIsPlaying();
  const shuffleMode = usePlayerStore((s) => s.shuffleMode);
  const repeatMode = usePlayerStore((s) => s.repeatMode);
  const handlers = usePlaybackHandlers();

  // Only mount when the sidebar is collapsed AND a track is loaded
  if (!isCollapsed || !currentTrack) return null;

  return (
    <div
      className="fixed left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 z-50 max-w-[560px] w-[90vw]"
      data-testid="now-playing-floating"
    >
      <div className="bg-card border border-border rounded-xl shadow-lg p-4">
        <div className="flex gap-4 items-center">

          {/* Album artwork — ArtworkImage handles URI scheme, caching, and fallback */}
          <ArtworkImage
            trackId={currentTrack.id}
            coverArtPath={currentTrack.coverArtPath}
            alt={currentTrack.title}
            className="w-14 h-14 rounded-md object-cover flex-shrink-0"
            fallbackIconSize="sm"
          />

          {/* Track info + controls */}
          <div className="flex-1 min-w-0 space-y-2">

            {/* Track title + artist */}
            <div>
              <p
                className="text-sm font-semibold truncate"
                data-testid="floating-now-playing-title"
              >
                {currentTrack.title}
              </p>
              <p
                className="text-xs text-muted-foreground truncate"
                data-testid="floating-now-playing-artist"
              >
                {currentTrack.artist}
              </p>
            </div>

            {/* Seek bar — second ProgressBar instance, safe to mount alongside sidebar's */}
            <div data-testid="floating-progress-bar">
              <ProgressBar />
            </div>

            {/* Playback controls */}
            <PlaybackControls
              isPlaying={isPlaying}
              hasCurrentTrack={true}
              shuffleMode={shuffleMode}
              repeatMode={repeatMode}
              onPlayPause={handlers.onPlayPause}
              onPrevious={handlers.onPrevious}
              onNext={handlers.onNext}
              onShuffleToggle={handlers.onShuffleToggle}
              onRepeatToggle={handlers.onRepeatToggle}
            />

          </div>
        </div>
      </div>
    </div>
  );
}
