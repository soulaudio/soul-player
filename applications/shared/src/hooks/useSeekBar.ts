import { useCallback, useRef, useEffect } from 'react';
import { usePlayerCommands } from '../contexts/PlayerCommandsContext';
import { usePlayerStore } from '../stores/player';
import { debug } from '../utils/debug';
import { usePlaybackTiming } from './usePlaybackTiming';

interface UseSeekBarReturn {
  handleSeek: (position: number) => void;
}

/**
 * Hook to manage click-to-seek interactions.
 * Supports only immediate seeking on click (no drag/scrubbing).
 *
 * Features:
 * - Immediate seek on click
 * - Configurable ignore window to prevent race conditions (synced with backend)
 * - Seek verification after ignore window expires
 * - React ref-based state management (no global variables)
 */
export function useSeekBar(): UseSeekBarReturn {
  const commands = usePlayerCommands();
  const timingConfig = usePlaybackTiming();
  const ignoreUpdatesTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const targetPositionRef = useRef<number | null>(null);
  const isIgnoringUpdatesRef = useRef<boolean>(false);

  // Cleanup timers on unmount
  useEffect(() => {
    return () => {
      if (ignoreUpdatesTimerRef.current) {
        clearTimeout(ignoreUpdatesTimerRef.current);
        ignoreUpdatesTimerRef.current = null;
      }
    };
  }, []);

  /**
   * Handle immediate seek on click
   * @param position - Target position in seconds
   */
  const handleSeek = useCallback((position: number) => {
    const { duration } = usePlayerStore.getState();

    debug.log('[useSeekBar] Seeking to position:', position, 'ignore window:', timingConfig.ignoreWindowMs);

    // Store target position for verification
    targetPositionRef.current = position;

    // Set flag to ignore position updates from backend for configured ignore window
    // This prevents the seek bar from jumping back due to race conditions
    // Window duration is calculated by backend as: position_update_interval * 1.2
    isIgnoringUpdatesRef.current = true;
    updateIgnoreFlag(true);

    // Immediately update the store with the target position
    const progressPercentage = duration > 0
      ? Math.min(100, (position / duration) * 100)
      : 0;
    usePlayerStore.getState().setProgress(progressPercentage);

    // Send seek command to backend
    commands.seek(position)
      .then(() => {
        debug.log('[useSeekBar] Seek command succeeded');
      })
      .catch((error) => {
        debug.error('[useSeekBar] Seek failed:', error);
        // On error, clear the ignore flag immediately
        isIgnoringUpdatesRef.current = false;
        updateIgnoreFlag(false);
        targetPositionRef.current = null;
      });

    // Re-enable position updates after configured ignore window and verify seek completed
    // Clear any existing timer first
    if (ignoreUpdatesTimerRef.current) {
      clearTimeout(ignoreUpdatesTimerRef.current);
    }
    ignoreUpdatesTimerRef.current = setTimeout(() => {
      isIgnoringUpdatesRef.current = false;
      updateIgnoreFlag(false);

      // Verify seek completed by checking current position
      const { progress, duration: currentDuration } = usePlayerStore.getState();
      const currentPosition = (progress / 100) * currentDuration;
      const expectedPosition = targetPositionRef.current;

      if (expectedPosition !== null) {
        const positionDiff = Math.abs(currentPosition - expectedPosition);
        // Allow 0.5s tolerance for seek verification
        if (positionDiff > 0.5) {
          debug.warn('[useSeekBar] Seek verification failed:', {
            expected: expectedPosition,
            actual: currentPosition,
            diff: positionDiff
          });
        } else {
          debug.log('[useSeekBar] Seek verified:', {
            expected: expectedPosition,
            actual: currentPosition
          });
        }
      }

      targetPositionRef.current = null;
      ignoreUpdatesTimerRef.current = null;
      debug.log('[useSeekBar] Re-enabled position updates');
    }, timingConfig.ignoreWindowMs);
  }, [commands, timingConfig]);

  return {
    handleSeek,
  };
}

/**
 * Check if position updates should be ignored
 * Used by event handlers to prevent race conditions during seek
 *
 * IMPORTANT: This uses a shared ref that's accessed via the hook instance.
 * Each component instance has its own ref, but only one ProgressBar should be active.
 */
export function shouldIgnorePositionUpdates(): boolean {
  // Access the ref from the latest hook instance
  // This works because there's only one ProgressBar component active at a time
  return shouldIgnorePositionUpdatesRef.current;
}

/**
 * Shared ref for ignore flag - accessed by shouldIgnorePositionUpdates
 * @internal
 */
const shouldIgnorePositionUpdatesRef = { current: false };

/**
 * Update the shared ignore flag
 * @internal Used by useSeekBar
 */
export function updateIgnoreFlag(value: boolean): void {
  shouldIgnorePositionUpdatesRef.current = value;
}
