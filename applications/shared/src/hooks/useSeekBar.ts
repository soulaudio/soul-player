import { useCallback } from 'react';
import { usePlayerCommands } from '../contexts/PlayerCommandsContext';
import { usePlayerStore } from '../stores/player';
import { debug } from '../utils/debug';

interface UseSeekBarReturn {
  handleSeek: (position: number) => void;
  isSeeking: boolean; // Always false now (kept for API compatibility)
}

/**
 * Hook to manage seek interactions - SIMPLIFIED production pattern.
 *
 * Pattern used by VLC, Clementine, Audacious (50-150ms latency):
 * 1. Optimistic UI update (instant visual feedback)
 * 2. Send seek command to backend (async, fire-and-forget)
 * 3. Backend position updates naturally sync after completion
 *
 * No ignore windows, no timers, no complex state - just works.
 * Supports both click-to-seek and drag-to-seek (on release).
 */
export function useSeekBar(): UseSeekBarReturn {
  const commands = usePlayerCommands();

  const handleSeek = useCallback((position: number) => {
    const seekStartTime = performance.now();
    console.log(`[SEEK PERF] ===== SEEK START ===== at ${seekStartTime.toFixed(2)}ms`);

    const { duration } = usePlayerStore.getState();

    // Clamp position to valid range (leave 0.1s buffer to avoid EOF)
    const clampedPosition = Math.max(0, Math.min(position, duration - 0.1));
    console.log(`[SEEK PERF] Target: ${clampedPosition.toFixed(3)}s`);

    // Optimistic UI update (instant feedback - this is the key to instant feel)
    const progressPercentage = duration > 0 ? (clampedPosition / duration) * 100 : 0;
    usePlayerStore.setState({ progress: progressPercentage });
    console.log(`[SEEK PERF] UI updated to ${progressPercentage.toFixed(1)}% in ${(performance.now() - seekStartTime).toFixed(2)}ms`);

    // Send to backend (fire-and-forget, position updates will sync naturally)
    commands.seek(clampedPosition)
      .then(() => {
        const totalTime = performance.now() - seekStartTime;
        console.log(`[SEEK PERF] ===== SEEK COMPLETE ===== (${totalTime.toFixed(2)}ms total)`);
      })
      .catch((error) => {
        console.error(`[SEEK PERF] SEEK FAILED:`, error);
        debug.error('[useSeekBar] Seek failed:', error);
      });
  }, [commands]);

  // Return false for isSeeking (kept for API compatibility, not needed anymore)
  return { handleSeek, isSeeking: false };
}
