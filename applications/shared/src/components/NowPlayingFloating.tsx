import { useState, useCallback, useRef, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useSidebarState } from '../contexts/SidebarStateContext';
import { useCurrentTrack, useIsPlaying, useVolume, usePlayerStore } from '../stores/player';
import { usePlayerCommands } from '../contexts/PlayerCommandsContext';
import { ArtworkImage } from './ArtworkImage';
import { ProgressBar } from './player/ProgressBar';
import { PlaybackControls } from './sidebar/PlaybackControls';
import { usePlaybackHandlers } from '../hooks/usePlaybackHandlers';
import { debug } from '../utils/debug';
import { Volume2, VolumeX } from 'lucide-react';

/**
 * NowPlayingFloating — full-width Spotify-style player bar anchored to the
 * bottom of the viewport, shown when the sidebar is collapsed and a track
 * is loaded.
 *
 * Layout (3-column, full width):
 *   [art · title · artist]   [⇄ ⏮ ▶ ⏭ ↺ / ─── progress ───]   [🔊 ─── vol ─── 80]
 */
export function NowPlayingFloating() {
  const navigate = useNavigate();
  const { isCollapsed } = useSidebarState();
  const currentTrack = useCurrentTrack();
  const isPlaying = useIsPlaying();
  const volume = useVolume();
  const shuffleMode = usePlayerStore((s) => s.shuffleMode);
  const repeatMode = usePlayerStore((s) => s.repeatMode);
  const handlers = usePlaybackHandlers();
  const commands = usePlayerCommands();

  const [isMuted, setIsMuted] = useState(false);
  const [volumeBeforeMute, setVolumeBeforeMute] = useState(volume);
  const debounceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const volumeTrackRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (volume > 0 && !isMuted) setVolumeBeforeMute(volume);
  }, [volume, isMuted]);

  const applyVolumeChange = useCallback(
    (newVolume: number) => {
      const v = Math.max(0, Math.min(1, newVolume));
      usePlayerStore.getState().setVolume(v);
      if (v > 0 && isMuted) setIsMuted(false);
      if (debounceTimerRef.current) clearTimeout(debounceTimerRef.current);
      debounceTimerRef.current = setTimeout(() => {
        commands.setVolume(v).catch((e) => debug.error('[NowPlayingFloating] volume', e));
      }, 150);
    },
    [commands, isMuted]
  );

  const handleMuteToggle = useCallback(async () => {
    try {
      if (isMuted) {
        await commands.setVolume(volumeBeforeMute);
        usePlayerStore.getState().setVolume(volumeBeforeMute);
        setIsMuted(false);
      } else {
        setVolumeBeforeMute(volume);
        await commands.setVolume(0);
        usePlayerStore.getState().setVolume(0);
        setIsMuted(true);
      }
    } catch (e) {
      debug.error('[NowPlayingFloating] mute', e);
    }
  }, [commands, isMuted, volume, volumeBeforeMute]);

  const handleVolumeMouseDown = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      e.preventDefault();
      const calc = (clientX: number) => {
        if (!volumeTrackRef.current) return 0;
        const rect = volumeTrackRef.current.getBoundingClientRect();
        return Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
      };
      applyVolumeChange(calc(e.clientX));
      const onMove = (ev: MouseEvent) => applyVolumeChange(calc(ev.clientX));
      const onUp = () => {
        document.removeEventListener('mousemove', onMove);
        document.removeEventListener('mouseup', onUp);
      };
      document.addEventListener('mousemove', onMove);
      document.addEventListener('mouseup', onUp);
    },
    [applyVolumeChange]
  );

  if (!isCollapsed || !currentTrack) return null;

  debug.log('[NowPlayingFloating] rendering with track:', {
    id: currentTrack.id,
    title: currentTrack.title,
    artist: currentTrack.artist,
  });

  const displayVolume = isMuted ? 0 : volume;
  const title = currentTrack.title || 'Unknown Track';
  const artist = currentTrack.artist || 'Unknown Artist';

  return (
    <div
      className="fixed bottom-0 left-0 right-0 z-50 bg-card border-t border-border"
      data-testid="now-playing-floating"
    >
      <div className="grid grid-cols-[minmax(160px,1fr)_minmax(auto,640px)_minmax(160px,1fr)] items-center h-[72px] px-4 gap-4">

        {/* Left: artwork + track info */}
        <div
          className="flex items-center gap-1.5 min-w-0"
          data-testid="floating-track-info"
        >
          <button
            className="flex-shrink-0 rounded-md focus:outline-none focus-visible:ring-2 focus-visible:ring-primary"
            onClick={() => currentTrack.albumId
              ? navigate(`/albums/${currentTrack.albumId}`)
              : undefined
            }
            aria-label={title}
          >
            <ArtworkImage
              trackId={currentTrack.id}
              coverArtPath={currentTrack.coverArtPath}
              alt={title}
              className="w-10 h-10 rounded-md object-cover"
              fallbackIconSize="sm"
            />
          </button>
          <div className="flex flex-col min-w-0">
            <button
              className="text-sm font-semibold truncate text-foreground text-left hover:text-primary transition-colors"
              onClick={() => currentTrack.albumId
                ? navigate(`/albums/${currentTrack.albumId}`)
                : undefined
              }
              data-testid="floating-now-playing-title"
            >
              {title}
            </button>
            <button
              className="text-xs text-muted-foreground truncate text-left hover:text-primary transition-colors"
              onClick={() => currentTrack.artistId
                ? navigate(`/artists/${currentTrack.artistId}`)
                : navigate('/artists')
              }
              data-testid="floating-now-playing-artist"
            >
              {artist}
            </button>
          </div>
        </div>

        {/* Middle: playback controls + progress bar — always centered */}
        <div className="flex flex-col items-center justify-center gap-0.5 min-w-0">
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
          <div className="w-full" data-testid="floating-progress-bar">
            <ProgressBar />
          </div>
        </div>

        {/* Right: volume */}
        <div className="flex items-center gap-2 justify-end min-w-0">
          <button
            onClick={handleMuteToggle}
            className="text-muted-foreground hover:text-foreground transition-colors shrink-0"
            aria-pressed={isMuted}
            data-testid="floating-volume-mute"
          >
            {isMuted || volume === 0
              ? <VolumeX className="w-4 h-4" />
              : <Volume2 className="w-4 h-4" />
            }
          </button>
          <div
            ref={volumeTrackRef}
            className="w-24 h-4 flex items-center cursor-pointer"
            onMouseDown={handleVolumeMouseDown}
            data-testid="floating-volume-slider"
          >
            <div className="relative w-full h-1 bg-muted rounded-full overflow-hidden">
              <div
                className="absolute inset-y-0 left-0 bg-primary rounded-full"
                style={{ width: `${displayVolume * 100}%` }}
              />
            </div>
          </div>
          <span className="text-[10px] font-mono w-6 text-right shrink-0 text-muted-foreground">
            {Math.round(displayVolume * 100)}
          </span>
        </div>

      </div>
    </div>
  );
}
