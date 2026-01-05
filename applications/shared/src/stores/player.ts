import { create } from 'zustand';
import type { Track } from '../types';

interface PlayerState {
  // Current playback
  currentTrack: Track | null;
  isPlaying: boolean;
  volume: number; // 0.0 to 1.0
  progress: number; // 0 to 100
  duration: number; // seconds

  // Queue
  queue: Track[];
  queueIndex: number;

  // Repeat & Shuffle
  repeatMode: 'off' | 'all' | 'one';
  shuffleEnabled: boolean;

  // Actions
  setCurrentTrack: (track: Track | null) => void;
  setIsPlaying: (isPlaying: boolean) => void;
  setVolume: (volume: number) => void;
  setProgress: (progress: number) => void;
  setDuration: (duration: number) => void;

  // Queue management
  setQueue: (tracks: Track[]) => void;
  addToQueue: (tracks: Track | Track[]) => void;
  removeFromQueue: (index: number) => void;
  clearQueue: () => void;
  playNext: () => void;
  playPrevious: () => void;

  // Settings
  setRepeatMode: (mode: 'off' | 'all' | 'one') => void;
  toggleShuffle: () => void;
}

export const usePlayerStore = create<PlayerState>((set, get) => ({
  // Initial state
  currentTrack: null,
  isPlaying: false,
  volume: 0.8,
  progress: 0,
  duration: 0,
  queue: [],
  queueIndex: -1,
  repeatMode: 'off',
  shuffleEnabled: false,

  // Actions
  setCurrentTrack: (track) => set({ currentTrack: track }),
  setIsPlaying: (isPlaying) => set({ isPlaying }),
  setVolume: (volume) => set({ volume: Math.max(0, Math.min(1, volume)) }),
  setProgress: (progress) => set({ progress: Math.max(0, Math.min(100, progress)) }),
  setDuration: (duration) => set({ duration }),

  // Queue management
  setQueue: (tracks) => set({ queue: tracks, queueIndex: tracks.length > 0 ? 0 : -1 }),

  addToQueue: (tracks) => {
    const tracksArray = Array.isArray(tracks) ? tracks : [tracks];
    set((state) => ({
      queue: [...state.queue, ...tracksArray],
    }));
  },

  removeFromQueue: (index) =>
    set((state) => ({
      queue: state.queue.filter((_, i) => i !== index),
      queueIndex:
        index < state.queueIndex
          ? state.queueIndex - 1
          : index === state.queueIndex
          ? Math.min(state.queueIndex, state.queue.length - 2)
          : state.queueIndex,
    })),

  clearQueue: () => set({ queue: [], queueIndex: -1, currentTrack: null }),

  playNext: () => {
    const state = get();
    const { queue, queueIndex, repeatMode } = state;

    if (queue.length === 0) return;

    let nextIndex = queueIndex + 1;

    if (nextIndex >= queue.length) {
      if (repeatMode === 'all') {
        nextIndex = 0;
      } else {
        // End of queue
        set({ isPlaying: false });
        return;
      }
    }

    set({
      queueIndex: nextIndex,
      currentTrack: queue[nextIndex],
      progress: 0,
    });
  },

  playPrevious: () => {
    const state = get();
    const { queue, queueIndex, progress } = state;

    if (queue.length === 0) return;

    // If more than 3 seconds into song, restart it
    if (progress > 3) {
      set({ progress: 0 });
      return;
    }

    let prevIndex = queueIndex - 1;

    if (prevIndex < 0) {
      if (state.repeatMode === 'all') {
        prevIndex = queue.length - 1;
      } else {
        prevIndex = 0;
      }
    }

    set({
      queueIndex: prevIndex,
      currentTrack: queue[prevIndex],
      progress: 0,
    });
  },

  setRepeatMode: (mode) => set({ repeatMode: mode }),

  toggleShuffle: () => set((state) => ({ shuffleEnabled: !state.shuffleEnabled })),
}));
