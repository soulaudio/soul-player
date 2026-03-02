/**
 * Tauri implementation of PlayerCommands context
 * Bridges desktop Tauri invoke() calls to shared PlayerCommands interface
 * Also handles event-to-store updates and keyboard shortcuts
 */

import { ReactNode, useMemo, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import debounce from 'lodash.debounce';
import {
  PlayerCommandsProvider,
  usePlayerStore,
  usePlaybackSession,
  useBackend,
  debug,
  type PlayerContextValue,
  type PlayerCommandsInterface,
  type PlaybackEventsInterface,
  type PlaybackCapabilities,
  type BackendTrack,
} from '@soul-player/shared';
import { useKeyboardShortcuts } from '../hooks/useKeyboardShortcuts';
import {
  invokeValidated,
  PlaybackSessionSchema,
  PlaybackContextSchema,
} from '../types/validation';

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
        debug.log('[PERSISTENCE] Skipping save - no current track');
        return;
      }

      debug.log('[PERSISTENCE] Saving session with track:', state.currentTrack.title);

      const sessionData = {
        currentTrackId: state.currentTrack.id,
        queueTrackIds: state.queue.map(t => t.id),
        queueIndex: state.queueIndex,
        positionSeconds: state.duration ? (state.progress / 100) * state.duration : 0,
        volume: state.volume * 100, // Convert 0-1 to 0-100
        repeatMode: state.repeatMode,
        shuffleMode: state.shuffleMode,
        contextType: session.contextType,
        contextId: session.contextId,
        wasPlaying: state.isPlaying,
      };

      await invoke('save_playback_session', { session: sessionData });

      debug.log('[PERSISTENCE] Session saved:', {
        currentTrackId: sessionData.currentTrackId,
        currentTrackTitle: state.currentTrack.title,
        queueLength: sessionData.queueTrackIds.length,
        position: sessionData.positionSeconds.toFixed(1) + 's',
      });
    } catch (error) {
      debug.error('[PERSISTENCE] Failed to save session:', error);

      // Retry once after 1 second
      if (retryCount === 0) {
        debug.log('[PERSISTENCE] Retrying save in 1 second...');
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
      debug.log('[PERSISTENCE] Cold start detected - restoring from database');

      try {
        // Load persisted session (with runtime validation)
        const session = await invokeValidated(
          'restore_playback_session',
          PlaybackSessionSchema.nullable()
        );

        debug.log('[PERSISTENCE] Session data from database:', session);

        if (!session || !session.currentTrackId) {
          debug.log('[PERSISTENCE] No saved session found or no current track ID');
          return;
        }

        // Validate session data
        if (session.queueTrackIds.length === 0) {
          debug.warn('[PERSISTENCE] Invalid session: empty queue');
          await invoke('clear_playback_session');
          return;
        }

        if (session.queueIndex < 0 || session.queueIndex >= session.queueTrackIds.length) {
          debug.warn('[PERSISTENCE] Invalid session: queue index out of bounds');
          session.queueIndex = 0;
        }

        if (session.volume < 0 || session.volume > 100) {
          debug.warn('[PERSISTENCE] Invalid session: volume out of range');
          session.volume = 80;
        }

        // Fetch full track objects by IDs
        const tracks = await backend.getTracksByIds(session.queueTrackIds);

        // Filter out missing tracks and convert to Track format
        const validTracks = tracks.filter((t): t is BackendTrack => t !== null).map(convertBackendTrackToTrack);

        const missingCount = tracks.length - validTracks.length;
        if (missingCount > 0) {
          debug.warn(`[PERSISTENCE] ${missingCount} track(s) were unavailable and skipped`);
          // TODO: Show toast notification when toast system is available
          // toast.info(`${missingCount} track(s) were unavailable and skipped`);
        }

        if (validTracks.length === 0) {
          debug.warn('[PERSISTENCE] All tracks missing - clearing session');
          await invoke('clear_playback_session');
          return;
        }

        // Adjust queue index if current track is missing
        let queueIndex = session.queueIndex;
        if (!validTracks[queueIndex]) {
          queueIndex = 0;
          debug.warn('[PERSISTENCE] Current track missing - starting from first valid track');
        }

        if (!isMounted) return;

        const currentTrackDuration = validTracks[queueIndex]?.duration ?? 0;
        const restoredProgress = session.positionSeconds && currentTrackDuration > 0
          ? Math.min(100, (session.positionSeconds / currentTrackDuration) * 100)
          : 0;

        // Update Zustand store FIRST so the sidebar shows the track immediately.
        // This must happen before any async backend calls that could fail.
        usePlayerStore.setState({
          queue: validTracks,
          queueIndex,
          currentTrack: validTracks[queueIndex],
          volume: session.volume / 100, // Convert 0-100 to 0-1
          isPlaying: false, // Always paused on cold start
          repeatMode: session.repeatMode as 'off' | 'all' | 'one',
          shuffleMode: session.shuffleMode as 'off' | 'random' | 'smart',
          progress: restoredProgress,
          duration: currentTrackDuration,
        });

        debug.log('[PERSISTENCE] State restored from database:', {
          queueLength: validTracks.length,
          queueIndex,
          currentTrack: validTracks[queueIndex]?.title,
          currentTrackId: validTracks[queueIndex]?.id,
          position: session.positionSeconds,
          progress: restoredProgress.toFixed(1) + '%',
        });

        // Restore backend playback state (queue + position) — isolated so UI is not affected by failure
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

        try {
          await invoke('restore_playback_state', {
            queue: queueForBackend,
            startIndex: queueIndex,
            positionSeconds: session.positionSeconds,
            volume: session.volume, // 0-100
            repeatMode: session.repeatMode,
            shuffleMode: session.shuffleMode,
          });
        } catch (backendError) {
          debug.warn('[PERSISTENCE] Backend state restoration failed (UI still restored):', backendError);
        }

        // Restore playback context
        if (session.contextType && session.contextId) {
          try {
            await backend.recordContext({
              contextType: session.contextType as 'album' | 'artist' | 'playlist' | 'genre' | 'tracks',
              contextId: session.contextId,
              contextName: null,
              contextArtworkPath: null,
            });
          } catch (contextError) {
            debug.warn('[PERSISTENCE] Context restoration failed:', contextError);
          }
        }
      } catch (error) {
        debug.error('[PERSISTENCE] Failed to restore from database:', error);
      }
    };

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
      debug.log('[PERSISTENCE] Hot reload detected - syncing from backend');

      try {
        // get_current_track returns soul_playback::QueueTrack (snake_case: id, path, duration as {secs,nanos}, track_number)
        // get_volume returns 0.0-1.0 (already divided by 100 in Rust)
        const [track, queue, queueIndex, position, volume, repeat, shuffle, playbackState] = await Promise.all([
          invoke<any | null>('get_current_track'),
          invoke<any[]>('get_queue'),
          invoke<number>('get_queue_index'),
          invoke<number>('get_position'),
          invoke<number>('get_volume'),
          invoke<string>('get_repeat'),
          invoke<string>('get_shuffle'),
          invoke<string>('get_playback_state'),
        ]);

        if (!isMounted) return;

        // Convert Rust QueueTrack (snake_case fields) to frontend Track
        const convertRustTrack = (qt: any): import('@soul-player/shared/types').Track => {
          const durationSecs = qt.duration && typeof qt.duration === 'object'
            ? (qt.duration.secs ?? 0) + (qt.duration.nanos ?? 0) / 1e9
            : (qt.durationSeconds ?? 0);
          return {
            id: parseInt(qt.id ?? qt.trackId, 10),
            title: qt.title || '',
            artist: qt.artist || '',
            album: qt.album || '',
            albumId: qt.albumId,
            filePath: qt.filePath || String(qt.path || ''),
            duration: durationSecs,
            trackNumber: qt.track_number ?? qt.trackNumber ?? undefined,
            coverArtPath: qt.cover_art_path ?? qt.coverArtPath ?? undefined,
            addedAt: new Date().toISOString(),
          };
        };

        const currentTrack = track ? convertRustTrack(track) : null;
        const queueTracks = (queue ?? []).map(convertRustTrack);
        const durationSecs = currentTrack?.duration ?? 0;

        usePlayerStore.setState({
          currentTrack,
          queue: queueTracks,
          queueIndex,
          volume, // already 0-1 from Rust (get_volume divides by 100 internally)
          isPlaying: playbackState === 'Playing',
          progress: currentTrack && position && durationSecs > 0
            ? Math.min(100, (position / durationSecs) * 100)
            : 0,
          duration: durationSecs,
          repeatMode: repeat as 'off' | 'all' | 'one',
          shuffleMode: shuffle as 'off' | 'random' | 'smart',
        });

        debug.log('[PERSISTENCE] State synced from backend:', {
          hasTrack: !!track,
          queueLength: queue?.length ?? 0,
          isPlaying: playbackState === 'Playing',
          volume,
        });
      } catch (error) {
        debug.error('[PERSISTENCE] Failed to sync from backend:', error);
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
        // Check if backend has active state (hot reload scenario).
        // Use raw invoke — Rust returns soul_playback::QueueTrack with `id` (not `trackId`).
        const backendTrack = await invoke<any>('get_current_track');

        // Any non-null response means the backend has a loaded track
        const hasValidTrack = backendTrack != null && (backendTrack.id || backendTrack.trackId);

        debug.log('[PERSISTENCE] Initial state sync - backend track:', hasValidTrack ? 'exists (hot reload)' : 'null (cold start)');

        if (hasValidTrack) {
          // Hot reload - backend is alive
          debug.log('[PERSISTENCE] Hot reload path - syncing from backend');
          await syncFromBackend();
        } else {
          // Cold start - restore from database
          debug.log('[PERSISTENCE] Cold start path - restoring from database');
          await restoreFromDatabase();
        }
      } catch (error) {
        debug.error('[PERSISTENCE] Failed to sync initial state:', error);
        // On error, try database restore as fallback
        debug.log('[PERSISTENCE] Error in sync - falling back to database restore');
        try {
          await restoreFromDatabase();
        } catch (fallbackError) {
          debug.error('[PERSISTENCE] Fallback restore also failed:', fallbackError);
        }
      }
    };

    const setupListeners = async () => {
      try {
        // Listen for playback state changes
        const unlistenStateChanged = await listen<string>('playback:state-changed', (event) => {
          const isPlaying = event.payload === 'Playing';
          usePlayerStore.setState({ isPlaying });
          // Session's isPlaying is derived from Zustand store - no need to update
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
        const unlistenTrackChanged = await listen<{ id: string; title: string; artist: string; album: string; filePath: string; duration: number; addedAt: string; coverArtPath?: string } | null>('playback:track-changed', async (event) => {
          const trackPayload = event.payload;
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

            // Update session with latest context from backend
            // Note: currentTrack is derived from Zustand store, don't pass to updateSession
            try {
              const context = await invokeValidated('get_current_playback_context', PlaybackContextSchema.nullable());
              if (context) {
                updateSession({
                  contextType: context.context_type as 'album' | 'artist' | 'playlist',
                  contextId: context.context_id,
                  contextName: context.context_name,
                });
              }
            } catch (error) {
              debug.error('[TauriPlayerCommandsProvider] Failed to get context:', error);
            }
          } else {
            // Null track means the queue has ended. Clear the current track and
            // reset isPlaying so the play/pause button reflects the stopped state.
            usePlayerStore.setState({
              currentTrack: null,
              isPlaying: false,
              duration: 0,
              progress: 0,
            });
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
            debug.error('[TauriPlayerCommandsProvider] Failed to get shuffle mode:', error);
          }
        });
        unlistenFunctions.push(unlistenQueueUpdated);

        // Listen for errors
        const unlistenError = await listen<string>('playback:error', (event) => {
          debug.error('[TauriPlayerCommandsProvider] Playback error:', event.payload);
        });
        unlistenFunctions.push(unlistenError);
      } catch (error) {
        debug.error('[TauriPlayerCommandsProvider] Failed to set up event listeners:', error);
      }
    };

    // Initialize listeners and state in parallel
    // Use proper promise handling to avoid loading cursor issues on macOS
    Promise.all([setupListeners(), syncInitialState()])
      .catch((error) => {
        debug.error('[TauriPlayerCommandsProvider] Initialization failed:', error);
      });

    // Cleanup function
    return () => {
      isMounted = false;
      unlistenFunctions.forEach(fn => fn());
    };
  }, [updateSession]);

  // Consolidated subscription for all state changes
  useEffect(() => {
    let lastState = usePlayerStore.getState();

    const unsubscribe = usePlayerStore.subscribe((state) => {
      // Check what changed
      const trackChanged = state.currentTrack?.id !== lastState.currentTrack?.id;
      const queueChanged = state.queue.length !== lastState.queue.length;
      const volumeChanged = Math.abs(state.volume - lastState.volume) > 0.05;
      const modesChanged =
        state.repeatMode !== lastState.repeatMode ||
        state.shuffleMode !== lastState.shuffleMode;
      const progressChanged = state.progress !== lastState.progress;

      // Save based on what changed
      if (trackChanged || queueChanged || volumeChanged || modesChanged) {
        savePlaybackSession();
      } else if (progressChanged) {
        debouncedSave();
      }

      lastState = state;
    });

    return unsubscribe;
  }, [savePlaybackSession, debouncedSave]);

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
        // Desktop backend expects 0-100, but shared interface uses 0-1.
        // Clamp to [0, 1] before converting to avoid sending values like 101 to the backend.
        const clamped = Math.max(0, Math.min(1, volume));
        await invoke('set_volume', { volume: Math.round(clamped * 100) });
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
          debug.log('[TauriPlayerCommandsProvider] Using lazy loading:', {
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

        // Sync queue into Zustand store so the persistence subscription can save it.
        // Without this, queue stays empty in the store and restoreFromDatabase bails on empty queue.
        const tracks = queue.map(qt => ({
          id: parseInt(qt.trackId, 10),
          title: qt.title || '',
          artist: qt.artist || '',
          album: qt.album || '',
          albumId: qt.albumId,
          filePath: qt.filePath || '',
          duration: qt.durationSeconds ?? 0,
          trackNumber: qt.trackNumber ?? undefined,
          coverArtPath: qt.coverArtPath,
          addedAt: new Date().toISOString(),
        }));
        usePlayerStore.setState({ queue: tracks, queueIndex: startIndex });
      },

      async playQueueWithContext(context, initialBatch, startIndex, enableShuffle) {
        // Lazy loading: Send context and initial batch instead of full queue
        await invoke('play_queue_with_context', {
          context,
          initialBatch,
          startIndex,
          enableShuffle,
        });

        // Sync initial batch into store for persistence
        const tracks = initialBatch.map(qt => ({
          id: parseInt(qt.trackId, 10),
          title: qt.title || '',
          artist: qt.artist || '',
          album: qt.album || '',
          albumId: qt.albumId,
          filePath: qt.filePath || '',
          duration: qt.durationSeconds ?? 0,
          trackNumber: qt.trackNumber ?? undefined,
          coverArtPath: qt.coverArtPath,
          addedAt: new Date().toISOString(),
        }));
        usePlayerStore.setState({ queue: tracks, queueIndex: startIndex });
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

    };

    // Events implementation using Tauri event listeners
    const events: PlaybackEventsInterface = {
      onStateChange(callback) {
        // The backend emits a string ("Playing" | "Paused" | "Stopped"), not a boolean.
        // Typed as string here so the conversion to boolean is explicit.
        const unlisten = listen<string>('playback:state-changed', (event) => {
          callback(event.payload === 'Playing');
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
