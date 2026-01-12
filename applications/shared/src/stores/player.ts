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
