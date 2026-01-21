/**
 * Artist-related React Query hooks
 * Uses queryOptions pattern for reusability and type safety
 */

import { queryOptions, useQuery } from '@tanstack/react-query'
import { useBackend } from '../../contexts/BackendContext'
import { artistKeys } from './queryKeys'
import type { BackendArtist, BackendTrack, BackendAlbum } from '../../contexts/BackendContext'

/**
 * Query options for fetching a single artist by ID
 */
export function artistDetailOptions(backend: ReturnType<typeof useBackend>, id: number) {
  return queryOptions({
    queryKey: artistKeys.detail(id),
    queryFn: async () => {
      const artist = await backend.getArtistById(id)
      if (!artist) {
        throw new Error(`Artist ${id} not found`)
      }
      return artist
    },
    staleTime: 1000 * 60 * 5, // 5 minutes
    gcTime: 1000 * 60 * 30, // 30 minutes
  })
}

/**
 * Query options for fetching artist tracks
 */
export function artistTracksOptions(backend: ReturnType<typeof useBackend>, id: number) {
  return queryOptions({
    queryKey: artistKeys.tracks(id),
    queryFn: () => backend.getArtistTracks(id),
    staleTime: 1000 * 60 * 5,
    gcTime: 1000 * 60 * 30,
  })
}

/**
 * Query options for fetching artist albums
 */
export function artistAlbumsOptions(backend: ReturnType<typeof useBackend>, id: number) {
  return queryOptions({
    queryKey: artistKeys.albums(id),
    queryFn: () => backend.getArtistAlbums(id),
    staleTime: 1000 * 60 * 5,
    gcTime: 1000 * 60 * 30,
  })
}

/**
 * Query options for fetching artist top tracks
 */
export function artistTopTracksOptions(
  backend: ReturnType<typeof useBackend>,
  id: number,
  limit: number = 10
) {
  return queryOptions({
    queryKey: artistKeys.topTracks(id, limit),
    queryFn: () => backend.getArtistTopTracks(id, limit),
    staleTime: 1000 * 60 * 10, // 10 minutes - top tracks change slowly
    gcTime: 1000 * 60 * 30,
  })
}

/**
 * Query options for fetching artist artwork (desktop only)
 */
export function artistArtworkOptions(backend: ReturnType<typeof useBackend>, id: number) {
  return queryOptions({
    queryKey: artistKeys.artwork(id),
    queryFn: () => backend.getArtistArtwork(id),
    staleTime: 1000 * 60 * 30, // 30 minutes - artwork rarely changes
    gcTime: 1000 * 60 * 60, // 1 hour
  })
}

/**
 * Query options for all artists list
 */
export function artistsListOptions(backend: ReturnType<typeof useBackend>) {
  return queryOptions({
    queryKey: artistKeys.list(),
    queryFn: () => backend.getAllArtists(),
    staleTime: 1000 * 60 * 2, // 2 minutes
    gcTime: 1000 * 60 * 10,
  })
}

// ============================================================================
// Hooks
// ============================================================================

/**
 * Hook to fetch a single artist by ID
 */
export function useArtist(id: number | undefined) {
  const backend = useBackend()
  return useQuery({
    ...artistDetailOptions(backend, id!),
    enabled: !!id,
  })
}

/**
 * Hook to fetch artist tracks
 */
export function useArtistTracks(id: number | undefined) {
  const backend = useBackend()
  return useQuery({
    ...artistTracksOptions(backend, id!),
    enabled: !!id,
  })
}

/**
 * Hook to fetch artist albums
 */
export function useArtistAlbums(id: number | undefined) {
  const backend = useBackend()
  return useQuery({
    ...artistAlbumsOptions(backend, id!),
    enabled: !!id,
  })
}

/**
 * Hook to fetch artist top tracks
 */
export function useArtistTopTracks(id: number | undefined, limit: number = 10) {
  const backend = useBackend()
  return useQuery({
    ...artistTopTracksOptions(backend, id!, limit),
    enabled: !!id,
  })
}

/**
 * Hook to fetch artist artwork
 */
export function useArtistArtwork(id: number | undefined) {
  const backend = useBackend()
  return useQuery({
    ...artistArtworkOptions(backend, id!),
    enabled: !!id,
  })
}

/**
 * Hook to fetch all artists
 */
export function useArtists() {
  const backend = useBackend()
  return useQuery(artistsListOptions(backend))
}

/**
 * Combined hook to fetch artist with all related data in parallel
 */
export function useArtistWithData(id: number | undefined) {
  const artistQuery = useArtist(id)
  const tracksQuery = useArtistTracks(id)
  const albumsQuery = useArtistAlbums(id)
  const topTracksQuery = useArtistTopTracks(id, 10)

  return {
    artist: artistQuery.data,
    tracks: tracksQuery.data,
    albums: albumsQuery.data,
    topTracks: topTracksQuery.data,
    isLoading:
      artistQuery.isLoading ||
      tracksQuery.isLoading ||
      albumsQuery.isLoading ||
      topTracksQuery.isLoading,
    isError:
      artistQuery.isError || tracksQuery.isError || albumsQuery.isError || topTracksQuery.isError,
    error: artistQuery.error || tracksQuery.error || albumsQuery.error || topTracksQuery.error,
    refetch: () => {
      artistQuery.refetch()
      tracksQuery.refetch()
      albumsQuery.refetch()
      topTracksQuery.refetch()
    },
  }
}
