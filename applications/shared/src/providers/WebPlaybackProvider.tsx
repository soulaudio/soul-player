/**
 * WebPlaybackProvider - Abstract web playback provider
 *
 * Generic provider that bridges WASM playback (soul-playback-web) to
 * the shared PlayerCommands interface. Reusable across marketing demo
 * and future web player applications.
 *
 * Key features:
 * - Accepts any data storage implementing PlaybackDataStorage interface
 * - Initializes WasmPlaybackAdapter internally
 * - Wires up all event listeners automatically
 * - Provides PlayerCommandsContext to children
 * - Handles cleanup on unmount
 *
 * Usage:
 * ```tsx
 * <WebPlaybackProvider storage={myStorage}>
 *   <App />
 * </WebPlaybackProvider>
 * ```
 */

import { ReactNode, useMemo, useEffect, useState, useRef } from 'react';
import {
  PlayerCommandsProvider,
  type PlayerContextValue,
  type PlayerCommandsInterface,
  type PlaybackEventsInterface,
  type PlaybackCapabilities,
} from '../contexts/PlayerCommandsContext';
import { WasmPlaybackAdapter, PlaybackState, toQueueTrack } from '@soul-player/playback-web';
import { usePlayerStore } from '../stores/player';
import { usePlaybackSession } from '../contexts/PlaybackSessionContext';
import type { PlaybackDataStorage } from '../types/storage';
import type { Track } from '../types';

interface WebPlaybackProviderProps {
  storage: PlaybackDataStorage;
  children: ReactNode;
}

export function WebPlaybackProvider({ storage, children }: WebPlaybackProviderProps) {
  const [isInitialized, setIsInitialized] = useState(false);
  const managerRef = useRef<WasmPlaybackAdapter | null>(null);
  const instanceId = useRef(Math.random().toString(36).substring(7));
  const { updateSession, clearSession } = usePlaybackSession();

  // Initialize WASM manager on mount
  useEffect(() => {
    const manager = new WasmPlaybackAdapter();
    managerRef.current = manager;

    manager
      .initialize()
      .then(() => {
        console.log(`[WebPlaybackProvider:${instanceId.current}] WASM manager initialized`);

        // Setup event bridge to shared store
        setupEventBridge(manager, storage);

        // CRITICAL FIX: Clear any stale UI state if queue is empty
        if (manager.queueLength() === 0) {
          console.log('[WebPlaybackProvider] Queue is empty on init, clearing stale UI state');
          usePlayerStore.setState({
            currentTrack: null,
            queue: [],
            queueIndex: -1,
            isPlaying: false,
            progress: 0,
            duration: 0,
          });
        }

        setIsInitialized(true);
      })
      .catch((err) => {
        console.error(`[WebPlaybackProvider:${instanceId.current}] Failed to initialize WASM:`, err);
      });

    // Cleanup on unmount
    return () => {
      if (managerRef.current) {
        console.log('[WebPlaybackProvider] Cleaning up WASM manager');
        managerRef.current.destroy();
        managerRef.current = null;
      }
    };
  }, [storage]);

  const value = useMemo<PlayerContextValue>(() => {
    // Helper to get manager, throws if not initialized yet
    const getManagerOrThrow = () => {
      const manager = managerRef.current;
      if (!manager || !isInitialized) {
        throw new Error('WASM playback manager not initialized yet');
      }
      return manager;
    };

    // Commands implementation using WASM playback manager
    const commands: PlayerCommandsInterface = {
      async playTrack(trackId: string | number) {
        const track = storage.getTrackById(String(trackId));
        if (!track) throw new Error(`Track ${trackId} not found`);

        const queueTrack = toQueueTrack(track);
        const manager = getManagerOrThrow();
        // Unlock audio during user gesture
        await manager.unlock();
        manager.clearQueue();
        manager.addToQueueNext(queueTrack);

        // Verify queue has items before playing
        if (manager.queueLength() === 0) {
          throw new Error('Failed to add track to queue');
        }

        await manager.play();
      },

      async pausePlayback() {
        const manager = getManagerOrThrow();
        // Validate queue exists before pausing
        if (manager.queueLength() === 0) {
          console.warn('[WebPlaybackProvider] Cannot pause - queue is empty');
          return;
        }
        manager.pause();
      },

      async resumePlayback() {
        const manager = getManagerOrThrow();
        // Validate queue exists before resuming
        if (manager.queueLength() === 0) {
          console.warn('[WebPlaybackProvider] Cannot resume - queue is empty');
          return;
        }
        // Unlock audio during user gesture
        await manager.unlock();
        await manager.play();
      },

      async stopPlayback() {
        getManagerOrThrow().stop();
        // Clear session when playback stops
        clearSession();
      },

      async skipNext() {
        await getManagerOrThrow().next();
      },

      async skipPrevious() {
        await getManagerOrThrow().previous();
      },

      async seek(position: number) {
        getManagerOrThrow().seek(position);
      },

      async setVolume(volume: number) {
        // WASM manager expects 0-100, shared interface uses 0-1
        const volumePercent = Math.max(0, Math.min(100, Math.round(volume * 100)));
        getManagerOrThrow().setVolume(volumePercent);
      },

      async setShuffle(mode: 'off' | 'random' | 'smart') {
        const { ShuffleMode } = await import('@soul-player/playback-web');
        const modeMap = {
          off: ShuffleMode.Off,
          random: ShuffleMode.Random,
          smart: ShuffleMode.Smart,
        };
        getManagerOrThrow().setShuffle(modeMap[mode]);
      },

      async cycleShuffle() {
        const { ShuffleMode } = await import('@soul-player/playback-web');
        const currentMode = getManagerOrThrow().getShuffle();

        // Cycle: Off → Random → Smart → Off
        const nextMode =
          currentMode === ShuffleMode.Off
            ? ShuffleMode.Random
            : currentMode === ShuffleMode.Random
            ? ShuffleMode.Smart
            : ShuffleMode.Off;

        getManagerOrThrow().setShuffle(nextMode);

        // Return string representation
        return nextMode === ShuffleMode.Off ? 'off' : nextMode === ShuffleMode.Random ? 'random' : 'smart';
      },

      async getShuffle() {
        const { ShuffleMode } = await import('@soul-player/playback-web');
        const currentMode = getManagerOrThrow().getShuffle();

        return currentMode === ShuffleMode.Off ? 'off' : currentMode === ShuffleMode.Random ? 'random' : 'smart';
      },

      async setRepeatMode(mode: 'off' | 'all' | 'one') {
        const { RepeatMode } = await import('@soul-player/playback-web');
        const modeMap = {
          off: RepeatMode.Off,
          all: RepeatMode.All,
          one: RepeatMode.One,
        };
        getManagerOrThrow().setRepeat(modeMap[mode]);
      },

      async cycleRepeat() {
        const { RepeatMode } = await import('@soul-player/playback-web');
        const currentMode = getManagerOrThrow().getRepeat();

        // Cycle: Off → All → One → Off
        const nextMode =
          currentMode === RepeatMode.Off
            ? RepeatMode.All
            : currentMode === RepeatMode.All
            ? RepeatMode.One
            : RepeatMode.Off;

        getManagerOrThrow().setRepeat(nextMode);

        return nextMode === RepeatMode.Off ? 'off' : nextMode === RepeatMode.All ? 'all' : 'one';
      },

      async getRepeat() {
        const { RepeatMode } = await import('@soul-player/playback-web');
        const currentMode = getManagerOrThrow().getRepeat();

        return currentMode === RepeatMode.Off ? 'off' : currentMode === RepeatMode.All ? 'all' : 'one';
      },

      async getPlaybackCapabilities(): Promise<PlaybackCapabilities> {
        return {
          hasNext: getManagerOrThrow().hasNext(),
          hasPrevious: getManagerOrThrow().hasPrevious(),
        };
      },

      async getQueue() {
        // Return current queue from manager with cover art
        // Note: coverUrl is not stored in WASM, so we look it up from storage
        const manager = managerRef.current;
        if (!manager || !isInitialized) {
          console.warn('[WebPlaybackProvider] getQueue called before initialization, returning empty queue');
          return [];
        }

        return manager
          .getQueue()
          .map((track) => {
            // Look up demo track to get coverUrl
            const demoTrack = storage.getTrackById(track.id);

            return {
              trackId: track.id,
              title: track.title,
              artist: track.artist,
              album: track.album || null,
              filePath: track.path,
              durationSeconds: track.duration_secs || null,
              trackNumber: track.track_number || null,
              coverArtPath: demoTrack?.coverUrl || undefined,
            };
          });
      },

      async playQueueWithContext(context, initialBatch, startIndex, enableShuffle) {
        console.log('[WebPlaybackProvider] playQueueWithContext called:', {
          context,
          batchSize: initialBatch.length,
          startIndex,
          enableShuffle,
        });

        // For web playback, just call playQueue with the initial batch
        // Context tracking not fully implemented in web
        await this.playQueue(initialBatch, startIndex);

        // Apply shuffle if requested
        if (enableShuffle) {
          await this.setShuffle('random');
        }
      },

      async playQueue(queue, startIndex = 0, context) {
        try {
          console.log(`[WebPlaybackProvider:${instanceId.current}] playQueue called:`, {
            queueLength: queue.length,
            startIndex,
            context,
            firstTrack: queue[0]?.title,
          });

          // CRITICAL: Unlock audio during user gesture (browser autoplay policy)
          // This must be called synchronously in the click handler, before any async operations
          const manager = getManagerOrThrow();
          await manager.unlock();

          // CRITICAL FIX: Update session context IMMEDIATELY (before WASM operations)
          // This ensures isActiveContext() returns true instantly, fixing play/pause toggle
          if (context) {
            let contextType: 'album' | 'artist' | 'playlist' | null = null;
            let contextId = '';

            // Extract context ID based on type (type narrowing for union type)
            if (context.type === 'Album') {
              contextType = 'album';
              contextId = String(context.albumId);
            } else if (context.type === 'Artist') {
              contextType = 'artist';
              contextId = String(context.artistId);
            } else if (context.type === 'Playlist') {
              contextType = 'playlist';
              contextId = String(context.playlistId);
            }

            // Only update session for album/artist/playlist contexts (not AllTracks or Search)
            if (contextType) {
              updateSession({
                contextType,
                contextId,
                contextName: queue[0]?.album || queue[0]?.artist || 'Unknown',
                startedAt: new Date(),
              });
              console.log('[WebPlaybackProvider] Session context updated:', { contextType, contextId });
            }
          }

          // Convert QueueTrack[] to WASM QueueTrack format
          const wasmQueue = queue.map((track) => {
            const demoTrack = storage.getTrackById(track.trackId);
            if (!demoTrack) {
              console.error('[WebPlaybackProvider] Track not found:', track.trackId);
            }
            return {
              id: track.trackId,
              title: track.title || 'Unknown',
              artist: track.artist || 'Unknown Artist', // CRITICAL: artist must be a string, never undefined
              album: track.album || undefined,
              path: track.filePath,
              duration_secs: track.durationSeconds || 0,
              track_number: track.trackNumber || undefined,
              coverUrl: demoTrack?.coverUrl,
            };
          });

        console.log('[WebPlaybackProvider] Converted to WASM queue:', {
          length: wasmQueue.length,
          firstPath: wasmQueue[0]?.path,
          allHavePaths: wasmQueue.every((t) => t.path),
        });

        // Load the queue starting from the specified index
        const reorderedQueue = [...wasmQueue.slice(startIndex), ...wasmQueue.slice(0, startIndex)];

        console.log('[WebPlaybackProvider] Loading playlist to WASM, starting track:', reorderedQueue[0]?.title);

        // Validate queue has tracks
        if (reorderedQueue.length === 0) {
          throw new Error('Cannot play empty queue');
        }

        // Validate all tracks have required fields (WASM requires: id, path, title, artist)
        const invalidTracks = reorderedQueue.filter((t) => !t.path || !t.id || !t.title || !t.artist);
        if (invalidTracks.length > 0) {
          console.error('[WebPlaybackProvider] Invalid tracks in queue:', invalidTracks);
          throw new Error(`Queue contains ${invalidTracks.length} invalid track(s) - missing required fields`);
        }

        console.log('[WebPlaybackProvider] Queue validation passed:', {
          totalTracks: reorderedQueue.length,
          firstTrack: {
            id: reorderedQueue[0].id,
            title: reorderedQueue[0].title,
            artist: reorderedQueue[0].artist,
            path: reorderedQueue[0].path,
            duration_secs: reorderedQueue[0].duration_secs,
          },
        });

        // Stop current playback first (Spotify behavior)
        // This ensures clicking play starts fresh, doesn't append
        try {
          const manager = getManagerOrThrow();
          manager.stop();
          manager.loadPlaylist(reorderedQueue);

          // Verify queue was loaded
          if (manager.queueLength() === 0) {
            throw new Error('Queue is empty after loading playlist');
          }

          console.log('[WebPlaybackProvider] Queue loaded, length:', manager.queueLength());
          await manager.play();
          console.log('[WebPlaybackProvider] Playback started successfully');
        } catch (error) {
          console.error('[WebPlaybackProvider] Failed to start playback:', error);
          throw error;
        }
        } catch (error) {
          console.error('[WebPlaybackProvider] ERROR in playQueue:', error);
          throw error;
        }
      },

      async skipToQueueIndex(index: number) {
        // Use the manager's built-in method that maintains history
        await getManagerOrThrow().skipToQueueIndex(index);
      },

      // Three-tier queue operations
      async addPlayNext(track) {
        const demoTrack = storage.getTrackById(track.trackId);
        if (!demoTrack) throw new Error(`Track ${track.trackId} not found`);

        const queueTrack = toQueueTrack(demoTrack);
        getManagerOrThrow().addToQueueNext(queueTrack);
      },

      async addToQueueEnd(track) {
        const demoTrack = storage.getTrackById(track.trackId);
        if (!demoTrack) throw new Error(`Track ${track.trackId} not found`);

        const queueTrack = toQueueTrack(demoTrack);
        getManagerOrThrow().addToQueueEnd(queueTrack);
      },

      async clearPlayNext() {
        getManagerOrThrow().clearPlayNext();
      },

      async clearAddToQueue() {
        getManagerOrThrow().clearAddToQueue();
      },

      async getAllSources() {
        // Web: Return mock sources (can be overridden by subclasses)
        return [
          {
            id: 1,
            name: 'Demo Library',
            sourceType: 'local',
            isActive: true,
            isOnline: true,
          },
        ];
      },
    };

    // Events implementation using WASM manager event emitter
    const events: PlaybackEventsInterface = {
      onStateChange(callback) {
        // Defensive: if not initialized yet, return no-op unsubscribe
        const manager = managerRef.current;
        if (!manager || !isInitialized) {
          console.warn('[WebPlaybackProvider] onStateChange called before initialization, returning no-op');
          return () => {};
        }
        const handler = (state: PlaybackState) => {
          callback(state === PlaybackState.Playing);
        };
        return manager.on('stateChange', handler);
      },

      onTrackChange(callback) {
        const manager = managerRef.current;
        if (!manager || !isInitialized) {
          console.warn('[WebPlaybackProvider] onTrackChange called before initialization, returning no-op');
          return () => {};
        }
        return manager.on('trackChange', callback);
      },

      onPositionUpdate(callback) {
        const manager = managerRef.current;
        if (!manager || !isInitialized) {
          console.warn('[WebPlaybackProvider] onPositionUpdate called before initialization, returning no-op');
          return () => {};
        }
        return manager.on('positionUpdate', callback);
      },

      onVolumeChange(callback) {
        const manager = managerRef.current;
        if (!manager || !isInitialized) {
          console.warn('[WebPlaybackProvider] onVolumeChange called before initialization, returning no-op');
          return () => {};
        }
        return manager.on('volumeChange', callback);
      },

      onQueueUpdate(callback) {
        const manager = managerRef.current;
        if (!manager || !isInitialized) {
          console.warn('[WebPlaybackProvider] onQueueUpdate called before initialization, returning no-op');
          return () => {};
        }
        return manager.on('queueChange', callback);
      },

      onError(callback) {
        const manager = managerRef.current;
        if (!manager || !isInitialized) {
          console.warn('[WebPlaybackProvider] onError called before initialization, returning no-op');
          return () => {};
        }
        return manager.on('error', callback);
      },
    };

    return { commands, events };
  }, [storage, isInitialized, updateSession, clearSession]);

  // Don't render children until WASM is initialized
  if (!isInitialized) {
    console.log('[WebPlaybackProvider] Not initialized yet, returning null');
    return null;
  }

  console.log('[WebPlaybackProvider] Initialized, rendering children');
  return <PlayerCommandsProvider value={value}>{children}</PlayerCommandsProvider>;
}

/**
 * Setup event bridge between WASM manager and shared Zustand store
 * This keeps the store in sync with playback events
 * PlaybackSession context derives from store, so no dual updates needed
 */
function setupEventBridge(manager: WasmPlaybackAdapter, storage: PlaybackDataStorage) {
  console.log('[WebPlaybackProvider] Setting up event bridge');

  // Bridge WASM events to shared store (session context derives from store)
  manager.on('stateChange', (state: PlaybackState) => {
    console.log('[WebPlaybackProvider] State change:', state);
    const isPlaying = state === PlaybackState.Playing;
    usePlayerStore.setState({ isPlaying });
  });

  manager.on('trackChange', (track) => {
    if (track) {
      // Convert WASM QueueTrack to shared Track format
      const trackId = Number(track.id);

      // Look up cover URL from storage (WASM doesn't store it)
      const demoTrack = storage.getTrackById(track.id);
      const coverUrl = demoTrack?.coverUrl || undefined;

      console.log('[WebPlaybackProvider] Track changed:', {
        id: track.id,
        convertedId: trackId,
        title: track.title,
        coverUrl: coverUrl,
      });

      const sharedTrack: Track = {
        id: trackId,
        title: track.title,
        artist: track.artist,
        album: track.album || '',
        duration: Math.floor(track.duration_secs || 0),
        filePath: track.path,
        coverArtPath: coverUrl,
        addedAt: new Date().toISOString(),
      };

      usePlayerStore.setState({ currentTrack: sharedTrack, duration: track.duration_secs || 0 });
    } else {
      console.log('[WebPlaybackProvider] Track cleared');
      usePlayerStore.setState({ currentTrack: null, duration: 0 });
    }
  });

  manager.on('positionUpdate', (position: number) => {
    const duration = manager.getDuration();
    if (duration > 0) {
      const progress = (position / duration) * 100;
      usePlayerStore.setState({ progress });
    }
  });

  manager.on('volumeChange', (volume: number) => {
    console.log('[WebPlaybackProvider] Volume change:', volume, '-> store:', volume / 100);
    usePlayerStore.setState({ volume: volume / 100 }); // 0-100 to 0-1
  });

  manager.on('shuffleChange', (mode: string) => {
    // mode is ShuffleMode enum string: 'off' | 'random' | 'smart'
    const shuffleMode = mode === 'off' ? 'off' : mode === 'random' ? 'random' : 'smart';
    usePlayerStore.setState({ shuffleMode: shuffleMode as 'off' | 'random' | 'smart' });
  });

  manager.on('repeatChange', (mode: string) => {
    // mode is RepeatMode enum string: 'off' | 'all' | 'one'
    const repeatMap: Record<string, 'off' | 'all' | 'one'> = {
      off: 'off',
      all: 'all',
      one: 'one',
    };
    usePlayerStore.setState({ repeatMode: repeatMap[mode] || 'off' });
  });

  // Sync queue to store when it changes
  manager.on('queueChange', () => {
    const wasmQueue = manager.getQueue();
    const tracks: Track[] = wasmQueue.map((queueTrack) => {
      const demoTrack = storage.getTrackById(queueTrack.id);
      return {
        id: Number(queueTrack.id),
        title: queueTrack.title,
        artist: queueTrack.artist,
        album: queueTrack.album || '',
        duration: Math.floor(queueTrack.duration_secs || 0),
        filePath: queueTrack.path,
        coverArtPath: demoTrack?.coverUrl,
        addedAt: new Date().toISOString(),
      };
    });
    console.log('[WebPlaybackProvider] Queue change, syncing', tracks.length, 'tracks to store');
    usePlayerStore.setState({ queue: tracks });
  });
}
