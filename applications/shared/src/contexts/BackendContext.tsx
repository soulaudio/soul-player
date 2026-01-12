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
  // Whether the track is in the managed library (vs watched folder)
  is_in_managed_library?: boolean
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
  cover_art_path?: string
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
  cover_art_path?: string
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

export interface SetArtworkParams {
  entityType: 'album' | 'artist' | 'playlist'
  entityId: string
  artworkBase64: string
  mimeType: string
  writeToFiles?: boolean // Only for albums - embed in track files
  useSoulStorage?: boolean // Only for albums - use Soul Player storage instead of album folder
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
  getPlaylistsContainingTrack: (trackId: number) => Promise<string[]>
  addTrackToPlaylist: (playlistId: string, trackId: number) => Promise<void>
  removeTrackFromPlaylist: (playlistId: string, trackId: number) => Promise<void>

  // Track operations
  deleteTrack: (id: number) => Promise<void>
  showInFileExplorer: (path: string) => Promise<void>

  // Onboarding (desktop only, can be no-op for web)
  checkOnboardingNeeded: () => Promise<boolean>

  // Artwork editing
  setArtwork: (params: SetArtworkParams) => Promise<void>
  removeArtwork: (entityType: 'album' | 'artist' | 'playlist', entityId: string) => Promise<void>
  getArtistArtwork: (artistId: number) => Promise<string | null>
  getPlaylistArtwork: (playlistId: string) => Promise<string | null>
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
