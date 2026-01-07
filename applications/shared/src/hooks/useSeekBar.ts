import { useState, useCallback, useRef } from 'react';
import { playerCommands } from '../lib/tauri';

interface UseSeekBarReturn {
  isDragging: boolean;
  seekPosition: number | null;
  handleSeekStart: (position: number) => void;
  handleSeekChange: (position: number) => void;
  handleSeekEnd: () => void;
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
   */
  const handleSeekEnd = useCallback(() => {
    // Clear any pending debounce timer
    if (debounceTimerRef.current) {
      clearTimeout(debounceTimerRef.current);
      debounceTimerRef.current = null;
    }

    // Send final position to backend
    if (seekPosition !== null) {
      playerCommands.seek(seekPosition)
        .then(() => {
          console.log('[useSeekBar] Seeked to position:', seekPosition);
        })
        .catch((error) => {
          console.error('[useSeekBar] Seek failed:', error);
        });
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
