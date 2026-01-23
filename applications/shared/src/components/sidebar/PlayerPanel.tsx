'use client';

import { useState, useCallback, useRef, useEffect } from 'react';
import { usePlayerStore } from '../../stores/player';
import { usePlayerCommands } from '../../contexts/PlayerCommandsContext';
import { NowPlayingPanel, type CurrentTrackInfo } from './NowPlayingPanel';
import { ProgressBar } from './ProgressBar';
import { PlaybackControls, type ShuffleMode, type RepeatMode } from './PlaybackControls';
import { VolumeControl } from './VolumeControl';
import { DeviceSelector, type AudioDevice, type AudioBackend } from './DeviceSelector';
import { debug } from '../../utils/debug';

export interface PlayerPanelProps {
  currentTrack: CurrentTrackInfo | null;
  isPlaying: boolean;
  progress: number;
  duration: number;
  volume: number;
  shuffleMode: ShuffleMode;
  repeatMode: RepeatMode;
  hasRealDevices: boolean;
  canCreatePlaylists: boolean;
  onTrackClick?: () => void;
  onAddToPlaylist?: () => void;
  onShuffleModeChange: (mode: ShuffleMode) => void;
  onRepeatModeChange: (mode: RepeatMode) => void;
}

export function PlayerPanel({
  currentTrack,
  isPlaying,
  progress,
  duration,
  volume,
  shuffleMode,
  repeatMode,
  hasRealDevices,
  canCreatePlaylists,
  onTrackClick,
  onAddToPlaylist,
  onShuffleModeChange,
  onRepeatModeChange,
}: PlayerPanelProps) {
  const commands = usePlayerCommands();
  const [isMuted, setIsMuted] = useState(false);
  const [volumeBeforeMute, setVolumeBeforeMute] = useState(volume);
  const debounceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Device selector state
  const [currentDevice, setCurrentDevice] = useState<AudioDevice | null>(null);
  const [backends, setBackends] = useState<AudioBackend[]>([]);
  const [devices, setDevices] = useState<Map<string, AudioDevice[]>>(new Map());
  const [isLoadingDevices, setIsLoadingDevices] = useState(false);

  useEffect(() => {
    if (volume > 0 && !isMuted) {
      setVolumeBeforeMute(volume);
    }
  }, [volume, isMuted]);

  useEffect(() => {
    if (!hasRealDevices) {
      setCurrentDevice({
        name: 'System Default',
        backend: 'System',
        isDefault: true,
        sampleRate: 48000,
        channels: 2,
        isRunning: true,
      });
    } else {
      loadCurrentDevice();
    }
  }, [hasRealDevices]);

  const loadCurrentDevice = async () => {
    try {
      if (!commands?.getCurrentAudioDevice) return;
      const device = await commands.getCurrentAudioDevice();
      setCurrentDevice(device);
    } catch (error) {
      debug.error('[PlayerPanel] Failed to load current device:', error);
    }
  };

  const loadDevices = async () => {
    if (!hasRealDevices) {
      const deviceMap = new Map<string, AudioDevice[]>();
      deviceMap.set('System', [
        {
          name: 'System Default',
          backend: 'System',
          isDefault: true,
          sampleRate: 48000,
          channels: 2,
          isRunning: true,
        },
      ]);
      setDevices(deviceMap);
      return;
    }

    if (isLoadingDevices) return;
    setIsLoadingDevices(true);

    try {
      if (commands?.getAudioBackends) {
        const backendList = await commands.getAudioBackends();
        setBackends(backendList);

        const deviceMap = new Map<string, AudioDevice[]>();
        for (const backend of backendList) {
          if (backend.available && commands?.getAudioDevices) {
            try {
              const backendDevices = await commands.getAudioDevices(backend.backend);
              deviceMap.set(backend.backend, backendDevices);
            } catch (error) {
              debug.error(`[PlayerPanel] Failed to load devices for ${backend.backend}:`, error);
            }
          }
        }
        setDevices(deviceMap);
      }
    } catch (error) {
      debug.error('[PlayerPanel] Failed to load devices:', error);
    } finally {
      setIsLoadingDevices(false);
    }
  };

  const switchDevice = async (backend: string, deviceName: string) => {
    if (!hasRealDevices) {
      if (backend === 'System') {
        setCurrentDevice({
          name: 'System Default',
          backend: 'System',
          isDefault: true,
          sampleRate: 48000,
          channels: 2,
          isRunning: true,
        });
      }
      return;
    }

    try {
      if (!commands?.setAudioDevice) return;
      await commands.setAudioDevice(backend, deviceName);
      await loadCurrentDevice();
    } catch (error) {
      debug.error('[PlayerPanel] Failed to switch device:', error);
    }
  };

  const handlePlayPause = useCallback(async () => {
    try {
      if (isPlaying) {
        await commands.pausePlayback();
      } else {
        await commands.resumePlayback();
      }
    } catch (error) {
      debug.error('[PlayerPanel] Failed to toggle playback:', error);
    }
  }, [isPlaying, commands]);

  const handlePrevious = useCallback(async () => {
    try {
      await commands.skipPrevious();
    } catch (error) {
      debug.error('[PlayerPanel] Failed to skip previous:', error);
    }
  }, [commands]);

  const handleNext = useCallback(async () => {
    try {
      await commands.skipNext();
    } catch (error) {
      debug.error('[PlayerPanel] Failed to skip next:', error);
    }
  }, [commands]);

  const handleSeek = useCallback(
    async (e: React.MouseEvent<HTMLDivElement>) => {
      if (!duration) return;
      const rect = e.currentTarget.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const percentage = x / rect.width;
      const newPosition = percentage * duration;
      try {
        await commands.seek(newPosition);
      } catch (error) {
        debug.error('[PlayerPanel] Failed to seek:', error);
      }
    },
    [duration, commands]
  );

  const handleShuffleToggle = async () => {
    debug.log('[PlayerPanel] Current shuffle mode:', shuffleMode);
    try {
      const newMode = await commands.cycleShuffle();
      debug.log('[PlayerPanel] New shuffle mode from backend:', newMode);
      onShuffleModeChange(newMode);
    } catch (error) {
      debug.error('[PlayerPanel] Cycle shuffle failed:', error);
    }
  };

  const handleRepeatToggle = async () => {
    debug.log('[PlayerPanel] Current repeat mode:', repeatMode);
    const nextMode: RepeatMode =
      repeatMode === 'off' ? 'all' : repeatMode === 'all' ? 'one' : 'off';
    debug.log('[PlayerPanel] Cycling to:', nextMode);
    onRepeatModeChange(nextMode);
    try {
      await commands.setRepeatMode(nextMode);
      debug.log('[PlayerPanel] Repeat mode set successfully');
    } catch (error) {
      debug.error('[PlayerPanel] Set repeat mode failed:', error);
      const prevMode: RepeatMode =
        nextMode === 'off' ? 'one' : nextMode === 'all' ? 'off' : 'all';
      onRepeatModeChange(prevMode);
    }
  };

  const applyVolumeChange = useCallback(
    (newVolume: number) => {
      const clampedVolume = Math.max(0, Math.min(1, newVolume));
      usePlayerStore.getState().setVolume(clampedVolume);

      if (clampedVolume > 0 && isMuted) {
        setIsMuted(false);
      }

      if (debounceTimerRef.current) {
        clearTimeout(debounceTimerRef.current);
      }

      debounceTimerRef.current = setTimeout(() => {
        commands.setVolume(clampedVolume).catch((error) => {
          debug.error('[PlayerPanel] Set volume failed:', error);
        });
      }, 150);
    },
    [commands, isMuted]
  );

  const handleVolumeChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    applyVolumeChange(parseFloat(e.target.value));
  };

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
      if (target.closest('[data-dropdown-menu]')) {
        return;
      }

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

      <div className="px-4 space-y-3">
        <ProgressBar
          progress={progress}
          duration={duration}
          hasCurrentTrack={!!currentTrack}
          onSeek={handleSeek}
        />

        <PlaybackControls
          isPlaying={isPlaying}
          hasCurrentTrack={!!currentTrack}
          shuffleMode={shuffleMode}
          repeatMode={repeatMode}
          onPlayPause={handlePlayPause}
          onPrevious={handlePrevious}
          onNext={handleNext}
          onShuffleToggle={handleShuffleToggle}
          onRepeatToggle={handleRepeatToggle}
        />

        <div className="flex items-center gap-2">
          <VolumeControl
            volume={volume}
            isMuted={isMuted}
            onVolumeChange={handleVolumeChange}
            onMuteToggle={handleMuteToggle}
            onWheel={handleVolumeWheel}
          />
          <DeviceSelector
            currentDevice={currentDevice}
            backends={backends}
            devices={devices}
            isLoadingDevices={isLoadingDevices}
            hasRealDevices={hasRealDevices}
            onLoadDevices={loadDevices}
            onSwitchDevice={switchDevice}
          />
        </div>
      </div>
    </div>
  );
}
