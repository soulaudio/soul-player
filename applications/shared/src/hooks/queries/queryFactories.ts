/**
 * Reusable query option factories to reduce boilerplate
 * These factories follow common patterns across different entity types
 */

import { queryOptions } from '@tanstack/react-query'
import type { useBackend } from '../../contexts/BackendContext'

// Common cache timing configurations
export const CACHE_TIMES = {
  // For metadata that rarely changes (artists, albums, playlists)
  METADATA: {
    staleTime: 1000 * 60 * 5, // 5 minutes
    gcTime: 1000 * 60 * 30, // 30 minutes
  },
  // For lists that can change more frequently
  LIST: {
    staleTime: 1000 * 60 * 2, // 2 minutes
    gcTime: 1000 * 60 * 10, // 10 minutes
  },
  // For dynamic/random content
  DYNAMIC: {
    staleTime: 1000 * 60 * 1, // 1 minute
    gcTime: 1000 * 60 * 5,
  },
  // For play counts and statistics
  STATS: {
    staleTime: 1000 * 60 * 5, // 5 minutes
    gcTime: 1000 * 60 * 15,
  },
}

/**
 * Generic factory for "detail" queries (fetch by ID with null check)
 * Reduces duplication across album/artist/playlist detail queries
 *
 * @example
 * ```ts
 * export function albumDetailOptions(backend: BackendType, id: number) {
 *   return createDetailQueryOptions({
 *     queryKey: albumKeys.detail(id),
 *     fetcher: () => backend.getAlbumById(id),
 *     entityName: 'Album',
 *     id,
 *     cacheTime: CACHE_TIMES.METADATA,
 *   })
 * }
 * ```
 */
export function createDetailQueryOptions<T>(config: {
  queryKey: readonly unknown[]
  fetcher: () => Promise<T | null | undefined>
  entityName: string
  id: number | string
  cacheTime?: { staleTime: number; gcTime: number }
}) {
  const { queryKey, fetcher, entityName, id, cacheTime = CACHE_TIMES.METADATA } = config

  return queryOptions({
    queryKey,
    queryFn: async () => {
      const result = await fetcher()
      if (!result) {
        throw new Error(`${entityName} ${id} not found`)
      }
      return result
    },
    staleTime: cacheTime.staleTime,
    gcTime: cacheTime.gcTime,
  })
}

/**
 * Generic factory for simple list/fetcher queries
 * For queries that just fetch data without null checks
 *
 * @example
 * ```ts
 * export function albumTracksOptions(backend: BackendType, id: number) {
 *   return createFetcherQueryOptions({
 *     queryKey: albumKeys.tracks(id),
 *     fetcher: () => backend.getAlbumTracks(id),
 *     cacheTime: CACHE_TIMES.METADATA,
 *   })
 * }
 * ```
 */
export function createFetcherQueryOptions<T>(config: {
  queryKey: readonly unknown[]
  fetcher: () => Promise<T>
  cacheTime?: { staleTime: number; gcTime: number }
}) {
  const { queryKey, fetcher, cacheTime = CACHE_TIMES.LIST } = config

  return queryOptions({
    queryKey,
    queryFn: fetcher,
    staleTime: cacheTime.staleTime,
    gcTime: cacheTime.gcTime,
  })
}

// Type helper for useBackend return type
export type BackendType = ReturnType<typeof useBackend>
