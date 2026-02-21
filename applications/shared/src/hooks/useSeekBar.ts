import { useCallback } from 'react';
import { usePlayerCommands } from '../contexts/PlayerCommandsContext';
import { usePlayerStore } from '../stores/player';
import { debug } from '../utils/debug';

interface UseSeekBarReturn {
  handleSeek: (position: number) => void;
}

/**
 * Seek hook used by ProgressBar.
 *
 * 1. Optimistic UI update — writes to the store immediately so
 *    useInterpolatedProgress snaps to the new position in the same frame.
 * 2. Fire-and-forget backend seek — Tauri/WASM picks it up asynchronously.
 * 3. Backend position events naturally re-sync the store after seek completes.
 */
export function useSeekBar(): UseSeekBarReturn {
  const commands = usePlayerCommands();

  const handleSeek = useCallback((position: number) => {
    const { duration } = usePlayerStore.getState();

    // Clamp to valid range (0.1s buffer avoids triggering EOF)
    const clampedPosition = Math.max(0, Math.min(position, duration - 0.1));

    // Optimistic update — bump seekVersion so useInterpolatedProgress knows this is
    // a user seek (not a backend position update) and can ignore stale backend events
    // that arrive during the seek operation.
    const progressPercentage = duration > 0 ? (clampedPosition / duration) * 100 : 0;
    usePlayerStore.setState({
      progress: progressPercentage,
      seekVersion: usePlayerStore.getState().seekVersion + 1,
      seekTarget: progressPercentage,
    });

    commands.seek(clampedPosition).catch((error) => {
      debug.error('[useSeekBar] Seek failed:', error);
    });
  }, [commands]);

  return { handleSeek };
}
