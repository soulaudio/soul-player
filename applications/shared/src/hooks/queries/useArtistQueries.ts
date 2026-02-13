/**
 * Artist-related React Query hooks
 * Uses queryOptions pattern for reusability and type safety
 */

import { queryOptions, useQuery } from '@tanstack/react-query'
import { useBackend } from '../../contexts/BackendContext'
import { artistKeys } from './queryKeys'
import {
  createDetailQueryOptions,
  createFetcherQueryOptions,
  CACHE_TIMES,
  type BackendType,
} from './queryFactories'

/**
 * Query options for fetching a single artist by ID
 */
export function artistDetailOptions(backend: BackendType, id: number) {
  return createDetailQueryOptions({
    queryKey: artistKeys.detail(id),
    fetcher: () => backend.getArtistById(id),
    entityName: 'Artist',
    id,
    cacheTime: CACHE_TIMES.METADATA,
  })
}

/**
 * Query options for fetching artist tracks
 */
export function artistTracksOptions(backend: BackendType, id: number) {
  return createFetcherQueryOptions({
    queryKey: artistKeys.tracks(id),
    fetcher: () => backend.getArtistTracks(id),
    cacheTime: CACHE_TIMES.METADATA,
  })
}

/**
 * Query options for fetching artist albums
 */
export function artistAlbumsOptions(backend: BackendType, id: number) {
  return createFetcherQueryOptions({
    queryKey: artistKeys.albums(id),
    fetcher: () => backend.getArtistAlbums(id),
    cacheTime: CACHE_TIMES.METADATA,
  })
}

/**
 * Query options for fetching artist top tracks
 */
export function artistTopTracksOptions(
  backend: BackendType,
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
export function artistArtworkOptions(backend: BackendType, id: number) {
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
export function artistsListOptions(backend: BackendType) {
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
  }
}
