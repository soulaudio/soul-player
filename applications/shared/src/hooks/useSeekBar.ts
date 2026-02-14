import { useCallback, useState } from 'react';
import { usePlayerCommands } from '../contexts/PlayerCommandsContext';
import { usePlayerStore } from '../stores/player';
import { debug } from '../utils/debug';

interface UseSeekBarReturn {
  handleSeek: (position: number) => void;
  isSeeking: boolean;
}

// Ignore window duration - matches TauriPlayerCommandsProvider
const SEEK_FEEDBACK_DURATION_MS = 100;

/**
 * Hook to manage seek interactions with visual feedback.
 *
 * Pattern based on industry standards (react-h5-audio-player, wavesurfer.js):
 * 1. Optimistic UI update (instant visual feedback)
 * 2. Send seek command to backend (async)
 * 3. Ignore position updates during window (handled by provider)
 * 4. Clear seeking state after feedback duration
 *
 * Supports both click-to-seek and drag-to-seek (on release).
 */
export function useSeekBar(): UseSeekBarReturn {
  const commands = usePlayerCommands();
  const [isSeeking, setIsSeeking] = useState(false);

  const handleSeek = useCallback((position: number) => {
    const { duration } = usePlayerStore.getState();

    // Clamp position to valid range (leave 0.1s buffer to avoid EOF)
    const clampedPosition = Math.max(0, Math.min(position, duration - 0.1));

    debug.log(`[useSeekBar] Seeking to ${clampedPosition.toFixed(2)}s`);

    // Set seeking state for visual feedback
    setIsSeeking(true);

    // 1. Optimistic UI update (instant feedback)
    const progressPercentage = duration > 0
      ? (clampedPosition / duration) * 100
      : 0;
    usePlayerStore.setState({ progress: progressPercentage });

    // 2. Send to backend (async)
    commands.seek(clampedPosition)
      .catch((error) => {
        debug.error('[useSeekBar] Seek failed:', error);
      })
      .finally(() => {
        // Clear seeking state after visual feedback duration
        setTimeout(() => setIsSeeking(false), SEEK_FEEDBACK_DURATION_MS);
      });
  }, [commands]);

  return { handleSeek, isSeeking };
}
