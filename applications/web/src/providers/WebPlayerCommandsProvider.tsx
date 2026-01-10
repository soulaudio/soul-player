/**
 * Web implementation of PlayerCommands context
 *
 * This provider bridges the shared UI components to the server API.
 * - Registers as a device on mount
 * - Syncs playback state with server
 * - For now, playback happens locally (browser plays audio via Web Audio API)
 * - State is synced to server for multi-device support
 */

import { ReactNode, useEffect, useMemo, useState, useCallback, useRef } from 'react';
import {
  PlayerCommandsProvider,
  usePlayerStore,
  type PlayerContextValue,
  type PlayerCommandsInterface,
  type PlaybackEventsInterface,
  type QueueTrack,
  type PlaybackCapabilities,
} from '@soul-player/shared';
import { apiClient } from '../api/client';

interface DeviceResponse {
  id: string;
  name: string;
  device_type: string;
  is_active: boolean;
}

interface PlaybackStateResponse {
  is_playing: boolean;
  current_track_id: string | null;
  position_ms: number;
  volume: number;
  shuffle_enabled: boolean;
  repeat_mode: 'off' | 'all' | 'one';
  queue: string[] | null;
}

// Simple audio element for playback
class WebAudioPlayer {
  private audio: HTMLAudioElement;
  private onTimeUpdate?: (position: number) => void;
  private onEnded?: () => void;
  private onError?: (error: string) => void;
  private onStateChange?: (playing: boolean) => void;

  constructor() {
    this.audio = new Audio();
    this.audio.addEventListener('timeupdate', () => {
      this.onTimeUpdate?.(this.audio.currentTime);
    });
    this.audio.addEventListener('ended', () => {
      this.onEnded?.();
    });
    this.audio.addEventListener('error', () => {
      this.onError?.('Playback error');
    });
    this.audio.addEventListener('play', () => {
      this.onStateChange?.(true);
    });
    this.audio.addEventListener('pause', () => {
      this.onStateChange?.(false);
    });
  }

  async loadTrack(trackId: string) {
    const token = apiClient.getAccessToken();
    const apiBase = import.meta.env.VITE_API_URL || '/api';
    this.audio.src = `${apiBase}/stream/${trackId}`;
    // Add auth header via fetch is not possible for audio element
    // The server needs to support token in query param or cookie for streaming
    // For now, assume streaming endpoint checks auth or is accessible
  }

  async play() {
    await this.audio.play();
  }

  pause() {
    this.audio.pause();
  }

  seek(seconds: number) {
    this.audio.currentTime = seconds;
  }

  setVolume(volume: number) {
    this.audio.volume = Math.max(0, Math.min(1, volume));
  }

  get currentTime() {
    return this.audio.currentTime;
  }

  get duration() {
    return this.audio.duration || 0;
  }

  get isPlaying() {
    return !this.audio.paused;
  }

  setOnTimeUpdate(callback: (position: number) => void) {
    this.onTimeUpdate = callback;
  }

  setOnEnded(callback: () => void) {
    this.onEnded = callback;
  }

  setOnError(callback: (error: string) => void) {
    this.onError = callback;
  }

  setOnStateChange(callback: (playing: boolean) => void) {
    this.onStateChange = callback;
  }

  destroy() {
    this.audio.pause();
    this.audio.src = '';
  }
}

function getBrowserName(): string {
  const ua = navigator.userAgent;
  if (ua.includes('Chrome')) return 'Chrome';
  if (ua.includes('Firefox')) return 'Firefox';
  if (ua.includes('Safari')) return 'Safari';
  if (ua.includes('Edge')) return 'Edge';
  return 'Web Browser';
}

interface Props {
  children: ReactNode;
}

export function WebPlayerCommandsProvider({ children }: Props) {
  const [deviceId, setDeviceId] = useState<string | null>(null);
  const audioPlayerRef = useRef<WebAudioPlayer | null>(null);
  const queueRef = useRef<QueueTrack[]>([]);
  const currentIndexRef = useRef<number>(0);

  const store = usePlayerStore();
  const eventCallbacks = useRef<{
    stateChange: Set<(isPlaying: boolean) => void>;
    trackChange: Set<(track: unknown) => void>;
    positionUpdate: Set<(position: number) => void>;
    volumeChange: Set<(volume: number) => void>;
    queueUpdate: Set<() => void>;
    error: Set<(error: string) => void>;
  }>({
    stateChange: new Set(),
    trackChange: new Set(),
    positionUpdate: new Set(),
    volumeChange: new Set(),
    queueUpdate: new Set(),
    error: new Set(),
  });

  // Initialize audio player
  useEffect(() => {
    const player = new WebAudioPlayer();
    audioPlayerRef.current = player;

    player.setOnTimeUpdate((position) => {
      const duration = player.duration;
      const progress = duration > 0 ? (position / duration) * 100 : 0;
      store.setProgress(progress);
      store.setDuration(duration);
      eventCallbacks.current.positionUpdate.forEach((cb) => cb(position));
    });

    player.setOnEnded(() => {
      // Auto-advance to next track
      const nextIndex = currentIndexRef.current + 1;
      if (nextIndex < queueRef.current.length) {
        const nextTrack = queueRef.current[nextIndex];
        currentIndexRef.current = nextIndex;
        player.loadTrack(nextTrack.trackId).then(() => player.play());
        updateStoreWithTrack(nextTrack);
      } else {
        store.setIsPlaying(false);
      }
    });

    player.setOnError((error) => {
      eventCallbacks.current.error.forEach((cb) => cb(error));
    });

    player.setOnStateChange((playing) => {
      store.setIsPlaying(playing);
      eventCallbacks.current.stateChange.forEach((cb) => cb(playing));
    });

    return () => {
      player.destroy();
    };
  }, []);

  // Register device on mount
  useEffect(() => {
    const registerDevice = async () => {
      try {
        const device = await apiClient.post<DeviceResponse>('/devices', {
          name: `${getBrowserName()} on ${navigator.platform}`,
          device_type: 'web',
        });
        setDeviceId(device.id);

        // Set as active device
        await apiClient.put(`/devices/${device.id}/activate`);
      } catch (err) {
        console.error('Failed to register device:', err);
      }
    };

    registerDevice();

    return () => {
      if (deviceId) {
        apiClient.delete(`/devices/${deviceId}`).catch(() => {});
      }
    };
  }, []);

  // Sync initial playback state from server
  useEffect(() => {
    const syncState = async () => {
      try {
        const state = await apiClient.get<PlaybackStateResponse>('/playback');
        store.setVolume(state.volume / 100);
        store.setShuffleEnabled(state.shuffle_enabled);
        store.setRepeatMode(state.repeat_mode);
        if (audioPlayerRef.current) {
          audioPlayerRef.current.setVolume(state.volume / 100);
        }
      } catch (err) {
        console.error('Failed to sync playback state:', err);
      }
    };

    syncState();
  }, []);

  const updateStoreWithTrack = useCallback((track: QueueTrack) => {
    store.setCurrentTrack({
      id: parseInt(track.trackId) || 0,
      title: track.title,
      artist: track.artist,
      album: track.album || undefined,
      duration: track.durationSeconds || 0,
      trackNumber: track.trackNumber || undefined,
      coverArtPath: track.coverArtPath,
    });
    store.setQueue(queueRef.current.map((t) => ({
      id: parseInt(t.trackId) || 0,
      title: t.title,
      artist: t.artist,
      album: t.album || undefined,
      duration: t.durationSeconds || 0,
    })));
    eventCallbacks.current.trackChange.forEach((cb) => cb(track));
    eventCallbacks.current.queueUpdate.forEach((cb) => cb());
  }, [store]);

  const value = useMemo<PlayerContextValue>(() => {
    const commands: PlayerCommandsInterface = {
      async playTrack(trackId) {
        const track = queueRef.current.find((t) => t.trackId === String(trackId));
        if (track && audioPlayerRef.current) {
          const index = queueRef.current.indexOf(track);
          currentIndexRef.current = index;
          await audioPlayerRef.current.loadTrack(String(trackId));
          await audioPlayerRef.current.play();
          updateStoreWithTrack(track);
        }
        // Sync to server
        await apiClient.post('/playback/play', { track_id: String(trackId) });
      },

      async pausePlayback() {
        audioPlayerRef.current?.pause();
        await apiClient.post('/playback/pause');
      },

      async resumePlayback() {
        await audioPlayerRef.current?.play();
        await apiClient.post('/playback/play');
      },

      async stopPlayback() {
        audioPlayerRef.current?.pause();
        store.setIsPlaying(false);
        store.setCurrentTrack(null);
        await apiClient.post('/playback/pause');
      },

      async skipNext() {
        const nextIndex = currentIndexRef.current + 1;
        if (nextIndex < queueRef.current.length && audioPlayerRef.current) {
          const nextTrack = queueRef.current[nextIndex];
          currentIndexRef.current = nextIndex;
          await audioPlayerRef.current.loadTrack(nextTrack.trackId);
          await audioPlayerRef.current.play();
          updateStoreWithTrack(nextTrack);
          await apiClient.post('/playback/skip/next');
        }
      },

      async skipPrevious() {
        const prevIndex = currentIndexRef.current - 1;
        if (prevIndex >= 0 && audioPlayerRef.current) {
          const prevTrack = queueRef.current[prevIndex];
          currentIndexRef.current = prevIndex;
          await audioPlayerRef.current.loadTrack(prevTrack.trackId);
          await audioPlayerRef.current.play();
          updateStoreWithTrack(prevTrack);
          await apiClient.post('/playback/skip/previous');
        }
      },

      async seek(position) {
        audioPlayerRef.current?.seek(position);
        await apiClient.post('/playback/seek', { position_ms: Math.floor(position * 1000) });
      },

      async setVolume(volume) {
        audioPlayerRef.current?.setVolume(volume);
        store.setVolume(volume);
        eventCallbacks.current.volumeChange.forEach((cb) => cb(volume));
        await apiClient.post('/playback/volume', { volume: Math.floor(volume * 100) });
      },

      async setShuffle(enabled) {
        store.setShuffleEnabled(enabled);
        await apiClient.put('/playback', { shuffle_enabled: enabled });
      },

      async setRepeatMode(mode) {
        store.setRepeatMode(mode);
        await apiClient.put('/playback', { repeat_mode: mode });
      },

      async getPlaybackCapabilities(): Promise<PlaybackCapabilities> {
        return {
          hasNext: currentIndexRef.current < queueRef.current.length - 1,
          hasPrevious: currentIndexRef.current > 0,
        };
      },

      async getQueue(): Promise<QueueTrack[]> {
        return queueRef.current;
      },

      async playQueue(queue, startIndex = 0) {
        queueRef.current = queue;
        currentIndexRef.current = startIndex;

        if (queue.length > 0 && audioPlayerRef.current) {
          const track = queue[startIndex];
          await audioPlayerRef.current.loadTrack(track.trackId);
          await audioPlayerRef.current.play();
          updateStoreWithTrack(track);

          // Sync queue to server
          await apiClient.post('/playback/play', {
            queue: queue.map((t) => t.trackId),
            start_index: startIndex,
          });
        }
      },

      async skipToQueueIndex(index) {
        if (index >= 0 && index < queueRef.current.length && audioPlayerRef.current) {
          const track = queueRef.current[index];
          currentIndexRef.current = index;
          await audioPlayerRef.current.loadTrack(track.trackId);
          await audioPlayerRef.current.play();
          updateStoreWithTrack(track);
        }
      },

      async getAllSources() {
        return [
          {
            id: 1,
            name: 'Server Library',
            sourceType: 'server',
            isActive: true,
            isOnline: true,
          },
        ];
      },
    };

    const events: PlaybackEventsInterface = {
      onStateChange(callback) {
        eventCallbacks.current.stateChange.add(callback);
        return () => eventCallbacks.current.stateChange.delete(callback);
      },

      onTrackChange(callback) {
        eventCallbacks.current.trackChange.add(callback);
        return () => eventCallbacks.current.trackChange.delete(callback);
      },

      onPositionUpdate(callback) {
        eventCallbacks.current.positionUpdate.add(callback);
        return () => eventCallbacks.current.positionUpdate.delete(callback);
      },

      onVolumeChange(callback) {
        eventCallbacks.current.volumeChange.add(callback);
        return () => eventCallbacks.current.volumeChange.delete(callback);
      },

      onQueueUpdate(callback) {
        eventCallbacks.current.queueUpdate.add(callback);
        return () => eventCallbacks.current.queueUpdate.delete(callback);
      },

      onError(callback) {
        eventCallbacks.current.error.add(callback);
        return () => eventCallbacks.current.error.delete(callback);
      },
    };

    return { commands, events };
  }, [updateStoreWithTrack, store]);

  return <PlayerCommandsProvider value={value}>{children}</PlayerCommandsProvider>;
}
