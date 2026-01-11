/**
 * BackendContext - abstracts all backend operations for platform-agnostic pages
 * Desktop: Uses Tauri invoke()
 * Marketing: Uses demo data and mock implementations
 */

import { createContext, useContext, ReactNode } from 'react'

// =============================================================================
// Types - shared data structures
// =============================================================================

export interface BackendTrack {
  id: number
  title: string
  artist_name?: string
  album_title?: string
  album_id?: number
  artist_id?: number
  duration_seconds?: number
  file_path?: string
  track_number?: number
  year?: number
  file_format?: string
  bit_rate?: number
  sample_rate?: number
  channels?: number
  source_type?: string
  source_name?: string
  source_online?: boolean
  cover_art_path?: string
}

export interface BackendAlbum {
  id: number
  title: string
  artist_id?: number
  artist_name?: string
  year?: number
  track_count?: number
  cover_art_path?: string
}

export interface BackendArtist {
  id: number
  name: string
  sort_name?: string
  track_count: number
  album_count: number
}

export interface BackendPlaylist {
  id: string
  name: string
  description?: string
  owner_id: number
  is_public: boolean
  is_favorite: boolean
  track_count: number
  created_at: string
  updated_at: string
}

export interface BackendGenre {
  id: number
  name: string
  track_count: number
}

export interface DatabaseHealth {
  total_tracks: number
  tracks_with_availability: number
  tracks_with_local_files: number
  issues: string[]
}

export interface PlaybackContext {
  id?: number
  contextType: 'album' | 'artist' | 'playlist' | 'genre' | 'tracks'
  contextId: string | null
  contextName: string | null
  contextArtworkPath: string | null
  playedAt?: string
}

export interface QueueTrack {
  trackId: string
  title: string
  artist: string
  album: string | null
  filePath: string
  durationSeconds: number | null
  trackNumber: number | null
}

// =============================================================================
// Backend Interface
// =============================================================================

export interface BackendInterface {
  // Library data
  getAllTracks: () => Promise<BackendTrack[]>
  getAllAlbums: () => Promise<BackendAlbum[]>
  getAllArtists: () => Promise<BackendArtist[]>
  getAllPlaylists: () => Promise<BackendPlaylist[]>
  getAllGenres: () => Promise<BackendGenre[]>

  // Single item lookups
  getAlbumById: (id: number) => Promise<BackendAlbum | null>
  getArtistById: (id: number) => Promise<BackendArtist | null>
  getPlaylistById: (id: string) => Promise<BackendPlaylist | null>
  getGenreById: (id: number) => Promise<BackendGenre | null>

  // Related data
  getAlbumTracks: (albumId: number) => Promise<BackendTrack[]>
  getArtistTracks: (artistId: number) => Promise<BackendTrack[]>
  getArtistAlbums: (artistId: number) => Promise<BackendAlbum[]>
  getPlaylistTracks: (playlistId: string) => Promise<BackendTrack[]>
  getGenreTracks: (genreId: number) => Promise<BackendTrack[]>

  // Health check
  checkDatabaseHealth: () => Promise<DatabaseHealth>

  // Playback context (for "Jump back into" feature)
  getRecentContexts: (limit: number) => Promise<PlaybackContext[]>
  recordContext: (context: Omit<PlaybackContext, 'id' | 'playedAt'>) => Promise<void>

  // Playlist operations
  createPlaylist: (name: string, description?: string) => Promise<BackendPlaylist>
  deletePlaylist: (id: string) => Promise<void>

  // Track operations
  deleteTrack: (id: number) => Promise<void>

  // Queue/playback
  playQueue: (queue: QueueTrack[], startIndex: number) => Promise<void>

  // Onboarding (desktop only, can be no-op for web)
  checkOnboardingNeeded: () => Promise<boolean>
}

// =============================================================================
// Context
// =============================================================================

const BackendContext = createContext<BackendInterface | null>(null)

export function useBackend(): BackendInterface {
  const context = useContext(BackendContext)
  if (!context) {
    throw new Error('useBackend must be used within BackendProvider')
  }
  return context
}

interface BackendProviderProps {
  children: ReactNode
  value: BackendInterface
}

export function BackendProvider({ children, value }: BackendProviderProps) {
  return (
    <BackendContext.Provider value={value}>
      {children}
    </BackendContext.Provider>
  )
}
