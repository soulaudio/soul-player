// Core types for Soul Player
// These should match the Rust types in soul-core

export interface Track {
  id: number;
  title: string;
  artist: string;
  album: string;
  albumArtist?: string;
  duration: number; // seconds
  filePath: string;
  trackNumber?: number;
  discNumber?: number;
  year?: number;
  genre?: string;
  coverArtPath?: string;
  addedAt: string; // ISO 8601
}

export interface Album {
  id: number;
  title: string;
  artist: string;
  year?: number;
  coverArtPath?: string;
  trackCount: number;
}

export interface Artist {
  id: number;
  name: string;
  albumCount: number;
  trackCount: number;
}

export interface Playlist {
  id: number;
  name: string;
  description?: string;
  ownerId: number;
  trackCount: number;
  duration: number; // total seconds
  coverArtPath?: string;
  createdAt: string;
  updatedAt: string;
}

export interface PlaylistTrack {
  playlistId: number;
  trackId: number;
  position: number;
  addedAt: string;
}

export interface User {
  id: number;
  username: string;
  email?: string;
  createdAt: string;
}

// Playback state
export interface PlaybackState {
  currentTrack: Track | null;
  isPlaying: boolean;
  volume: number; // 0.0 to 1.0
  progress: number; // 0 to 100 (percentage)
  duration: number; // seconds
  queue: Track[];
  queueIndex: number;
}

// Settings
export interface AppSettings {
  theme: 'light' | 'dark' | 'system';
  volume: number;
  repeatMode: 'off' | 'all' | 'one';
  shuffleEnabled: boolean;
  libraryPaths: string[];
  serverUrl?: string;
  autoSync: boolean;
}
