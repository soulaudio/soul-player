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
    // Outer: full-width fixed strip at bottom — no overflow possible
    <div
      className="fixed bottom-0 left-0 right-0 z-50 flex justify-center px-4 pb-4"
      data-testid="now-playing-floating"
    >
      {/* Card: max-width constrained, fills available width */}
      <div className="w-full max-w-[820px] bg-card border border-border rounded-xl shadow-lg px-4 py-3 overflow-hidden">
        <div className="flex items-center gap-3 min-w-0">

          {/* Album artwork */}
          <ArtworkImage
            trackId={currentTrack.id}
            coverArtPath={currentTrack.coverArtPath}
            alt={currentTrack.title}
            className="w-12 h-12 rounded-md object-cover flex-shrink-0"
            fallbackIconSize="sm"
          />

          {/* Track title + artist */}
          <div className="w-36 flex-shrink-0 overflow-hidden">
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

          {/* Center: controls above progress bar, fills remaining space */}
          <div className="flex-1 flex flex-col gap-1 min-w-0 overflow-hidden">
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
            <div data-testid="floating-progress-bar">
              <ProgressBar />
            </div>
          </div>

        </div>
      </div>
    </div>
  );
}
