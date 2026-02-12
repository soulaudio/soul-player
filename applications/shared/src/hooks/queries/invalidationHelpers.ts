/**
 * Cache Invalidation Helpers
 *
 * Centralized functions for consistent cache invalidation across the app.
 * These helpers ensure related queries are invalidated together to prevent stale data.
 *
 * Usage:
 * ```typescript
 * import { invalidateAfterAlbumArtworkChange } from './invalidationHelpers'
 *
 * // In mutation onSuccess callback:
 * invalidateAfterAlbumArtworkChange(queryClient, albumId)
 * ```
 */

import { QueryClient } from '@tanstack/react-query'
import { albumKeys, artistKeys, playlistKeys, trackKeys, genreKeys, libraryKeys } from './queryKeys'

/**
 * Invalidates all library-related caches after file scan completes.
 * Call this when library import/scan finishes.
 *
 * Invalidates:
 * - All tracks
 * - All albums
 * - All artists
 * - All genres
 * - Library health status
 *
 * @param queryClient - TanStack Query client instance
 */
export function invalidateAfterFileScan(queryClient: QueryClient): void {
  queryClient.invalidateQueries({ queryKey: trackKeys.all() })
  queryClient.invalidateQueries({ queryKey: albumKeys.all() })
  queryClient.invalidateQueries({ queryKey: artistKeys.all() })
  queryClient.invalidateQueries({ queryKey: genreKeys.all() })
  queryClient.invalidateQueries({ queryKey: libraryKeys.health() })
}

/**
 * Invalidates caches after album artwork changes.
 *
 * Invalidates:
 * - Album detail (contains artwork path)
 * - Album lists (thumbnails need updating)
 * - Artwork cache (if integrated with React Query)
 *
 * @param queryClient - TanStack Query client instance
 * @param albumId - ID of the album whose artwork changed
 */
export function invalidateAfterAlbumArtworkChange(
  queryClient: QueryClient,
  albumId: number
): void {
  // Invalidate specific album detail
  queryClient.invalidateQueries({ queryKey: albumKeys.detail(albumId) })

  // Invalidate album lists (thumbnails need to update)
  queryClient.invalidateQueries({ queryKey: albumKeys.lists() })

  // Invalidate artwork queries (if using React Query for artwork)
  queryClient.invalidateQueries({ queryKey: ['artwork', 'album', albumId] })
}

/**
 * Invalidates caches after artist artwork changes.
 *
 * Invalidates:
 * - Artist detail
 * - Artist lists (thumbnails)
 * - Artwork cache
 *
 * @param queryClient - TanStack Query client instance
 * @param artistId - ID of the artist whose artwork changed
 */
export function invalidateAfterArtistArtworkChange(
  queryClient: QueryClient,
  artistId: number
): void {
  queryClient.invalidateQueries({ queryKey: artistKeys.detail(artistId) })
  queryClient.invalidateQueries({ queryKey: artistKeys.lists() })
  queryClient.invalidateQueries({ queryKey: ['artwork', 'artist', artistId] })
}

/**
 * Invalidates caches after playlist artwork changes.
 *
 * @param queryClient - TanStack Query client instance
 * @param playlistId - ID of the playlist whose artwork changed
 */
export function invalidateAfterPlaylistArtworkChange(
  queryClient: QueryClient,
  playlistId: string
): void {
  queryClient.invalidateQueries({ queryKey: playlistKeys.detail(playlistId) })
  queryClient.invalidateQueries({ queryKey: playlistKeys.lists() })
  queryClient.invalidateQueries({ queryKey: ['artwork', 'playlist', playlistId] })
}

/**
 * Invalidates caches after album metadata update (title, year, artist, etc.).
 *
 * IMPORTANT: Also invalidates tracks because they cache album_title.
 *
 * Invalidates:
 * - Album detail
 * - Album lists
 * - Album tracks
 * - All tracks (they cache album_title)
 *
 * @param queryClient - TanStack Query client instance
 * @param albumId - ID of the album that was updated
 */
export function invalidateAfterAlbumMetadataUpdate(
  queryClient: QueryClient,
  albumId: number
): void {
  // Album-specific queries
  queryClient.invalidateQueries({ queryKey: albumKeys.detail(albumId) })
  queryClient.invalidateQueries({ queryKey: albumKeys.lists() })
  queryClient.invalidateQueries({ queryKey: albumKeys.tracks(albumId) })

  // All tracks cache album_title, so must invalidate
  queryClient.invalidateQueries({ queryKey: trackKeys.all() })
}

/**
 * Invalidates caches after artist metadata update (name, etc.).
 *
 * IMPORTANT: Also invalidates tracks because they cache artist_name.
 *
 * Invalidates:
 * - Artist detail
 * - Artist lists
 * - Artist tracks
 * - Artist albums
 * - All tracks (they cache artist_name)
 *
 * @param queryClient - TanStack Query client instance
 * @param artistId - ID of the artist that was updated
 */
export function invalidateAfterArtistMetadataUpdate(
  queryClient: QueryClient,
  artistId: number
): void {
  // Artist-specific queries
  queryClient.invalidateQueries({ queryKey: artistKeys.detail(artistId) })
  queryClient.invalidateQueries({ queryKey: artistKeys.lists() })
  queryClient.invalidateQueries({ queryKey: artistKeys.tracks(artistId) })
  queryClient.invalidateQueries({ queryKey: artistKeys.albums(artistId) })

  // All tracks cache artist_name, so must invalidate
  queryClient.invalidateQueries({ queryKey: trackKeys.all() })
}

/**
 * Invalidates caches after track deletion.
 *
 * Uses cascade invalidation based on affected entities returned from backend.
 * More efficient than broad invalidation.
 *
 * @param queryClient - TanStack Query client instance
 * @param affectedEntities - Entities affected by deletion (from backend response)
 */
export function invalidateAfterTrackDeletion(
  queryClient: QueryClient,
  affectedEntities: {
    albumId?: number
    artistId?: number
    playlistIds?: string[]
  }
): void {
  // Always invalidate all tracks
  queryClient.invalidateQueries({ queryKey: trackKeys.all() })

  // Invalidate specific album (track count changed)
  if (affectedEntities.albumId) {
    queryClient.invalidateQueries({ queryKey: albumKeys.detail(affectedEntities.albumId) })
    queryClient.invalidateQueries({ queryKey: albumKeys.tracks(affectedEntities.albumId) })
  }

  // Invalidate specific artist (track count changed)
  if (affectedEntities.artistId) {
    queryClient.invalidateQueries({ queryKey: artistKeys.detail(affectedEntities.artistId) })
    queryClient.invalidateQueries({ queryKey: artistKeys.tracks(affectedEntities.artistId) })
  }

  // Invalidate affected playlists
  if (affectedEntities.playlistIds) {
    affectedEntities.playlistIds.forEach(playlistId => {
      queryClient.invalidateQueries({ queryKey: playlistKeys.detail(playlistId) })
      queryClient.invalidateQueries({ queryKey: playlistKeys.tracks(playlistId) })
    })
  }
}

/**
 * Batch invalidation for multiple albums.
 * Useful when bulk operations affect multiple albums.
 *
 * @param queryClient - TanStack Query client instance
 * @param albumIds - Array of album IDs to invalidate
 */
export function invalidateMultipleAlbums(
  queryClient: QueryClient,
  albumIds: number[]
): void {
  albumIds.forEach(albumId => {
    queryClient.invalidateQueries({ queryKey: albumKeys.detail(albumId) })
    queryClient.invalidateQueries({ queryKey: albumKeys.tracks(albumId) })
  })

  // Invalidate lists once for all albums
  queryClient.invalidateQueries({ queryKey: albumKeys.lists() })
}

/**
 * Batch invalidation for multiple artists.
 *
 * @param queryClient - TanStack Query client instance
 * @param artistIds - Array of artist IDs to invalidate
 */
export function invalidateMultipleArtists(
  queryClient: QueryClient,
  artistIds: number[]
): void {
  artistIds.forEach(artistId => {
    queryClient.invalidateQueries({ queryKey: artistKeys.detail(artistId) })
    queryClient.invalidateQueries({ queryKey: artistKeys.tracks(artistId) })
  })

  // Invalidate lists once for all artists
  queryClient.invalidateQueries({ queryKey: artistKeys.lists() })
}
