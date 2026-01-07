import { useState, useCallback, useRef } from 'react';
import { playerCommands } from '../lib/tauri';
import { usePlayerStore } from '../stores/player';

interface UseSeekBarReturn {
  isDragging: boolean;
  seekPosition: number | null;
  handleSeekStart: (position: number) => void;
  handleSeekChange: (position: number) => void;
  handleSeekEnd: (finalPosition?: number) => void;
}

/**
 * Hook to manage seek bar interactions with debouncing.
 * Prevents excessive backend calls while dragging the seek bar.
 *
 * @param debounceMs - Debounce delay in milliseconds (default: 300ms)
 * @returns Seek bar state and handlers
 */
export function useSeekBar(debounceMs: number = 300): UseSeekBarReturn {
  const [isDragging, setIsDragging] = useState(false);
  const [seekPosition, setSeekPosition] = useState<number | null>(null);
  const debounceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  /**
   * Called when user starts dragging the seek bar
   */
  const handleSeekStart = useCallback((position: number) => {
    console.log('[useSeekBar] handleSeekStart:', position);
    setIsDragging(true);
    setSeekPosition(position);
  }, []);

  /**
   * Called continuously while dragging
   * Updates UI immediately but debounces backend calls
   */
  const handleSeekChange = useCallback((position: number) => {
    setSeekPosition(position);

    // Clear existing timer
    if (debounceTimerRef.current) {
      clearTimeout(debounceTimerRef.current);
    }

    // Set new debounced timer
    debounceTimerRef.current = setTimeout(() => {
      // This will only execute if user pauses dragging for debounceMs
      // Currently not sending intermediate updates - only final position on drag end
    }, debounceMs);
  }, [debounceMs]);

  /**
   * Called when user releases the seek bar
   * Sends final position to backend
   * @param finalPosition - Optional position to seek to (if not provided, uses seekPosition state)
   */
  const handleSeekEnd = useCallback((finalPosition?: number) => {
    // Clear any pending debounce timer
    if (debounceTimerRef.current) {
      clearTimeout(debounceTimerRef.current);
      debounceTimerRef.current = null;
    }

    // Use provided position or fall back to state
    const targetPosition = finalPosition ?? seekPosition;

    console.log('[useSeekBar] handleSeekEnd called with:', { finalPosition, seekPosition, targetPosition });

    // Send final position to backend
    if (targetPosition !== null) {
      const { duration } = usePlayerStore.getState();

      console.log('[useSeekBar] Seeking to position:', targetPosition);

      // Set flag to ignore position updates from backend for 500ms
      // This prevents the seek bar from jumping back due to race conditions
      setIgnorePositionUpdates(true);

      // Immediately update the store with the target position
      const progressPercentage = duration > 0
        ? Math.min(100, (targetPosition / duration) * 100)
        : 0;
      usePlayerStore.getState().setProgress(progressPercentage);

      // Send seek command to backend
      playerCommands.seek(targetPosition)
        .then(() => {
          console.log('[useSeekBar] Seek command succeeded');
        })
        .catch((error) => {
          console.error('[useSeekBar] Seek failed:', error);
        });

      // Re-enable position updates after 500ms
      // This gives the backend time to process the seek
      setTimeout(() => {
        setIgnorePositionUpdates(false);
        console.log('[useSeekBar] Re-enabled position updates');
      }, 500);
    } else {
      console.warn('[useSeekBar] handleSeekEnd called but no position available');
    }

    // Reset dragging state
    setIsDragging(false);
    setSeekPosition(null);
  }, [seekPosition]);

  return {
    isDragging,
    seekPosition,
    handleSeekStart,
    handleSeekChange,
    handleSeekEnd,
  };
}

/**
 * Global ref to track if position updates should be ignored
 * Used to prevent seek bar jumping during seek operations
 */
let globalIgnorePositionUpdates = false;

/**
 * Set whether position updates from backend should be ignored
 * @internal Used by useSeekBar
 */
export function setIgnorePositionUpdates(ignore: boolean): void {
  globalIgnorePositionUpdates = ignore;
}

/**
 * Check if position updates should be ignored
 * Used by usePlaybackEvents to prevent race conditions during seek
 */
export function shouldIgnorePositionUpdates(): boolean {
  return globalIgnorePositionUpdates;
}
