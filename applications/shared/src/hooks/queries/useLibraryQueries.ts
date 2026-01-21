/**
 * Library-wide query hooks (tracks, playlists, genres, health, etc.)
 * Uses queryOptions pattern for reusability and type safety
 */

import { useMemo } from 'react'
import { queryOptions, useQuery } from '@tanstack/react-query'
import { useBackend } from '../../contexts/BackendContext'
import { trackKeys, playlistKeys, genreKeys, libraryKeys, contextKeys } from './queryKeys'
import { albumsListOptions } from './useAlbumQueries'
import { artistsListOptions } from './useArtistQueries'

// Re-export for convenience (avoids having to import from multiple files)
export { albumsListOptions, artistsListOptions }

// ============================================================================
// Tracks Query Options
// ============================================================================

export function tracksListOptions(backend: ReturnType<typeof useBackend>) {
  return queryOptions({
    queryKey: trackKeys.list(),
    queryFn: () => backend.getAllTracks(),
    staleTime: 1000 * 60 * 2, // 2 minutes
    gcTime: 1000 * 60 * 10, // 10 minutes
  })
}

// NOTE: albumsListOptions and artistsListOptions are imported from their
// dedicated query files (useAlbumQueries, useArtistQueries) to avoid duplication

// ============================================================================
// Playlists Query Options
// ============================================================================

export function playlistsListOptions(backend: ReturnType<typeof useBackend>) {
  return queryOptions({
    queryKey: playlistKeys.list(),
    queryFn: () => backend.getAllPlaylists(),
    staleTime: 1000 * 60 * 1, // 1 minute - playlists change frequently
    gcTime: 1000 * 60 * 5,
  })
}

export function playlistDetailOptions(
  backend: ReturnType<typeof useBackend>,
  id: string
) {
  return queryOptions({
    queryKey: playlistKeys.detail(id),
    queryFn: async () => {
      const playlist = await backend.getPlaylistById(id)
      if (!playlist) {
        throw new Error(`Playlist ${id} not found`)
      }
      return playlist
    },
    staleTime: 1000 * 60 * 2,
    gcTime: 1000 * 60 * 10,
  })
}

export function playlistTracksOptions(
  backend: ReturnType<typeof useBackend>,
  id: string
) {
  return queryOptions({
    queryKey: playlistKeys.tracks(id),
    queryFn: () => backend.getPlaylistTracks(id),
    staleTime: 1000 * 60 * 1, // 1 minute - tracks can be added/removed frequently
    gcTime: 1000 * 60 * 5,
  })
}

export function playlistArtworkOptions(
  backend: ReturnType<typeof useBackend>,
  id: string
) {
  return queryOptions({
    queryKey: playlistKeys.artwork(id),
    queryFn: () => backend.getPlaylistArtwork(id),
    staleTime: 1000 * 60 * 10, // 10 minutes
    gcTime: 1000 * 60 * 30,
  })
}

export function playlistsContainingTrackOptions(
  backend: ReturnType<typeof useBackend>,
  trackId: number
) {
  return queryOptions({
    queryKey: playlistKeys.containingTrack(trackId),
    queryFn: () => backend.getPlaylistsContainingTrack(trackId),
    staleTime: 1000 * 30, // 30 seconds
    gcTime: 1000 * 60 * 2,
  })
}

// ============================================================================
// Genres Query Options
// ============================================================================

export function genresListOptions(backend: ReturnType<typeof useBackend>) {
  return queryOptions({
    queryKey: genreKeys.list(),
    queryFn: () => backend.getAllGenres(),
    staleTime: 1000 * 60 * 10, // 10 minutes - genres rarely change
    gcTime: 1000 * 60 * 60, // 1 hour
  })
}

export function genreDetailOptions(backend: ReturnType<typeof useBackend>, id: number) {
  return queryOptions({
    queryKey: genreKeys.detail(id),
    queryFn: async () => {
      const genre = await backend.getGenreById(id)
      if (!genre) {
        throw new Error(`Genre ${id} not found`)
      }
      return genre
    },
    staleTime: 1000 * 60 * 10,
    gcTime: 1000 * 60 * 60,
  })
}

export function genreTracksOptions(backend: ReturnType<typeof useBackend>, id: number) {
  return queryOptions({
    queryKey: genreKeys.tracks(id),
    queryFn: () => backend.getGenreTracks(id),
    staleTime: 1000 * 60 * 5,
    gcTime: 1000 * 60 * 30,
  })
}

export function genreAlbumsOptions(
  backend: ReturnType<typeof useBackend>,
  id: number,
  limit: number
) {
  return queryOptions({
    queryKey: genreKeys.albums(id, limit),
    queryFn: () => backend.getGenreAlbums(id, limit),
    staleTime: 1000 * 60 * 5,
    gcTime: 1000 * 60 * 30,
  })
}

// ============================================================================
// Library Health Query Options
// ============================================================================

export function databaseHealthOptions(backend: ReturnType<typeof useBackend>) {
  return queryOptions({
    queryKey: libraryKeys.health(),
    queryFn: () => backend.checkDatabaseHealth(),
    staleTime: 1000 * 60 * 5, // 5 minutes
    gcTime: 1000 * 60 * 10,
  })
}

// ============================================================================
// Playback Context Query Options
// ============================================================================

export function recentContextsOptions(
  backend: ReturnType<typeof useBackend>,
  limit: number
) {
  return queryOptions({
    queryKey: contextKeys.recent(limit),
    queryFn: () => backend.getRecentContexts(limit),
    staleTime: 1000 * 30, // 30 seconds - contexts change frequently
    gcTime: 1000 * 60 * 2,
  })
}

// ============================================================================
// Hooks
// ============================================================================

export function useTracks() {
  const backend = useBackend()
  return useQuery(tracksListOptions(backend))
}

export function useAlbums() {
  const backend = useBackend()
  return useQuery(albumsListOptions(backend))
}

export function useArtists() {
  const backend = useBackend()
  return useQuery(artistsListOptions(backend))
}

export function usePlaylists() {
  const backend = useBackend()
  return useQuery(playlistsListOptions(backend))
}

export function usePlaylist(id: string | undefined) {
  const backend = useBackend()
  return useQuery({
    ...playlistDetailOptions(backend, id!),
    enabled: !!id,
  })
}

export function usePlaylistTracks(id: string | undefined) {
  const backend = useBackend()
  return useQuery({
    ...playlistTracksOptions(backend, id!),
    enabled: !!id,
  })
}

export function usePlaylistArtwork(id: string | undefined) {
  const backend = useBackend()
  return useQuery({
    ...playlistArtworkOptions(backend, id!),
    enabled: !!id,
  })
}

export function usePlaylistsContainingTrack(trackId: number | undefined) {
  const backend = useBackend()
  return useQuery({
    ...playlistsContainingTrackOptions(backend, trackId!),
    enabled: !!trackId,
  })
}

export function useGenres() {
  const backend = useBackend()
  return useQuery(genresListOptions(backend))
}

export function useGenre(id: number | undefined) {
  const backend = useBackend()
  return useQuery({
    ...genreDetailOptions(backend, id!),
    enabled: !!id,
  })
}

export function useGenreTracks(id: number | undefined) {
  const backend = useBackend()
  return useQuery({
    ...genreTracksOptions(backend, id!),
    enabled: !!id,
  })
}

export function useGenreAlbums(id: number | undefined, limit: number) {
  const backend = useBackend()
  return useQuery({
    ...genreAlbumsOptions(backend, id!, limit),
    enabled: !!id,
  })
}

export function useDatabaseHealth() {
  const backend = useBackend()
  return useQuery(databaseHealthOptions(backend))
}

export function useRecentContexts(limit: number, options?: { enabled?: boolean }) {
  const backend = useBackend()
  return useQuery({
    ...recentContextsOptions(backend, limit),
    ...options,
  })
}

/**
 * Combined hook for full library data (used in LibraryPage)
 * Returns individual loading states for progressive rendering
 *
 * Performance: Uses centralized query options (albumsListOptions, artistsListOptions)
 * to ensure proper cache reuse across the app. Previously used inline queries
 * which prevented cache sharing between LibraryPage and other pages.
 *
 * Memoizes array references to prevent unnecessary re-renders when data
 * hasn't actually changed (React Query structural sharing).
 */
export function useLibraryData() {
  const tracksQuery = useTracks()
  const albumsQuery = useAlbums()
  const artistsQuery = useArtists()
  const playlistsQuery = usePlaylists()
  const healthQuery = useDatabaseHealth()

  // Memoize arrays to prevent unnecessary re-renders
  // React Query's structural sharing ensures data reference only changes when content changes
  // But we need to stabilize the fallback empty arrays
  const tracks = useMemo(() => tracksQuery.data ?? [], [tracksQuery.data])
  const albums = useMemo(() => albumsQuery.data ?? [], [albumsQuery.data])
  const artists = useMemo(() => artistsQuery.data ?? [], [artistsQuery.data])
  const playlists = useMemo(() => playlistsQuery.data ?? [], [playlistsQuery.data])

  return {
    tracks,
    albums,
    artists,
    playlists,
    health: healthQuery.data,
    // Individual loading states for progressive rendering
    isTracksLoading: tracksQuery.isLoading,
    isAlbumsLoading: albumsQuery.isLoading,
    isArtistsLoading: artistsQuery.isLoading,
    isPlaylistsLoading: playlistsQuery.isLoading,
    isHealthLoading: healthQuery.isLoading,
    // Combined loading state for initial page load (any query loading)
    isAnyLoading:
      tracksQuery.isLoading ||
      albumsQuery.isLoading ||
      artistsQuery.isLoading ||
      playlistsQuery.isLoading ||
      healthQuery.isLoading,
    // Error states
    isError:
      tracksQuery.isError ||
      albumsQuery.isError ||
      artistsQuery.isError ||
      playlistsQuery.isError ||
      healthQuery.isError,
    error:
      tracksQuery.error ||
      albumsQuery.error ||
      artistsQuery.error ||
      playlistsQuery.error ||
      healthQuery.error,
  }
}

/**
 * Combined hook for playlist with tracks
 */
export function usePlaylistWithTracks(id: string | undefined) {
  const playlistQuery = usePlaylist(id)
  const tracksQuery = usePlaylistTracks(id)

  return {
    playlist: playlistQuery.data,
    tracks: tracksQuery.data,
    isLoading: playlistQuery.isLoading || tracksQuery.isLoading,
    isError: playlistQuery.isError || tracksQuery.isError,
    error: playlistQuery.error || tracksQuery.error,
  }
}
