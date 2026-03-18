import { useState, useCallback, useRef, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useSidebarState } from '../contexts/SidebarStateContext';
import { useCurrentTrack, useIsPlaying, useVolume, usePlayerStore } from '../stores/player';
import { usePlayerCommands } from '../contexts/PlayerCommandsContext';
import { ArtworkImage } from './ArtworkImage';
import { ProgressBar } from './player/ProgressBar';
import { PlaybackControls } from './sidebar/PlaybackControls';
import { usePlaybackHandlers } from '../hooks/usePlaybackHandlers';
import { useSeekBar } from '../hooks/useSeekBar';
import { useInterpolatedProgress } from '../hooks/useInterpolatedProgress';
import { debug } from '../utils/debug';
import { Volume2, VolumeX, Play, Pause, SkipForward, Heart } from 'lucide-react';

/**
 * NowPlayingFloating — bottom player bar shown in two modes:
 *
 * Mobile (< 640px):  compact mini player — art + title/artist + play/pause + next
 *                    visible on both sidebar and content views
 *                    tapping opens a "now playing" view
 *
 * Desktop collapsed: full 3-column bar — [art·info] [controls·progress] [volume]
 */
export function NowPlayingFloating() {
  const navigate = useNavigate();
  const { isCollapsed, isMobile, mobileShowContent } = useSidebarState();
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

  if (!currentTrack) return null;

  const title = currentTrack.title || 'Unknown Track';
  const artist = currentTrack.artist || 'Unknown Artist';

  // ── Mobile: compact mini player (only on content view, not sidebar) ─────
  if (isMobile) {
    if (!mobileShowContent) return null;

    return (
      <MobileMiniPlayer
        currentTrack={currentTrack}
        title={title}
        artist={artist}
        isPlaying={isPlaying}
        onPlayPause={handlers.onPlayPause}
        onNext={handlers.onNext}
        onOpenNowPlaying={() => navigate('/now-playing-todo')}
      />
    );
  }

  // ── Desktop: only show when sidebar is collapsed ─────────────────────────
  if (!isCollapsed) return null;

  debug.log('[NowPlayingFloating] rendering with track:', {
    id: currentTrack.id,
    title: currentTrack.title,
    artist: currentTrack.artist,
  });

  const displayVolume = isMuted ? 0 : volume;

  return (
    <div
      className="fixed bottom-3 left-3 right-3 z-50 mx-auto max-w-[960px] group/bar"
      data-testid="now-playing-floating"
    >
      <div className="bg-card/85 backdrop-blur-md border border-border rounded-2xl shadow-lg opacity-[0.97] group-hover/bar:opacity-100 transition-opacity">
      <div className="grid grid-cols-[minmax(160px,1fr)_minmax(auto,640px)_minmax(160px,1fr)] items-center h-[72px] px-5 gap-4">

        {/* Left: artwork + track info + heart — clickable container opens now playing */}
        <div
          className="flex items-center gap-1.5 min-w-0 max-w-[240px] rounded-lg px-1.5 py-1 -mx-1.5 -my-1 hover:bg-foreground/[0.06] transition-colors cursor-pointer"
          onClick={() => navigate('/now-playing-todo')}
          data-testid="floating-track-info"
        >
          <div className="w-10 h-10 rounded-md overflow-hidden flex-shrink-0">
            <ArtworkImage
              trackId={currentTrack.id}
              coverArtPath={currentTrack.coverArtPath}
              alt={title}
              className="w-full h-full object-cover"
              fallbackClassName="w-full h-full flex items-center justify-center bg-muted"
              fallbackIconSize="sm"
            />
          </div>
          <div className="flex flex-col min-w-0 w-[180px] text-left">
            <span
              className="text-sm font-semibold truncate text-foreground hover:underline"
              data-testid="floating-now-playing-title"
            >
              {title}
            </span>
            <span
              className="text-xs text-muted-foreground truncate hover:underline"
              data-testid="floating-now-playing-artist"
            >
              {artist}
            </span>
          </div>
          <button
            onClick={(e) => { e.stopPropagation(); }}
            className="ml-auto p-1.5 text-muted-foreground hover:text-foreground transition-colors flex-shrink-0"
          >
            <Heart className="w-4 h-4" />
          </button>
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
    </div>
  );
}

/** Separate component so hooks (useSeekBar, useInterpolatedProgress) are called unconditionally */
function MobileMiniPlayer({
  currentTrack,
  title,
  artist,
  isPlaying,
  onPlayPause,
  onNext,
  onOpenNowPlaying,
}: {
  currentTrack: { id: number; coverArtPath?: string };
  title: string;
  artist: string;
  isPlaying: boolean;
  onPlayPause: () => void;
  onNext: () => void;
  onOpenNowPlaying: () => void;
}) {
  const { progress: seekProgress, duration } = useInterpolatedProgress();
  const { handleSeek } = useSeekBar();
  const seekBarRef = useRef<HTMLDivElement>(null);
  const [isDragging, setIsDragging] = useState(false);
  const [dragProgress, setDragProgress] = useState(0);

  const displayProgress = isDragging ? dragProgress : seekProgress;

  const calcFromClient = useCallback((clientX: number) => {
    if (!seekBarRef.current) return 0;
    const rect = seekBarRef.current.getBoundingClientRect();
    return Math.max(0, Math.min(100, ((clientX - rect.left) / rect.width) * 100));
  }, []);

  // Mouse drag
  const onMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsDragging(true);
    setDragProgress(calcFromClient(e.clientX));
  }, [calcFromClient]);

  useEffect(() => {
    if (!isDragging) return;
    const onMove = (e: MouseEvent) => setDragProgress(calcFromClient(e.clientX));
    const onUp = (e: MouseEvent) => {
      const pct = calcFromClient(e.clientX);
      handleSeek(Math.min((pct / 100) * duration, Math.max(0, duration - 0.1)));
      setIsDragging(false);
    };
    document.addEventListener('mousemove', onMove);
    document.addEventListener('mouseup', onUp);
    return () => { document.removeEventListener('mousemove', onMove); document.removeEventListener('mouseup', onUp); };
  }, [isDragging, calcFromClient, duration, handleSeek]);

  // Touch drag
  const onTouchStart = useCallback((e: React.TouchEvent) => {
    setIsDragging(true);
    setDragProgress(calcFromClient(e.touches[0].clientX));
  }, [calcFromClient]);

  const onTouchMove = useCallback((e: React.TouchEvent) => {
    if (isDragging) setDragProgress(calcFromClient(e.touches[0].clientX));
  }, [isDragging, calcFromClient]);

  const onTouchEnd = useCallback((e: React.TouchEvent) => {
    if (!isDragging) return;
    const touch = e.changedTouches[0];
    const pct = calcFromClient(touch.clientX);
    handleSeek(Math.min((pct / 100) * duration, Math.max(0, duration - 0.1)));
    setIsDragging(false);
  }, [isDragging, calcFromClient, duration, handleSeek]);

  // Click to seek
  const onClick = useCallback((e: React.MouseEvent) => {
    if (isDragging) return;
    const pct = calcFromClient(e.clientX);
    handleSeek(Math.min((pct / 100) * duration, Math.max(0, duration - 0.1)));
  }, [isDragging, calcFromClient, duration, handleSeek]);

  return (
    <div
      className="fixed bottom-2 left-2 right-2 z-50 mx-auto max-w-[480px]"
      data-testid="now-playing-floating"
    >
      <div className="bg-card/85 backdrop-blur-md border border-border rounded-xl overflow-hidden shadow-lg opacity-[0.97]">
        {/* Seekable progress bar — top */}
        <div
          ref={seekBarRef}
          className="w-full h-2 flex items-start cursor-pointer touch-none"
          onMouseDown={onMouseDown}
          onClick={onClick}
          onTouchStart={onTouchStart}
          onTouchMove={onTouchMove}
          onTouchEnd={onTouchEnd}
          data-testid="mobile-seek-bar"
        >
          <div className="w-full h-[3px] bg-muted">
            <div
              className="h-full bg-primary"
              style={{ width: `${Math.max(0, Math.min(100, displayProgress))}%` }}
            />
          </div>
        </div>
        {/* Mini player content */}
        <div className="flex items-center h-[52px] px-3 gap-2.5">
          {/* Artwork */}
          <div
            className="w-10 h-10 rounded overflow-hidden flex-shrink-0 cursor-pointer"
            onClick={onOpenNowPlaying}
          >
            <ArtworkImage
              trackId={currentTrack.id}
              coverArtPath={currentTrack.coverArtPath}
              alt={title}
              className="w-full h-full object-cover"
              fallbackClassName="w-full h-full flex items-center justify-center bg-muted"
              fallbackIconSize="sm"
            />
          </div>
          {/* Track info */}
          <button
            className="flex flex-col justify-center min-w-0 flex-1 text-left"
            onClick={onOpenNowPlaying}
          >
            <span className="text-[13px] font-medium leading-tight truncate text-foreground">{title}</span>
            <span className="text-[11px] leading-tight truncate text-muted-foreground">{artist}</span>
          </button>
          {/* Heart + Playback controls */}
          <div className="flex items-center gap-0.5 shrink-0">
            <button
              onClick={(e) => { e.stopPropagation(); }}
              className="w-8 h-8 flex items-center justify-center text-muted-foreground hover:text-foreground transition-colors"
            >
              <Heart className="w-4 h-4" />
            </button>
            <button
              onClick={onPlayPause}
              className="w-10 h-10 flex items-center justify-center text-foreground"
              data-testid="mobile-play-pause"
            >
              {isPlaying
                ? <Pause className="w-[22px] h-[22px]" />
                : <Play className="w-[22px] h-[22px] ml-0.5" />
              }
            </button>
            <button
              onClick={onNext}
              className="w-9 h-9 flex items-center justify-center text-muted-foreground"
              data-testid="mobile-next"
            >
              <SkipForward className="w-[18px] h-[18px]" />
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
