import { useCallback, useState, useEffect } from 'react';
import { usePlayerCommands } from '../contexts/PlayerCommandsContext';
import { usePlayerStore } from '../stores/player';
import { debug } from '../utils/debug';

interface UseSeekBarReturn {
  handleSeek: (position: number) => void;
  isSeeking: boolean;
}

// Hardcoded ignore window - should match TauriPlayerCommandsProvider
const IGNORE_WINDOW_MS = 50;

/**
 * Hook to manage click-to-seek interactions with visual feedback.
 *
 * Simple implementation:
 * - Click triggers immediate seek
 * - Optimistic UI update (instant feedback)
 * - Ignore window handled in PlayerCommandsProvider
 * - Performance logging enabled
 * - Tracks seeking state for UI feedback
 */
export function useSeekBar(): UseSeekBarReturn {
  const commands = usePlayerCommands();
  const [isSeeking, setIsSeeking] = useState(false);

  const handleSeek = useCallback((position: number) => {
    const t0 = performance.now();
    const { duration } = usePlayerStore.getState();

    // Clamp position to valid range
    const clampedPosition = Math.max(0, Math.min(position, duration - 0.1));

    debug.log('[useSeekBar] Seek to:', clampedPosition);

    // Set seeking state for visual feedback
    setIsSeeking(true);

    // Optimistic UI update (instant feedback)
    const progressPercentage = duration > 0
      ? (clampedPosition / duration) * 100
      : 0;

    const t1 = performance.now();
    usePlayerStore.setState({ progress: progressPercentage });
    console.log(`[SEEK PERF] Store updated in ${(t1 - t0).toFixed(1)}ms`);

    // Send to backend (ignore window handled by provider)
    commands.seek(clampedPosition)
      .then(() => {
        const t2 = performance.now();
        console.log(`[SEEK PERF] Backend completed in ${(t2 - t0).toFixed(1)}ms`);
      })
      .catch((error) => {
        debug.error('[useSeekBar] Seek failed:', error);
      })
      .finally(() => {
        // Clear seeking state after ignore window + small buffer
        setTimeout(() => setIsSeeking(false), IGNORE_WINDOW_MS + 50);
      });
  }, [commands]);

  return { handleSeek, isSeeking };
}
