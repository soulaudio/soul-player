/**
 * Album-related React Query hooks
 * Uses queryOptions pattern for reusability and type safety
 *
 * @see https://tanstack.com/query/v5/docs/framework/react/guides/query-options
 */

import { queryOptions, useQuery } from '@tanstack/react-query'
import { useBackend } from '../../contexts/BackendContext'
import { albumKeys } from './queryKeys'
import {
  createDetailQueryOptions,
  createFetcherQueryOptions,
  CACHE_TIMES,
  type BackendType,
} from './queryFactories'

/**
 * Query options for fetching a single album by ID
 * Can be used with useQuery, useSuspenseQuery, or queryClient.prefetchQuery
 */
export function albumDetailOptions(backend: BackendType, id: number) {
  return createDetailQueryOptions({
    queryKey: albumKeys.detail(id),
    fetcher: () => backend.getAlbumById(id),
    entityName: 'Album',
    id,
    cacheTime: CACHE_TIMES.METADATA,
  })
}

/**
 * Query options for fetching album tracks
 */
export function albumTracksOptions(backend: BackendType, id: number) {
  return createFetcherQueryOptions({
    queryKey: albumKeys.tracks(id),
    fetcher: () => backend.getAlbumTracks(id),
    cacheTime: CACHE_TIMES.METADATA,
  })
}

/**
 * Query options for all albums list
 */
export function albumsListOptions(backend: BackendType) {
  return createFetcherQueryOptions({
    queryKey: albumKeys.list(),
    fetcher: () => backend.getAllAlbums(),
    cacheTime: CACHE_TIMES.LIST,
  })
}

/**
 * Query options for random albums
 */
export function randomAlbumsOptions(backend: BackendType, limit: number) {
  return createFetcherQueryOptions({
    queryKey: albumKeys.random(limit),
    fetcher: () => backend.getRandomAlbums(limit),
    cacheTime: CACHE_TIMES.DYNAMIC,
  })
}

/**
 * Query options for recently added albums
 */
export function recentlyAddedAlbumsOptions(
  backend: BackendType,
  limit: number
) {
  return queryOptions({
    queryKey: albumKeys.recentlyAdded(limit),
    queryFn: () => backend.getRecentlyAddedAlbums(limit),
    staleTime: 1000 * 60 * 2, // 2 minutes
    gcTime: 1000 * 60 * 10,
  })
}

/**
 * Query options for recently added albums within days
 */
export function recentlyAddedAlbumsWithinDaysOptions(
  backend: BackendType,
  days: number,
  limit: number
) {
  return queryOptions({
    queryKey: albumKeys.recentlyAddedWithinDays(days, limit),
    queryFn: () => backend.getRecentlyAddedAlbumsWithinDays(days, limit),
    staleTime: 1000 * 60 * 2,
    gcTime: 1000 * 60 * 10,
  })
}

/**
 * Query options for least played albums
 */
export function leastPlayedAlbumsOptions(
  backend: BackendType,
  limit: number
) {
  return queryOptions({
    queryKey: albumKeys.leastPlayed(limit),
    queryFn: () => backend.getLeastPlayedAlbums(limit),
    staleTime: 1000 * 60 * 5, // 5 minutes - play counts change slowly
    gcTime: 1000 * 60 * 15,
  })
}

/**
 * Query options for time capsule albums
 */
export function timeCapsuleAlbumsOptions(
  backend: BackendType,
  limit: number
) {
  return queryOptions({
    queryKey: albumKeys.timeCapsule(limit),
    queryFn: () => backend.getTimeCapsuleAlbums(limit),
    staleTime: 1000 * 60 * 5,
    gcTime: 1000 * 60 * 15,
  })
}

// ============================================================================
// Hooks - convenient wrappers around queryOptions
// ============================================================================

/**
 * Hook to fetch a single album by ID
 * Automatically handles loading, error, and caching
 *
 * @example
 * ```tsx
 * const { data: album, isLoading, error } = useAlbum(albumId)
 * if (isLoading) return <Spinner />
 * if (error) return <Error error={error} />
 * return <div>{album.title}</div>
 * ```
 */
export function useAlbum(id: number | undefined) {
  const backend = useBackend()
  return useQuery({
    ...albumDetailOptions(backend, id!),
    enabled: !!id, // Don't run query if id is undefined
  })
}

/**
 * Hook to fetch album tracks
 */
export function useAlbumTracks(id: number | undefined) {
  const backend = useBackend()
  return useQuery({
    ...albumTracksOptions(backend, id!),
    enabled: !!id,
  })
}

/**
 * Hook to fetch all albums
 * @param options - Optional React Query options (e.g., { enabled: false })
 */
export function useAlbums(options?: { enabled?: boolean }) {
  const backend = useBackend()
  return useQuery({
    ...albumsListOptions(backend),
    ...options,
  })
}

/**
 * Hook to fetch random albums
 */
export function useRandomAlbums(limit: number) {
  const backend = useBackend()
  return useQuery(randomAlbumsOptions(backend, limit))
}

/**
 * Hook to fetch recently added albums
 */
export function useRecentlyAddedAlbums(limit: number) {
  const backend = useBackend()
  return useQuery(recentlyAddedAlbumsOptions(backend, limit))
}

/**
 * Hook to fetch recently added albums within days
 */
export function useRecentlyAddedAlbumsWithinDays(days: number, limit: number) {
  const backend = useBackend()
  return useQuery(recentlyAddedAlbumsWithinDaysOptions(backend, days, limit))
}

/**
 * Hook to fetch least played albums
 */
export function useLeastPlayedAlbums(limit: number) {
  const backend = useBackend()
  return useQuery(leastPlayedAlbumsOptions(backend, limit))
}

/**
 * Hook to fetch time capsule albums
 */
export function useTimeCapsuleAlbums(limit: number) {
  const backend = useBackend()
  return useQuery(timeCapsuleAlbumsOptions(backend, limit))
}

/**
 * Combined hook to fetch album with tracks in parallel
 * More efficient than calling useAlbum + useAlbumTracks separately
 *
 * @example
 * ```tsx
 * const { album, tracks, isLoading } = useAlbumWithTracks(albumId)
 * ```
 */
export function useAlbumWithTracks(id: number | undefined) {
  const albumQuery = useAlbum(id)
  const tracksQuery = useAlbumTracks(id)

  return {
    album: albumQuery.data,
    tracks: tracksQuery.data,
    isLoading: albumQuery.isLoading || tracksQuery.isLoading,
    isError: albumQuery.isError || tracksQuery.isError,
    error: albumQuery.error || tracksQuery.error,
  }
}
