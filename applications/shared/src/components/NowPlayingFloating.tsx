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
 * NowPlayingFloating — compact vertical card anchored bottom-center,
 * shown when the sidebar is collapsed and a track is loaded.
 *
 * Layout (320px wide card):
 *   [art 40×40]  Title
 *                Artist • Album        ← clickable → /now-playing
 *   ─── progress ─────────────────
 *   ⇄   ⏮   ▶   ⏭   ↺
 *   🔊  ── vol ──────────────── 80%
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

  // Inline volume track click/drag
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

  const displayVolume = isMuted ? 0 : volume;
  const artist = currentTrack.artist || 'Unknown Artist';

  return (
    <div
      className="fixed bottom-4 left-1/2 -translate-x-1/2 z-50 w-[min(320px,calc(100vw-32px))]"
      data-testid="now-playing-floating"
    >
      <div className="bg-card border border-border rounded-xl shadow-lg p-3 space-y-2">

        {/* Row 1: artwork + track info — clickable */}
        <button
          className="flex items-center gap-3 w-full text-left hover:opacity-80 transition-opacity"
          onClick={() => navigate('/now-playing')}
          data-testid="floating-track-info"
        >
          <ArtworkImage
            trackId={currentTrack.id}
            coverArtPath={currentTrack.coverArtPath}
            alt={currentTrack.title}
            className="w-10 h-10 rounded-md object-cover flex-shrink-0"
            fallbackIconSize="sm"
          />
          <div className="flex-1 min-w-0">
            <p
              className="text-sm font-semibold truncate"
              data-testid="floating-now-playing-title"
            >
              {currentTrack.title}
            </p>
            <p
              className="text-xs text-foreground/60 truncate"
              data-testid="floating-now-playing-artist"
            >
              {artist}
            </p>
          </div>
        </button>

        {/* Row 2: progress bar */}
        <div data-testid="floating-progress-bar">
          <ProgressBar />
        </div>

        {/* Row 3: playback controls */}
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

        {/* Row 4: compact inline volume */}
        <div className="flex items-center gap-2">
          <button
            onClick={handleMuteToggle}
            className="text-muted-foreground hover:opacity-80 transition-opacity shrink-0"
            aria-pressed={isMuted}
            data-testid="floating-volume-mute"
          >
            {isMuted || volume === 0
              ? <VolumeX className="w-3.5 h-3.5" />
              : <Volume2 className="w-3.5 h-3.5" />
            }
          </button>
          <div
            ref={volumeTrackRef}
            className="flex-1 h-3 flex items-center cursor-pointer"
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
          <span className="text-[10px] font-mono w-5 text-right shrink-0 text-muted-foreground">
            {Math.round(displayVolume * 100)}
          </span>
        </div>

      </div>
    </div>
  );
}
