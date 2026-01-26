/**
 * Demo implementation of PlayerCommands context
 * Provides a no-op PlayerCommandsProvider for non-interactive demos
 */

import { ReactNode, useMemo } from 'react';
import { PlayerCommandsProvider, type PlayerContextValue, type PlayerCommandsInterface, type PlaybackEventsInterface } from '@soul-player/shared';

interface DemoPlayerCommandsProviderProps {
  storage?: unknown;
  children: ReactNode;
}

/**
 * Simple mock player commands provider for showcase demos
 * All commands are no-ops since the demo is non-interactive
 */
export function DemoPlayerCommandsProvider({ children }: DemoPlayerCommandsProviderProps) {
  const value = useMemo<PlayerContextValue>(() => {
    // Complete no-op implementation of PlayerCommandsInterface
    const commands: PlayerCommandsInterface = {
      // Playback control
      async playTrack() {},
      async pausePlayback() {},
      async resumePlayback() {},
      async stopPlayback() {},

      // Navigation
      async skipNext() {},
      async skipPrevious() {},

      // Seek and volume
      async seek() {},
      async setVolume() {},

      // Shuffle and repeat
      async setShuffle() {},
      async cycleShuffle() { return 'off'; },
      async getShuffle() { return 'off'; },
      async setRepeatMode() {},
      async cycleRepeat() { return 'off'; },
      async getRepeat() { return 'off'; },

      // Capabilities
      async getPlaybackCapabilities() {
        return { hasNext: false, hasPrevious: false };
      },

      // Queue management
      async getQueue() { return []; },
      async playQueue() {},
      async playQueueWithContext() {},
      async skipToQueueIndex() {},

      // Three-tier queue operations
      async addPlayNext() {},
      async addToQueueEnd() {},
      async clearPlayNext() {},
      async clearAddToQueue() {},

      // Sources management
      async getAllSources() { return []; },
    };

    // No-op events that return empty cleanup functions
    const events: PlaybackEventsInterface = {
      onStateChange: () => () => {},
      onTrackChange: () => () => {},
      onPositionUpdate: () => () => {},
      onVolumeChange: () => () => {},
      onQueueUpdate: () => () => {},
      onError: () => () => {},
    };

    return { commands, events };
  }, []);

  return <PlayerCommandsProvider value={value}>{children}</PlayerCommandsProvider>;
}
