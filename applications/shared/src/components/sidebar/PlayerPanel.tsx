import { useState, useCallback, useRef, useEffect } from 'react';
import { usePlayerStore } from '../../stores/player';
import { usePlayerCommands } from '../../contexts/PlayerCommandsContext';
import { usePlaybackHandlers } from '../../hooks/usePlaybackHandlers';
import { NowPlayingPanel, type CurrentTrackInfo } from './NowPlayingPanel';
import { ProgressBar } from '../player/ProgressBar';
import { PlaybackControls } from './PlaybackControls';
import { VolumeControl } from './VolumeControl';
import { debug } from '../../utils/debug';

export interface PlayerPanelProps {
  currentTrack: CurrentTrackInfo | null;
  isPlaying: boolean;
  volume: number;
  canCreatePlaylists: boolean;
  onTrackClick?: () => void;
  onAddToPlaylist?: () => void;
  // Removed: shuffleMode, repeatMode, onShuffleModeChange, onRepeatModeChange
  // These are now owned by usePlaybackHandlers + read from Zustand store directly.
}

export function PlayerPanel({
  currentTrack,
  isPlaying,
  volume,
  canCreatePlaylists,
  onTrackClick,
  onAddToPlaylist,
}: PlayerPanelProps) {
  const commands = usePlayerCommands();
  const handlers = usePlaybackHandlers();
  // Read shuffle/repeat from Zustand — no prop needed
  const shuffleMode = usePlayerStore((s) => s.shuffleMode);
  const repeatMode  = usePlayerStore((s) => s.repeatMode);

  const [isMuted, setIsMuted] = useState(false);
  const [volumeBeforeMute, setVolumeBeforeMute] = useState(volume);
  const debounceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (volume > 0 && !isMuted) {
      setVolumeBeforeMute(volume);
    }
  }, [volume, isMuted]);

  const applyVolumeChange = useCallback(
    (newVolume: number) => {
      const clampedVolume = Math.max(0, Math.min(1, newVolume));
      usePlayerStore.getState().setVolume(clampedVolume);
      if (clampedVolume > 0 && isMuted) setIsMuted(false);
      if (debounceTimerRef.current) clearTimeout(debounceTimerRef.current);
      debounceTimerRef.current = setTimeout(() => {
        commands.setVolume(clampedVolume).catch((error) => {
          debug.error('[PlayerPanel] Set volume failed:', error);
        });
      }, 150);
    },
    [commands, isMuted]
  );

  const handleVolumeChange = (newVolume: number) => applyVolumeChange(newVolume);

  const handleMuteToggle = async () => {
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
    } catch (error) {
      debug.error('[PlayerPanel] Mute toggle failed:', error);
    }
  };

  const handleVolumeWheel = useCallback(
    (e: React.WheelEvent) => {
      const target = e.target as HTMLElement;
      if (target.closest('[data-dropdown-menu]')) return;
      e.preventDefault();
      const delta = e.deltaY > 0 ? -0.05 : 0.05;
      applyVolumeChange(volume + delta);
    },
    [volume, applyVolumeChange]
  );

  return (
    <div className="flex-shrink-0">
      <NowPlayingPanel
        currentTrack={currentTrack}
        isPlaying={isPlaying}
        canCreatePlaylists={canCreatePlaylists}
        onTrackClick={onTrackClick}
        onAddToPlaylist={onAddToPlaylist}
      />

      <div className="px-4 pt-2 pb-4 space-y-3">
        <ProgressBar />

        <PlaybackControls
          isPlaying={isPlaying}
          hasCurrentTrack={!!currentTrack}
          shuffleMode={shuffleMode}
          repeatMode={repeatMode}
          onPlayPause={handlers.onPlayPause}
          onPrevious={handlers.onPrevious}
          onNext={handlers.onNext}
          onShuffleToggle={handlers.onShuffleToggle}
          onRepeatToggle={handlers.onRepeatToggle}
        />

        <VolumeControl
          volume={volume}
          isMuted={isMuted}
          onVolumeChange={handleVolumeChange}
          onMuteToggle={handleMuteToggle}
          onWheel={handleVolumeWheel}
        />
      </div>
    </div>
  );
}
