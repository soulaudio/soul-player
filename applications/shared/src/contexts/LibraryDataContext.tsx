/**
 * LibraryData context - provides platform-agnostic library data access
 * Desktop: Uses Tauri invoke()
 * Marketing demo: Uses DemoStorage
 */

import { createContext, useContext, ReactNode } from 'react';
import type { Track, SourceType } from '../components/TrackList';
import type { QueueTrack } from './PlayerCommandsContext';

// Shared interfaces for library data
export interface Album {
  id: number | string;
  title: string;
  artist?: string;
  artist_id?: number;
  artist_name?: string;
  year?: number;
  trackCount?: number;
  coverUrl?: string;
  cover_art_path?: string;
  trackIds?: string[];
}

export interface Artist {
  id: number | string;
  name: string;
  sort_name?: string;
  track_count: number;
  album_count: number;
}

export interface Playlist {
  id: string;
  name: string;
  description?: string;
  owner_id?: number;
  is_public?: boolean;
  is_favorite?: boolean;
  track_count: number;
  created_at?: string;
  updated_at?: string;
}

export interface Genre {
  name: string;
  track_count: number;
}

export interface DatabaseHealth {
  total_tracks: number;
  tracks_with_availability: number;
  tracks_with_local_files: number;
  issues: string[];
}

// Extended Track interface with desktop-specific fields
export interface LibraryTrack extends Track {
  artist_name?: string;
  album_title?: string;
  duration_seconds?: number;
  file_path?: string;
  year?: number;
  file_format?: string;
  bit_rate?: number;
  sample_rate?: number;
  channels?: number;
  source_id?: number;
  source_name?: string;
  source_type?: SourceType;
  source_online?: boolean;
  // Convenience getters for mapping
  coverUrl?: string;
  path?: string;
}

// Library data interface - platform-agnostic operations
export interface LibraryDataInterface {
  // Data loading
  isLoading: boolean;
  error: string | null;
  healthWarning: string | null;

  // Data arrays
  tracks: LibraryTrack[];
  albums: Album[];
  artists: Artist[];
  playlists: Playlist[];
  genres: Genre[];

  // Operations
  loadLibrary: () => Promise<void>;

  // Optional operations (desktop-only)
  createPlaylist?: (name: string, description?: string) => Promise<Playlist>;
  deleteTrack?: (id: number) => Promise<void>;
  checkDatabaseHealth?: () => Promise<DatabaseHealth>;

  // Track lookup helpers
  getTrackById: (id: string | number) => LibraryTrack | undefined;
  getAlbumById: (id: string | number) => Album | undefined;
  getArtistById: (id: string | number) => Artist | undefined;
  getPlaylistById: (id: string) => Playlist | undefined;

  // Album/Artist data
  getAlbumTracks: (albumId: string | number) => Promise<LibraryTrack[]>;
  getArtistTracks: (artistId: string | number) => Promise<LibraryTrack[]>;
  getArtistAlbums: (artistId: string | number) => Promise<Album[]>;
  getPlaylistTracks: (playlistId: string) => Promise<LibraryTrack[]>;

  // Queue building helper
  buildQueueFromTracks: (
    tracks: LibraryTrack[],
    clickedTrack: Track,
    clickedIndex: number
  ) => QueueTrack[];
}

const LibraryDataContext = createContext<LibraryDataInterface | null>(null);

export function useLibraryData(): LibraryDataInterface {
  const context = useContext(LibraryDataContext);
  if (!context) {
    throw new Error('useLibraryData must be used within LibraryDataProvider');
  }
  return context;
}

interface LibraryDataProviderProps {
  children: ReactNode;
  value: LibraryDataInterface;
}

export function LibraryDataProvider({ children, value }: LibraryDataProviderProps) {
  return (
    <LibraryDataContext.Provider value={value}>
      {children}
    </LibraryDataContext.Provider>
  );
}
