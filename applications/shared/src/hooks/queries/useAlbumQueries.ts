/**
 * Album-related React Query hooks
 * Uses queryOptions pattern for reusability and type safety
 *
 * @see https://tanstack.com/query/v5/docs/framework/react/guides/query-options
 */

import { queryOptions, useQuery, useSuspenseQuery } from '@tanstack/react-query'
import { useBackend } from '../../contexts/BackendContext'
import { albumKeys } from './queryKeys'
import type { BackendAlbum, BackendTrack } from '../../contexts/BackendContext'

/**
 * Query options for fetching a single album by ID
 * Can be used with useQuery, useSuspenseQuery, or queryClient.prefetchQuery
 */
export function albumDetailOptions(backend: ReturnType<typeof useBackend>, id: number) {
  return queryOptions({
    queryKey: albumKeys.detail(id),
    queryFn: async () => {
      const album = await backend.getAlbumById(id)
      if (!album) {
        throw new Error(`Album ${id} not found`)
      }
      return album
    },
    staleTime: 1000 * 60 * 5, // 5 minutes - album metadata rarely changes
    gcTime: 1000 * 60 * 30, // 30 minutes in cache
  })
}

/**
 * Query options for fetching album tracks
 */
export function albumTracksOptions(backend: ReturnType<typeof useBackend>, id: number) {
  return queryOptions({
    queryKey: albumKeys.tracks(id),
    queryFn: () => backend.getAlbumTracks(id),
    staleTime: 1000 * 60 * 5, // 5 minutes
    gcTime: 1000 * 60 * 30, // 30 minutes
  })
}

/**
 * Query options for all albums list
 */
export function albumsListOptions(backend: ReturnType<typeof useBackend>) {
  return queryOptions({
    queryKey: albumKeys.list(),
    queryFn: () => backend.getAllAlbums(),
    staleTime: 1000 * 60 * 2, // 2 minutes - list can change more frequently
    gcTime: 1000 * 60 * 10, // 10 minutes
  })
}

/**
 * Query options for random albums
 */
export function randomAlbumsOptions(backend: ReturnType<typeof useBackend>, limit: number) {
  return queryOptions({
    queryKey: albumKeys.random(limit),
    queryFn: () => backend.getRandomAlbums(limit),
    staleTime: 1000 * 60 * 1, // 1 minute - randomness should refresh more often
    gcTime: 1000 * 60 * 5,
  })
}

/**
 * Query options for recently added albums
 */
export function recentlyAddedAlbumsOptions(
  backend: ReturnType<typeof useBackend>,
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
  backend: ReturnType<typeof useBackend>,
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
  backend: ReturnType<typeof useBackend>,
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
  backend: ReturnType<typeof useBackend>,
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
 */
export function useAlbums() {
  const backend = useBackend()
  return useQuery(albumsListOptions(backend))
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
    refetch: () => {
      albumQuery.refetch()
      tracksQuery.refetch()
    },
  }
}
