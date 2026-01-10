/**
 * PlayerCommands context - provides platform-agnostic player commands
 * Desktop: Uses Tauri invoke()
 * Marketing demo: Uses web audio API
 */

import { createContext, useContext, ReactNode } from 'react';

export interface PlaybackCapabilities {
  hasNext: boolean;
  hasPrevious: boolean;
}

export interface QueueTrack {
  trackId: string;
  title: string;
  artist: string;
  album: string | null;
  filePath: string;
  durationSeconds: number | null;
  trackNumber: number | null;
  coverArtPath?: string;
}

export interface Source {
  id: number;
  name: string;
  sourceType: string;
  isActive: boolean;
  isOnline: boolean;
}

export interface PlayerCommandsInterface {
  // Playback control
  playTrack: (trackId: string | number) => Promise<void>;
  pausePlayback: () => Promise<void>;
  resumePlayback: () => Promise<void>;
  stopPlayback: () => Promise<void>;

  // Navigation
  skipNext: () => Promise<void>;
  skipPrevious: () => Promise<void>;

  // Seek and volume
  seek: (position: number) => Promise<void>;
  setVolume: (volume: number) => Promise<void>;

  // Shuffle and repeat
  setShuffle: (enabled: boolean) => Promise<void>;
  setRepeatMode: (mode: 'off' | 'all' | 'one') => Promise<void>;

  // Capabilities
  getPlaybackCapabilities: () => Promise<PlaybackCapabilities>;

  // State query (for syncing UI with audio layer)
  getPlaybackState?: () => Promise<string>;

  // Queue management
  getQueue: () => Promise<QueueTrack[]>;
  playQueue: (queue: QueueTrack[], startIndex?: number) => Promise<void>;
  skipToQueueIndex: (index: number) => Promise<void>;

  // Sources management
  getAllSources: () => Promise<Source[]>;

  // Audio device management (Desktop only - optional)
  getCurrentAudioDevice?: () => Promise<any>;
  getAudioBackends?: () => Promise<any[]>;
  getAudioDevices?: (backend: string) => Promise<any[]>;
  setAudioDevice?: (backend: string, deviceName: string) => Promise<void>;
}

export interface PlaybackEventsInterface {
  // Event subscription
  onStateChange: (callback: (isPlaying: boolean) => void) => () => void;
  onTrackChange: (callback: (track: any) => void) => () => void;
  onPositionUpdate: (callback: (position: number) => void) => () => void;
  onVolumeChange: (callback: (volume: number) => void) => () => void;
  onQueueUpdate: (callback: () => void) => () => void;
  onError: (callback: (error: string) => void) => () => void;
}

export interface PlayerContextValue {
  commands: PlayerCommandsInterface;
  events: PlaybackEventsInterface;
}

const PlayerCommandsContext = createContext<PlayerContextValue | null>(null);

export function usePlayerCommands(): PlayerCommandsInterface {
  const context = useContext(PlayerCommandsContext);
  if (!context) {
    throw new Error('usePlayerCommands must be used within PlayerCommandsProvider');
  }
  return context.commands;
}

export function usePlaybackEvents(): PlaybackEventsInterface {
  const context = useContext(PlayerCommandsContext);
  if (!context) {
    throw new Error('usePlaybackEvents must be used within PlayerCommandsProvider');
  }
  return context.events;
}

interface PlayerCommandsProviderProps {
  children: ReactNode;
  value: PlayerContextValue;
}

export function PlayerCommandsProvider({ children, value }: PlayerCommandsProviderProps) {
  return (
    <PlayerCommandsContext.Provider value={value}>
      {children}
    </PlayerCommandsContext.Provider>
  );
}
