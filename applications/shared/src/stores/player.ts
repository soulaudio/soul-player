import { create } from 'zustand';
import type { Track } from '../types';

interface PlayerState {
  // Current playback
  currentTrack: Track | null;
  isPlaying: boolean;
  volume: number; // 0.0 to 1.0
  previousVolume: number; // For mute toggle restore
  progress: number; // 0 to 100
  duration: number; // seconds
  seekVersion: number; // incremented on every user-initiated seek
  seekTarget: number;  // progress (0-100) the user seeked to; only written by useSeekBar

  // Queue
  queue: Track[];
  queueIndex: number;

  // Repeat & Shuffle
  repeatMode: 'off' | 'all' | 'one';
  shuffleMode: 'off' | 'random' | 'smart';

  // Actions
  setCurrentTrack: (track: Track | null) => void;
  setIsPlaying: (isPlaying: boolean) => void;
  setVolume: (volume: number) => void;
  setProgress: (progress: number) => void;
  setDuration: (duration: number) => void;

  // Queue management (read-only, updated via commands → backend → event bridge)
  setQueue: (tracks: Track[]) => void;

  // Settings (optimistic updates for UI responsiveness)
  setRepeatMode: (mode: 'off' | 'all' | 'one') => void;
  setShuffleMode: (mode: 'off' | 'random' | 'smart') => void;
}

export const usePlayerStore = create<PlayerState>((set) => ({
  // Initial state
  currentTrack: null,
  isPlaying: false,
  volume: 0.8,
  previousVolume: 0.8,
  progress: 0,
  duration: 0,
  seekVersion: 0,
  seekTarget: 0,
  queue: [],
  queueIndex: -1,
  repeatMode: 'off',
  shuffleMode: 'off',

  // Actions
  setCurrentTrack: (track) => set({ currentTrack: track }),
  setIsPlaying: (isPlaying) => set({ isPlaying }),
  setVolume: (volume) => set({ volume: Math.max(0, Math.min(1, volume)) }),
  setProgress: (progress) => set({ progress: Math.max(0, Math.min(100, progress)) }),
  setDuration: (duration) => set({ duration }),

  // Queue management (read-only, updated via commands → backend → event bridge)
  setQueue: (tracks) => set({ queue: tracks, queueIndex: tracks.length > 0 ? 0 : -1 }),

  // Settings (optimistic updates for UI responsiveness)
  setRepeatMode: (mode) => set({ repeatMode: mode }),

  setShuffleMode: (mode) => set({ shuffleMode: mode }),
}));

// =============================================================================
// Optimized Selector Hooks (Performance Enhancement)
// =============================================================================
// These selector hooks prevent unnecessary re-renders by subscribing to only
// the specific state values that a component needs, instead of the entire store.
//
// Example: A component using useCurrentTrack() will ONLY re-render when
// currentTrack changes, not when volume, progress, or any other state changes.
//
// Before: const { currentTrack } = usePlayerStore() → re-renders on ANY state change
// After: const currentTrack = useCurrentTrack() → re-renders ONLY on currentTrack change
// =============================================================================

export const useCurrentTrack = () => usePlayerStore(state => state.currentTrack);
export const useIsPlaying = () => usePlayerStore(state => state.isPlaying);
export const useProgress = () => usePlayerStore(state => state.progress);
export const useDuration = () => usePlayerStore(state => state.duration);
export const useVolume = () => usePlayerStore(state => state.volume);
export const useShuffleMode = () => usePlayerStore(state => state.shuffleMode);
export const useRepeatMode = () => usePlayerStore(state => state.repeatMode);
export const useQueue = () => usePlayerStore(state => state.queue);
export const useQueueIndex = () => usePlayerStore(state => state.queueIndex);

// Composite selectors for components that need multiple values
// Still better than full store subscription
export const usePlayerPlayback = () => usePlayerStore(state => ({
  currentTrack: state.currentTrack,
  isPlaying: state.isPlaying,
}));

export const usePlayerProgress = () => usePlayerStore(state => ({
  progress: state.progress,
  duration: state.duration,
}));

export const usePlayerModes = () => usePlayerStore(state => ({
  shuffleMode: state.shuffleMode,
  repeatMode: state.repeatMode,
  setShuffleMode: state.setShuffleMode,
  setRepeatMode: state.setRepeatMode,
}));
