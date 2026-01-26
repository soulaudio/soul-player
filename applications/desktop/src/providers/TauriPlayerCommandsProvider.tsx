/**
 * Tauri implementation of PlayerCommands context
 * Bridges desktop Tauri invoke() calls to shared PlayerCommands interface
 * Also handles event-to-store updates and keyboard shortcuts
 */

import { ReactNode, useMemo, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
  PlayerCommandsProvider,
  usePlayerStore,
  usePlaybackSession,
  type PlayerContextValue,
  type PlayerCommandsInterface,
  type PlaybackEventsInterface,
  type PlaybackCapabilities,
} from '@soul-player/shared';
import { shouldIgnorePositionUpdates } from '@soul-player/shared';
import { useKeyboardShortcuts } from '../hooks/useKeyboardShortcuts';

// Separate component to initialize keyboard shortcuts AFTER context is provided
function KeyboardShortcutsInitializer() {
  useKeyboardShortcuts();
  return null;
}

export function TauriPlayerCommandsProvider({ children }: { children: ReactNode }) {
  const { updateSession } = usePlaybackSession();

  // Set up event listeners to update store (similar to old usePlaybackEvents hook)
  useEffect(() => {
    // Store unlisten functions for cleanup
    const unlistenFunctions: (() => void)[] = [];
    let isMounted = true;

    // Sync initial state from backend on mount (non-blocking)
    // Deferred to allow React tree to render first, then update state
    const syncInitialState = async () => {
      // Defer state sync to next tick to avoid blocking initial render
      await new Promise(resolve => setTimeout(resolve, 0));

      try {
        const [state, shuffleMode, repeatMode] = await Promise.all([
          invoke<string>('get_playback_state'),
          invoke<string>('get_shuffle'),
          invoke<string>('get_repeat')
        ]);

        // Only update if component is still mounted
        if (!isMounted) return;

        const isPlaying = state === 'Playing';
        usePlayerStore.setState({
          isPlaying,
          shuffleMode: shuffleMode as 'off' | 'random' | 'smart',
          repeatMode: repeatMode as 'off' | 'all' | 'one'
        });
      } catch (error) {
        console.error('[TauriPlayerCommandsProvider] Failed to sync initial state:', error);
      }
    };

    const setupListeners = async () => {
      try {
        // Listen for playback state changes
        const unlistenStateChanged = await listen<string>('playback:state-changed', (event) => {
          const isPlaying = event.payload === 'Playing';
          usePlayerStore.setState({ isPlaying });
          // Update session state
          updateSession({ isPlaying });
        });
        unlistenFunctions.push(unlistenStateChanged);

        // Listen for position updates
        const unlistenPositionUpdated = await listen<number>('playback:position-updated', (event) => {
          if (shouldIgnorePositionUpdates()) return;

          const positionInSeconds = event.payload;
          const { duration } = usePlayerStore.getState();
          const progressPercentage = duration > 0 ? Math.min(100, (positionInSeconds / duration) * 100) : 0;
          usePlayerStore.setState({ progress: progressPercentage });
        });
        unlistenFunctions.push(unlistenPositionUpdated);

        // Listen for track changes
        const unlistenTrackChanged = await listen<{ id: string; title: string; artist: string; album: string; filePath: string; duration: number; addedAt: string; coverArtPath?: string }>('playback:track-changed', async (event) => {
          const trackPayload = event.payload;
          // Only update if track is valid - don't clear current track on null/undefined
          // (e.g., when skipPrevious is called at the start of queue)
          if (trackPayload && trackPayload.id) {
            // Convert id from string to number to match Track type
            const track = {
              ...trackPayload,
              id: parseInt(trackPayload.id, 10),
            };
            usePlayerStore.setState({
              currentTrack: track,
              duration: track.duration || 0,
              progress: 0
            });

            // Update session with current track and fetch latest context from backend
            try {
              const context = await invoke<{ contextType: string; contextId: string; contextName: string; contextArtworkPath: string | null } | null>('get_current_playback_context');
              if (context) {
                updateSession({
                  currentTrack: track,
                  contextType: context.contextType as 'album' | 'artist' | 'playlist',
                  contextId: context.contextId,
                  contextName: context.contextName,
                  contextArtworkPath: context.contextArtworkPath,
                });
              } else {
                updateSession({ currentTrack: track });
              }
            } catch (error) {
              console.error('[TauriPlayerCommandsProvider] Failed to get context:', error);
              updateSession({ currentTrack: track });
            }
          }
        });
        unlistenFunctions.push(unlistenTrackChanged);

        // Listen for volume changes (0-100 from backend)
        const unlistenVolumeChanged = await listen<number>('playback:volume-changed', (event) => {
          usePlayerStore.setState({ volume: event.payload / 100 }); // Convert to 0-1
        });
        unlistenFunctions.push(unlistenVolumeChanged);

        // Listen for queue updates (shuffle changes emit this event)
        const unlistenQueueUpdated = await listen('playback:queue-updated', async () => {
          // Query and update shuffle mode when queue changes
          try {
            const shuffleMode = await invoke<string>('get_shuffle');
            usePlayerStore.setState({ shuffleMode: shuffleMode as 'off' | 'random' | 'smart' });
          } catch (error) {
            console.error('[TauriPlayerCommandsProvider] Failed to get shuffle mode:', error);
          }
        });
        unlistenFunctions.push(unlistenQueueUpdated);

        // Listen for errors
        const unlistenError = await listen<string>('playback:error', (event) => {
          console.error('[TauriPlayerCommandsProvider] Playback error:', event.payload);
        });
        unlistenFunctions.push(unlistenError);
      } catch (error) {
        console.error('[TauriPlayerCommandsProvider] Failed to set up event listeners:', error);
      }
    };

    // Initialize listeners and state in parallel
    // Use proper promise handling to avoid loading cursor issues on macOS
    Promise.all([setupListeners(), syncInitialState()])
      .catch((error) => {
        console.error('[TauriPlayerCommandsProvider] Initialization failed:', error);
      });

    // Cleanup function
    return () => {
      isMounted = false;
      unlistenFunctions.forEach(fn => fn());
    };
  }, [updateSession]);

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

      async setShuffle(mode: 'off' | 'random' | 'smart') {
        await invoke('set_shuffle', { mode });
      },

      async cycleShuffle() {
        const newMode = await invoke<string>('cycle_shuffle');
        return newMode as 'off' | 'random' | 'smart';
      },

      async getShuffle() {
        const mode = await invoke<string>('get_shuffle');
        return mode as 'off' | 'random' | 'smart';
      },

      async setRepeatMode(mode: 'off' | 'all' | 'one') {
        await invoke('set_repeat', { mode });
      },

      async cycleRepeat() {
        const newMode = await invoke<string>('cycle_repeat');
        return newMode as 'off' | 'all' | 'one';
      },

      async getRepeat() {
        const mode = await invoke<string>('get_repeat');
        return mode as 'off' | 'all' | 'one';
      },

      async getPlaybackCapabilities(): Promise<PlaybackCapabilities> {
        return await invoke<PlaybackCapabilities>('get_playback_capabilities');
      },

      async getPlaybackState(): Promise<string> {
        return await invoke<string>('get_playback_state');
      },

      async getQueue() {
        return await invoke('get_queue');
      },

      async playQueue(queue, startIndex = 0, context) {
        // Lazy loading: If context provided and total count is large, use context-based loading
        const LAZY_LOADING_THRESHOLD = 100;

        // Check context.totalCount (not queue.length) since queue is already limited to 50
        const totalCount = context && 'totalCount' in context ? context.totalCount : queue.length;

        if (context && totalCount > LAZY_LOADING_THRESHOLD) {
          console.log('[TauriPlayerCommandsProvider] Using lazy loading:', {
            totalCount,
            queueSize: queue.length,
            context: context.type,
            threshold: LAZY_LOADING_THRESHOLD,
          });

          // Get current shuffle state
          const shuffleMode = await invoke<string>('get_shuffle');
          const enableShuffle = shuffleMode !== 'off';

          // Take first 50 tracks as initial batch
          const initialBatch = queue.slice(0, 50);

          // Use lazy loading via playQueueWithContext command
          await invoke('play_queue_with_context', {
            context,
            initialBatch,
            startIndex,
            enableShuffle,
          });
        } else {
          // Small queue or no context - use regular playback
          await invoke('play_queue', { queue, startIndex });
        }
      },

      async playQueueWithContext(context, initialBatch, startIndex, enableShuffle) {
        // Lazy loading: Send context and initial batch instead of full queue
        await invoke('play_queue_with_context', {
          context,
          initialBatch,
          startIndex,
          enableShuffle,
        });
      },

      async skipToQueueIndex(index: number) {
        await invoke('skip_to_queue_index', { index });
      },

      // Three-tier queue operations
      async addPlayNext(track) {
        await invoke('add_play_next', { track });
      },

      async addToQueueEnd(track) {
        await invoke('add_to_queue_end', { track });
      },

      async clearPlayNext() {
        await invoke('clear_play_next');
      },

      async clearAddToQueue() {
        await invoke('clear_add_to_queue');
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

  return (
    <PlayerCommandsProvider value={value}>
      <KeyboardShortcutsInitializer />
      {children}
    </PlayerCommandsProvider>
  );
}
