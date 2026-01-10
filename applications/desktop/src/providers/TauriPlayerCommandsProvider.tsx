/**
 * Tauri implementation of PlayerCommands context
 * Bridges desktop Tauri invoke() calls to shared PlayerCommands interface
 * Also handles event-to-store updates
 */

import { ReactNode, useMemo, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
  PlayerCommandsProvider,
  usePlayerStore,
  type PlayerContextValue,
  type PlayerCommandsInterface,
  type PlaybackEventsInterface,
  type PlaybackCapabilities,
} from '@soul-player/shared';
import { shouldIgnorePositionUpdates } from '@soul-player/shared';

export function TauriPlayerCommandsProvider({ children }: { children: ReactNode }) {
  // Set up event listeners to update store (similar to old usePlaybackEvents hook)
  useEffect(() => {
    console.log('[TauriPlayerCommandsProvider] Setting up playback event listeners');

    // Listen for playback state changes
    const unlistenStateChanged = listen<string>('playback:state-changed', (event) => {
      const isPlaying = event.payload === 'Playing';
      usePlayerStore.setState({ isPlaying });
    });

    // Listen for position updates
    const unlistenPositionUpdated = listen<number>('playback:position-updated', (event) => {
      if (shouldIgnorePositionUpdates()) return;

      const positionInSeconds = event.payload;
      const { duration } = usePlayerStore.getState();
      const progressPercentage = duration > 0 ? Math.min(100, (positionInSeconds / duration) * 100) : 0;
      usePlayerStore.setState({ progress: progressPercentage });
    });

    // Listen for track changes
    const unlistenTrackChanged = listen<any>('playback:track-changed', (event) => {
      const track = event.payload;
      console.log('[TauriPlayerCommandsProvider] Track changed:', track);
      console.log('[TauriPlayerCommandsProvider] coverArtPath:', track?.coverArtPath);
      usePlayerStore.setState({
        currentTrack: track,
        duration: track?.duration || 0,
        progress: 0
      });
    });

    // Listen for volume changes (0-100 from backend)
    const unlistenVolumeChanged = listen<number>('playback:volume-changed', (event) => {
      usePlayerStore.setState({ volume: event.payload / 100 }); // Convert to 0-1
    });

    // Listen for queue updates
    const unlistenQueueUpdated = listen('playback:queue-updated', () => {
      // Queue updates handled by components
    });

    // Listen for errors
    const unlistenError = listen<string>('playback:error', (event) => {
      console.error('[TauriPlayerCommandsProvider] Playback error:', event.payload);
    });

    // Cleanup
    return () => {
      console.log('[TauriPlayerCommandsProvider] Cleaning up event listeners');
      unlistenStateChanged.then((fn) => fn());
      unlistenPositionUpdated.then((fn) => fn());
      unlistenTrackChanged.then((fn) => fn());
      unlistenVolumeChanged.then((fn) => fn());
      unlistenQueueUpdated.then((fn) => fn());
      unlistenError.then((fn) => fn());
    };
  }, []);

  const value = useMemo<PlayerContextValue>(() => {
    // Commands implementation using Tauri
    const commands: PlayerCommandsInterface = {
      async playTrack(trackId: string | number) {
        await invoke('play_track', { trackId: Number(trackId) });
      },

      async pausePlayback() {
        await invoke('pause_playback');
      },

      async resumePlayback() {
        await invoke('resume_playback');
      },

      async stopPlayback() {
        await invoke('stop_playback');
      },

      async skipNext() {
        await invoke('next_track');
      },

      async skipPrevious() {
        await invoke('previous_track');
      },

      async seek(position: number) {
        await invoke('seek_to', { position });
      },

      async setVolume(volume: number) {
        // Desktop backend expects 0-100, but shared interface uses 0-1
        await invoke('set_volume', { volume: Math.round(volume * 100) });
      },

      async setShuffle(enabled: boolean) {
        await invoke('set_shuffle', { enabled });
      },

      async setRepeatMode(mode: 'off' | 'all' | 'one') {
        await invoke('set_repeat_mode', { mode });
      },

      async getPlaybackCapabilities(): Promise<PlaybackCapabilities> {
        return await invoke<PlaybackCapabilities>('get_playback_capabilities');
      },

      async getQueue() {
        return await invoke('get_queue');
      },

      async playQueue(queue, startIndex = 0) {
        // Backend handles: stop current playback, load new context, start playing
        // This ensures clicking play replaces queue (Spotify behavior)
        await invoke('play_queue', { queue, startIndex });
      },

      async skipToQueueIndex(index: number) {
        await invoke('skip_to_queue_index', { index });
      },

      async getAllSources() {
        return await invoke('get_all_sources');
      },

      // Audio device management (Desktop only)
      async getCurrentAudioDevice() {
        return await invoke('get_current_audio_device');
      },

      async getAudioBackends() {
        return await invoke('get_audio_backends');
      },

      async getAudioDevices(backend: string) {
        return await invoke('get_audio_devices', { backendStr: backend });
      },

      async setAudioDevice(backend: string, deviceName: string) {
        await invoke('set_audio_device', { backendStr: backend, deviceName });
      },
    };

    // Events implementation using Tauri event listeners
    const events: PlaybackEventsInterface = {
      onStateChange(callback) {
        const unlisten = listen<boolean>('playback:state-changed', (event) => {
          callback(event.payload);
        });
        return () => {
          unlisten.then((fn) => fn());
        };
      },

      onTrackChange(callback) {
        const unlisten = listen('playback:track-changed', (event) => {
          callback(event.payload);
        });
        return () => {
          unlisten.then((fn) => fn());
        };
      },

      onPositionUpdate(callback) {
        const unlisten = listen<number>('playback:position-updated', (event) => {
          callback(event.payload);
        });
        return () => {
          unlisten.then((fn) => fn());
        };
      },

      onVolumeChange(callback) {
        const unlisten = listen<number>('playback:volume-changed', (event) => {
          // Backend sends 0-100, convert to 0-1
          callback(event.payload);
        });
        return () => {
          unlisten.then((fn) => fn());
        };
      },

      onQueueUpdate(callback) {
        const unlisten = listen('playback:queue-updated', () => {
          callback();
        });
        return () => {
          unlisten.then((fn) => fn());
        };
      },

      onError(callback) {
        const unlisten = listen<string>('playback:error', (event) => {
          callback(event.payload);
        });
        return () => {
          unlisten.then((fn) => fn());
        };
      },
    };

    return { commands, events };
  }, []);

  return <PlayerCommandsProvider value={value}>{children}</PlayerCommandsProvider>;
}
