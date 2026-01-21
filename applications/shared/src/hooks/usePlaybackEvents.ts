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
    // Listen for playback state changes (Playing, Paused, Stopped, Loading)
    const unlistenStateChanged = listen<string>('playback:state-changed', (event) => {
      const state = event.payload;

      // Map backend state to isPlaying boolean
      const isPlaying = state === 'Playing';
      usePlayerStore.getState().setIsPlaying(isPlaying);
    });

    // Listen for position updates (in seconds)
    const unlistenPositionUpdated = listen<number>('playback:position-updated', (event) => {
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

    // Listen for track changes
    const unlistenTrackChanged = listen<Track | null>('playback:track-changed', (event) => {
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

    // Listen for volume changes (0-100)
    const unlistenVolumeChanged = listen<number>('playback:volume-changed', (event) => {
      const volume = event.payload;

      // Convert from 0-100 to 0.0-1.0
      usePlayerStore.getState().setVolume(volume / 100);
    });

    // Listen for queue updates
    const unlistenQueueUpdated = listen('playback:queue-updated', () => {
      // TODO: Fetch updated queue from backend when queue management is implemented
    });

    // Listen for playback errors
    const unlistenError = listen<string>('playback:error', (event) => {
      debug.error('[Playback Error]', event.payload);
      // TODO: Show error notification to user
    });

    // Cleanup: Unsubscribe from all events on unmount
    return () => {
      unlistenStateChanged.then((fn) => fn());
      unlistenPositionUpdated.then((fn) => fn());
      unlistenTrackChanged.then((fn) => fn());
      unlistenVolumeChanged.then((fn) => fn());
      unlistenQueueUpdated.then((fn) => fn());
      unlistenError.then((fn) => fn());
    };
  }, []); // Empty dependency array - setup once on mount
}
