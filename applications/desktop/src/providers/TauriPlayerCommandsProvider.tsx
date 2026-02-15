/**
 * Tauri implementation of PlayerCommands context
 * Bridges desktop Tauri invoke() calls to shared PlayerCommands interface
 * Also handles event-to-store updates and keyboard shortcuts
 */

import { ReactNode, useMemo, useEffect, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import debounce from 'lodash.debounce';
import {
  PlayerCommandsProvider,
  usePlayerStore,
  usePlaybackSession,
  useBackend,
  type PlayerContextValue,
  type PlayerCommandsInterface,
  type PlaybackEventsInterface,
  type PlaybackCapabilities,
  type QueueTrack,
  type BackendTrack,
} from '@soul-player/shared';
import { useKeyboardShortcuts } from '../hooks/useKeyboardShortcuts';

// REMOVED: Ignore window timer (artificial 120ms delay)
// Production players (VLC, Clementine, Audacious) don't use this pattern
// Optimistic UI updates provide instant feedback without race conditions

// Separate component to initialize keyboard shortcuts AFTER context is provided
function KeyboardShortcutsInitializer() {
  useKeyboardShortcuts();
  return null;
}

export function TauriPlayerCommandsProvider({ children }: { children: ReactNode }) {
  const { updateSession, session } = usePlaybackSession();
  const backend = useBackend();
  // REMOVED: ignoringPositionUpdatesRef and ignoreTimerRef (ignore window pattern)
  // Simplified to match production player patterns (VLC, Clementine, etc.)

  // Save current session to database
  const savePlaybackSession = useCallback(async (retryCount = 0) => {
    try {
      const state = usePlayerStore.getState();

      if (!state.currentTrack) {
        return;
      }

      await invoke('save_playback_session', {
        session: {
          current_track_id: state.currentTrack.id,
          queue_track_ids: state.queue.map(t => t.id),
          queue_index: state.queueIndex,
          position_seconds: state.duration ? (state.progress / 100) * state.duration : 0,
          volume: state.volume * 100, // Convert 0-1 to 0-100
          repeat_mode: state.repeatMode,
          shuffle_mode: state.shuffleMode,
          context_type: session.contextType,
          context_id: session.contextId,
          was_playing: state.isPlaying,
        },
      });

      console.log('[PERSISTENCE] Session saved');
    } catch (error) {
      console.error('[PERSISTENCE] Failed to save session:', error);

      // Retry once after 1 second
      if (retryCount === 0) {
        console.log('[PERSISTENCE] Retrying save in 1 second...');
        setTimeout(() => savePlaybackSession(1), 1000);
      }
    }
  }, [session.contextType, session.contextId]);

  // Create debounced save function (5 seconds) for progress updates
  const debouncedSave = useMemo(
    () => debounce(savePlaybackSession, 5000),
    [savePlaybackSession]
  );

  // Cleanup debounced function on unmount
  useEffect(() => {
    return () => {
      debouncedSave.cancel();
    };
  }, [debouncedSave]);

  // Set up event listeners to update store (similar to old usePlaybackEvents hook)
  useEffect(() => {
    // Store unlisten functions for cleanup
    const unlistenFunctions: (() => void)[] = [];
    let isMounted = true;

    // Restore state from database (cold start scenario - no backend state)
    const restoreFromDatabase = async () => {
      console.log('[PERSISTENCE] Cold start detected - restoring from database');

      try {
        // Load persisted session
        const session = await invoke<{
          current_track_id: number | null;
          queue_track_ids: number[];
          queue_index: number;
          position_seconds: number;
          volume: number;
          repeat_mode: string;
          shuffle_mode: string;
          context_type: string | null;
          context_id: string | null;
          was_playing: boolean;
        } | null>('restore_playback_session');

        if (!session || !session.current_track_id) {
          console.log('[PERSISTENCE] No saved session found');
          return;
        }

        // Validate session data
        if (session.queue_track_ids.length === 0) {
          console.warn('[PERSISTENCE] Invalid session: empty queue');
          await invoke('clear_playback_session');
          return;
        }

        if (session.queue_index < 0 || session.queue_index >= session.queue_track_ids.length) {
          console.warn('[PERSISTENCE] Invalid session: queue index out of bounds');
          session.queue_index = 0;
        }

        if (session.volume < 0 || session.volume > 100) {
          console.warn('[PERSISTENCE] Invalid session: volume out of range');
          session.volume = 80;
        }

        // Fetch full track objects by IDs
        const tracks = await backend.getTracksByIds(session.queue_track_ids);

        // Filter out missing tracks and convert to Track format
        const validTracks = tracks.filter((t): t is BackendTrack => t !== null).map(convertBackendTrackToTrack);

        const missingCount = tracks.length - validTracks.length;
        if (missingCount > 0) {
          console.warn(`[PERSISTENCE] ${missingCount} track(s) were unavailable and skipped`);
          // TODO: Show toast notification when toast system is available
          // toast.info(`${missingCount} track(s) were unavailable and skipped`);
        }

        if (validTracks.length === 0) {
          console.warn('[PERSISTENCE] All tracks missing - clearing session');
          await invoke('clear_playback_session');
          return;
        }

        // Adjust queue index if current track is missing
        let queueIndex = session.queue_index;
        if (!validTracks[queueIndex]) {
          queueIndex = 0;
          console.warn('[PERSISTENCE] Current track missing - starting from first valid track');
        }

        if (!isMounted) return;

        const currentTrackDuration = validTracks[queueIndex]?.duration ?? 0;
        const restoredProgress = session.position_seconds && currentTrackDuration > 0
          ? Math.min(100, (session.position_seconds / currentTrackDuration) * 100)
          : 0;

        // Update Zustand store
        usePlayerStore.setState({
          queue: validTracks,
          queueIndex,
          currentTrack: validTracks[queueIndex],
          volume: session.volume / 100, // Convert 0-100 to 0-1
          isPlaying: false, // Always paused on cold start
          repeatMode: session.repeat_mode as 'off' | 'all' | 'one',
          shuffleMode: session.shuffle_mode as 'off' | 'random' | 'smart',
          progress: restoredProgress,
          duration: currentTrackDuration,
        });

        // Convert Track[] to QueueTrack[] for backend
        const queueForBackend = validTracks.map(track => ({
          trackId: String(track.id),
          title: track.title,
          artist: track.artist,
          album: track.album,
          albumId: track.albumId,
          filePath: track.filePath,
          durationSeconds: track.duration,
          trackNumber: track.trackNumber,
          coverArtPath: track.coverArtPath,
        }));

        // Load queue into backend WITHOUT starting playback
        // This ensures skip next/prev work, but doesn't auto-play
        await invoke('load_queue_paused', {
          queue: queueForBackend,
          startIndex: queueIndex
        });

        // Seek to saved position if we have one
        if (session.position_seconds > 0) {
          await invoke('seek_to', { position: session.position_seconds });
        }

        // Set backend preferences (volume, repeat, shuffle)
        await invoke('set_volume', { volume: session.volume });
        await invoke('set_repeat', { mode: session.repeat_mode });
        await invoke('set_shuffle', { mode: session.shuffle_mode });

        // Restore playback context
        if (session.context_type && session.context_id) {
          await backend.recordContext({
            contextType: session.context_type as 'album' | 'artist' | 'playlist' | 'genre' | 'tracks',
            contextId: session.context_id,
            contextName: null,
            contextArtworkPath: null,
          });
        }

        console.log('[PERSISTENCE] State restored from database:', {
          queueLength: validTracks.length,
          currentTrack: validTracks[queueIndex]?.title,
          volumeFromDB: session.volume,
          volumeConverted: session.volume / 100,
          position: session.position_seconds,
          progress: restoredProgress.toFixed(1) + '%',
        });
      } catch (error) {
        console.error('[PERSISTENCE] Failed to restore from database:', error);
      }
    };

    // Convert QueueTrack to Track (store expects Track type)
    const convertQueueTrackToTrack = (queueTrack: QueueTrack): import('@soul-player/shared/types').Track => ({
      id: parseInt(queueTrack.trackId, 10),
      title: queueTrack.title,
      artist: queueTrack.artist,
      album: queueTrack.album || '',
      albumId: queueTrack.albumId,
      filePath: queueTrack.filePath,
      duration: queueTrack.durationSeconds ?? 0,
      trackNumber: queueTrack.trackNumber ?? undefined,
      coverArtPath: queueTrack.coverArtPath,
      addedAt: new Date().toISOString(), // Not available in QueueTrack, use current time
    });

    // Convert BackendTrack to Track (store expects Track type)
    const convertBackendTrackToTrack = (backendTrack: BackendTrack): import('@soul-player/shared/types').Track => ({
      id: backendTrack.id,
      title: backendTrack.title,
      artist: backendTrack.artist_name || 'Unknown Artist',
      album: backendTrack.album_title || '',
      albumId: backendTrack.album_id,
      filePath: backendTrack.file_path || '',
      duration: backendTrack.duration_seconds ?? 0,
      trackNumber: backendTrack.track_number,
      year: backendTrack.year,
      coverArtPath: backendTrack.cover_art_path,
      addedAt: new Date().toISOString(), // Not available in BackendTrack, use current time
    });

    // Sync state from backend (hot reload scenario - backend is still running)
    const syncFromBackend = async () => {
      console.log('[PERSISTENCE] Hot reload detected - syncing from backend');

      try {
        const [track, queue, queueIndex, position, volume, repeat, shuffle] = await Promise.all([
          invoke<QueueTrack | null>('get_current_track'),
          invoke<QueueTrack[]>('get_queue'),
          invoke<number>('get_queue_index'),
          invoke<number>('get_position'),
          invoke<number>('get_volume'),
          invoke<string>('get_repeat'),
          invoke<string>('get_shuffle'),
        ]);

        if (!isMounted) return;

        // Convert QueueTrack to Track format
        const currentTrack = track ? convertQueueTrackToTrack(track) : null;
        const queueTracks = queue.map(convertQueueTrackToTrack);

        // Update store with backend state
        usePlayerStore.setState({
          currentTrack,
          queue: queueTracks,
          queueIndex,
          volume: volume / 100, // Convert 0-100 to 0-1
          progress: track && position ? (position / track.durationSeconds!) * 100 : 0,
          duration: track?.durationSeconds ?? 0,
          repeatMode: repeat as 'off' | 'all' | 'one',
          shuffleMode: shuffle as 'off' | 'random' | 'smart',
        });

        console.log('[PERSISTENCE] State synced from backend:', {
          hasTrack: !!track,
          queueLength: queue.length,
          volumeFromBackend: volume,
          volumeConverted: volume / 100,
        });
      } catch (error) {
        console.error('[PERSISTENCE] Failed to sync from backend:', error);
        // Fall back to database restore
        await restoreFromDatabase();
      }
    };

    // Sync initial state from backend on mount (non-blocking)
    // Deferred to allow React tree to render first, then update state
    const syncInitialState = async () => {
      // Defer state sync to next tick to avoid blocking initial render
      await new Promise(resolve => setTimeout(resolve, 0));

      try {
        // Check if backend has active state (hot reload scenario)
        const backendTrack = await invoke<QueueTrack | null>('get_current_track');

        if (backendTrack) {
          // Hot reload - backend is alive
          await syncFromBackend();
        } else {
          // Cold start - restore from database
          await restoreFromDatabase();
        }
      } catch (error) {
        console.error('[PERSISTENCE] Failed to sync initial state:', error);
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

        // Listen for position updates (with ignore window check)
        const unlistenPositionUpdated = await listen<number>('playback:position-updated', (event) => {
          // SIMPLIFIED: No ignore window check needed
          // Optimistic updates in useSeekBar provide instant feedback
          // Backend position updates naturally sync after seek completes
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

  // Subscribe to track changes - save immediately
  useEffect(() => {
    let lastTrackId = usePlayerStore.getState().currentTrack?.id;

    const unsubscribe = usePlayerStore.subscribe((state) => {
      const currentTrackId = state.currentTrack?.id;
      if (currentTrackId !== lastTrackId) {
        lastTrackId = currentTrackId;
        savePlaybackSession();
      }
    });

    return unsubscribe;
  }, [savePlaybackSession]);

  // Subscribe to queue changes - save immediately
  useEffect(() => {
    let lastQueueLength = usePlayerStore.getState().queue.length;

    const unsubscribe = usePlayerStore.subscribe((state) => {
      if (state.queue.length !== lastQueueLength) {
        lastQueueLength = state.queue.length;
        savePlaybackSession();
      }
    });

    return unsubscribe;
  }, [savePlaybackSession]);

  // Subscribe to volume changes - save if changed by >5%
  useEffect(() => {
    let lastSavedVolume = usePlayerStore.getState().volume;

    const unsubscribe = usePlayerStore.subscribe((state) => {
      if (Math.abs(state.volume - lastSavedVolume) > 0.05) {
        lastSavedVolume = state.volume;
        savePlaybackSession();
      }
    });

    return unsubscribe;
  }, [savePlaybackSession]);

  // Subscribe to repeat/shuffle mode changes - save immediately
  useEffect(() => {
    let lastModes = `${usePlayerStore.getState().repeatMode}-${usePlayerStore.getState().shuffleMode}`;

    const unsubscribe = usePlayerStore.subscribe((state) => {
      const currentModes = `${state.repeatMode}-${state.shuffleMode}`;
      if (currentModes !== lastModes) {
        lastModes = currentModes;
        savePlaybackSession();
      }
    });

    return unsubscribe;
  }, [savePlaybackSession]);

  // Subscribe to progress changes - save debounced (5s)
  useEffect(() => {
    let lastProgress = usePlayerStore.getState().progress;

    const unsubscribe = usePlayerStore.subscribe((state) => {
      if (state.progress !== lastProgress) {
        lastProgress = state.progress;
        debouncedSave();
      }
    });

    return unsubscribe;
  }, [debouncedSave]);

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
        // SIMPLIFIED: Direct seek without ignore window timer
        // Pattern used by VLC, Clementine, Audacious (50-150ms latency)
        // Optimistic updates in useSeekBar provide instant visual feedback
        const seekStartTime = performance.now();
        console.log(`[SEEK PERF] TauriProvider.seek() called at +${seekStartTime.toFixed(2)}ms`);

        try {
          await invoke('seek_to', { position });
          const seekDuration = performance.now() - seekStartTime;
          console.log(`[SEEK PERF] invoke('seek_to') completed in ${seekDuration.toFixed(2)}ms`);
        } catch (error) {
          console.error(`[SEEK PERF] invoke('seek_to') failed:`, error);
          throw error;
        }
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
