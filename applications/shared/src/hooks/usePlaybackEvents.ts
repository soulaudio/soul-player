import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { usePlayerStore } from '../stores/player';
import type { Track } from '../types';
import { debug } from '../utils/debug';

// Note: Position update ignore window is now handled in TauriPlayerCommandsProvider

/**
 * Hook to subscribe to Tauri playback events and update the player store.
 * Should be used once at the top level of the app (typically in PlayerFooter).
 */
export function usePlaybackEvents() {
  useEffect(() => {
    const unlistenFunctions: (() => void)[] = [];
    let isMounted = true;

    const setupListeners = async () => {
      try {
        if (!isMounted) return;

        // Listen for playback state changes (Playing, Paused, Stopped)
        const unlistenStateChanged = await listen<string>('playback:state-changed', (event) => {
          if (!isMounted) return;
          const state = event.payload;

          // Map backend state to isPlaying boolean
          const isPlaying = state === 'Playing';
          usePlayerStore.getState().setIsPlaying(isPlaying);
        });
        unlistenFunctions.push(unlistenStateChanged);

        // Listen for position updates (in seconds)
        const unlistenPositionUpdated = await listen<number>('playback:position-updated', (event) => {
          if (!isMounted) return;
          // Note: Ignore window for seek race condition is handled in TauriPlayerCommandsProvider

          const positionInSeconds = event.payload;

          // Guard: IPC can deliver undefined if the backend serialises an absent field.
          // NaN would propagate through the percentage formula and corrupt the store.
          if (typeof positionInSeconds !== 'number' || !isFinite(positionInSeconds)) return;

          const { duration } = usePlayerStore.getState();

          // Convert position to percentage (0-100)
          const progressPercentage = duration > 0
            ? Math.min(100, (positionInSeconds / duration) * 100)
            : 0;

          usePlayerStore.getState().setProgress(progressPercentage);
        });
        unlistenFunctions.push(unlistenPositionUpdated);

        // Listen for track changes
        const unlistenTrackChanged = await listen<Track | null>('playback:track-changed', (event) => {
          if (!isMounted) return;
          const track = event.payload;

          usePlayerStore.getState().setCurrentTrack(track);

          // If track has duration, update the store
          if (track?.duration) {
            usePlayerStore.getState().setDuration(track.duration);
          } else {
            usePlayerStore.getState().setDuration(0);
          }

          // Reset progress when track changes
          usePlayerStore.getState().setProgress(0);

          // When the track becomes null the queue has ended — ensure isPlaying is cleared
          // so the play/pause button reflects the stopped state.
          if (track === null) {
            usePlayerStore.getState().setIsPlaying(false);
          }
        });
        unlistenFunctions.push(unlistenTrackChanged);

        // Listen for volume changes (0-100)
        const unlistenVolumeChanged = await listen<number>('playback:volume-changed', (event) => {
          if (!isMounted) return;
          const volume = event.payload;

          // Convert from 0-100 to 0.0-1.0
          usePlayerStore.getState().setVolume(volume / 100);
        });
        unlistenFunctions.push(unlistenVolumeChanged);

        // Listen for queue updates
        const unlistenQueueUpdated = await listen('playback:queue-updated', () => {
          if (!isMounted) return;
          // TODO: Implement queue synchronization - fetch updated queue from backend
          // when queue management (add to queue, play next, reorder) is implemented.
          // For now, queue updates are handled via React Query invalidation.
        });
        unlistenFunctions.push(unlistenQueueUpdated);

        // Listen for playback errors
        const unlistenError = await listen<string>('playback:error', (event) => {
          if (!isMounted) return;
          debug.error('[Playback Error]', event.payload);
          // TODO: Show user-facing error notification (toast/snackbar)
          // Currently errors are only logged to console for debugging.
        });
        unlistenFunctions.push(unlistenError);
      } catch (error) {
        debug.error('[usePlaybackEvents] Failed to set up listeners:', error);
      }
    };

    // Setup listeners asynchronously.
    // We keep a reference to the promise so that the cleanup function can await it
    // before invoking the unlisten callbacks — otherwise listeners registered after
    // the synchronous cleanup call would leak.
    const setupPromise = setupListeners();

    // Cleanup: Unsubscribe from all events on unmount
    return () => {
      isMounted = false;
      // Wait for setup to finish before calling unlisten, so all registered listeners
      // are properly removed even if unmount races with the async registration.
      setupPromise.then(() => {
        unlistenFunctions.forEach(fn => fn());
      }).catch(() => {
        // setup already logged any error; still clean up whatever was registered
        unlistenFunctions.forEach(fn => fn());
      });
    };
  }, []); // Empty dependency array - setup once on mount
}
