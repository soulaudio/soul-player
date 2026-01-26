/**
 * Demo implementation of PlaybackContextProvider
 * Provides a no-op PlaybackContextProvider for non-interactive demos
 */

import { ReactNode, useMemo, createContext, useContext } from 'react';
import type { PlaybackContextData } from '@soul-player/shared';

interface DemoPlaybackContextProviderProps {
  children: ReactNode;
}

const PlaybackContextContext = createContext<PlaybackContextData | null>(null);

export function usePlaybackContext(): PlaybackContextData {
  const context = useContext(PlaybackContextContext);
  if (!context) {
    throw new Error('usePlaybackContext must be used within PlaybackContextProvider');
  }
  return context;
}

/**
 * Simple mock playback context provider for showcase demos
 * Returns empty contexts since the demo is non-interactive
 */
export function DemoPlaybackContextProvider({ children }: DemoPlaybackContextProviderProps) {
  const value = useMemo<PlaybackContextData>(() => {
    return {
      // Always return false - no active contexts in demo
      isActiveContext: () => false,
      contexts: [],
      isLoading: false,
    };
  }, []);

  return (
    <PlaybackContextContext.Provider value={value}>
      {children}
    </PlaybackContextContext.Provider>
  );
}
