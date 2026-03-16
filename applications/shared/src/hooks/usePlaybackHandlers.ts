import { useCallback } from 'react';
import { usePlayerCommands } from '../contexts/PlayerCommandsContext';
import { usePlayerStore } from '../stores/player';
import { debug } from '../utils/debug';
import type { ShuffleMode, RepeatMode } from '../components/sidebar/PlaybackControls';

export interface PlaybackHandlers {
  onPlayPause: () => Promise<void>;
  onNext: () => Promise<void>;
  onPrevious: () => Promise<void>;
  /**
   * Cycles shuffle mode via backend, then writes the returned mode directly
   * to the Zustand store. Does NOT use a prop callback — owns the store write.
   */
  onShuffleToggle: () => Promise<void>;
  /**
   * Cycles repeat mode with optimistic update + rollback on error.
   * Writes directly to the Zustand store — does NOT use a prop callback.
   */
  onRepeatToggle: () => Promise<void>;
}

/**
 * Shared hook for playback control handlers.
 * Consumed by both PlayerPanel and NowPlayingFloating so the logic
 * is not duplicated. Uses usePlayerCommands() and writes directly to
 * Zustand store (usePlayerStore.getState()) for shuffle/repeat.
 */
export function usePlaybackHandlers(): PlaybackHandlers {
  const commands = usePlayerCommands();

  const onPlayPause = useCallback(async () => {
    try {
      const { isPlaying } = usePlayerStore.getState();
      if (isPlaying) {
        await commands.pausePlayback();
      } else {
        await commands.resumePlayback();
      }
    } catch (error) {
      debug.error('[usePlaybackHandlers] Failed to toggle playback:', error);
    }
  }, [commands]);

  const onNext = useCallback(async () => {
    try {
      await commands.skipNext();
    } catch (error) {
      debug.error('[usePlaybackHandlers] Failed to skip next:', error);
    }
  }, [commands]);

  const onPrevious = useCallback(async () => {
    try {
      await commands.skipPrevious();
    } catch (error) {
      debug.error('[usePlaybackHandlers] Failed to skip previous:', error);
    }
  }, [commands]);

  const onShuffleToggle = useCallback(async () => {
    try {
      const newMode = (await commands.cycleShuffle()) as ShuffleMode;
      usePlayerStore.getState().setShuffleMode(newMode);
    } catch (error) {
      debug.error('[usePlaybackHandlers] Failed to cycle shuffle:', error);
    }
  }, [commands]);

  const onRepeatToggle = useCallback(async () => {
    const { repeatMode } = usePlayerStore.getState();
    const currentMode = repeatMode as RepeatMode;
    const nextMode: RepeatMode =
      currentMode === 'off' ? 'all' : currentMode === 'all' ? 'one' : 'off';
    // Optimistic update
    usePlayerStore.getState().setRepeatMode(nextMode);
    try {
      await commands.setRepeatMode(nextMode);
    } catch (error) {
      debug.error('[usePlaybackHandlers] Failed to set repeat mode, rolling back:', error);
      // Rollback to the pre-toggle mode
      usePlayerStore.getState().setRepeatMode(currentMode);
    }
  }, [commands]);

  return { onPlayPause, onNext, onPrevious, onShuffleToggle, onRepeatToggle };
}
