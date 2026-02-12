import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { usePlayerStore } from '../stores/player';
import { shouldIgnorePositionUpdates } from './useSeekBar';
import type { Track } from '../types';
import { debug } from '../utils/debug';

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
          // Ignore position updates if we're currently seeking
          // This prevents the seek bar from jumping back due to race conditions
          if (shouldIgnorePositionUpdates()) {
            return;
          }

          const positionInSeconds = event.payload;
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
          // TODO: Fetch updated queue from backend when queue management is implemented
        });
        unlistenFunctions.push(unlistenQueueUpdated);

        // Listen for playback errors
        const unlistenError = await listen<string>('playback:error', (event) => {
          if (!isMounted) return;
          debug.error('[Playback Error]', event.payload);
          // TODO: Show error notification to user
        });
        unlistenFunctions.push(unlistenError);
      } catch (error) {
        console.error('[usePlaybackEvents] Failed to set up listeners:', error);
      }
    };

    // Setup listeners asynchronously
    void setupListeners();

    // Cleanup: Unsubscribe from all events on unmount
    return () => {
      isMounted = false;
      unlistenFunctions.forEach(fn => fn());
    };
  }, []); // Empty dependency array - setup once on mount
}
