import { create } from 'zustand';
import type { Track, Album, Artist } from '../types';

interface LibraryState {
  // Data
  tracks: Track[];
  albums: Album[];
  artists: Artist[];

  // Loading state
  isLoading: boolean;
  error: string | null;

  // Actions
  setTracks: (tracks: Track[]) => void;
  setAlbums: (albums: Album[]) => void;
  setArtists: (artists: Artist[]) => void;
  setLoading: (isLoading: boolean) => void;
  setError: (error: string | null) => void;

  // Utility
  getTrackById: (id: number) => Track | undefined;
  getAlbumById: (id: number) => Album | undefined;
  getArtistById: (id: number) => Artist | undefined;
  searchTracks: (query: string) => Track[];
}

export const useLibraryStore = create<LibraryState>((set, get) => ({
  // Initial state
  tracks: [],
  albums: [],
  artists: [],
  isLoading: false,
  error: null,

  // Actions
  setTracks: (tracks) => set({ tracks }),
  setAlbums: (albums) => set({ albums }),
  setArtists: (artists) => set({ artists }),
  setLoading: (isLoading) => set({ isLoading }),
  setError: (error) => set({ error }),

  // Utility functions
  getTrackById: (id) => {
    const { tracks } = get();
    return tracks.find((track) => track.id === id);
  },

  getAlbumById: (id) => {
    const { albums } = get();
    return albums.find((album) => album.id === id);
  },

  getArtistById: (id) => {
    const { artists } = get();
    return artists.find((artist) => artist.id === id);
  },

  searchTracks: (query) => {
    const { tracks } = get();
    const lowerQuery = query.toLowerCase();

    return tracks.filter(
      (track) =>
        track.title.toLowerCase().includes(lowerQuery) ||
        track.artist.toLowerCase().includes(lowerQuery) ||
        track.album.toLowerCase().includes(lowerQuery)
    );
  },
}));
