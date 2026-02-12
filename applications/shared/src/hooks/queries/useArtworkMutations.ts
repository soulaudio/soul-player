/**
 * Artwork Mutation Hooks
 *
 * Provides mutations for setting and removing artwork with automatic cache invalidation.
 * Coordinates invalidation between React Query cache and component-level artwork cache.
 *
 * Usage:
 * ```typescript
 * const setArtworkMutation = useSetArtwork()
 * await setArtworkMutation.mutateAsync({
 *   entityType: 'album',
 *   entityId: '123',
 *   artworkBase64: base64Data,
 *   mimeType: 'image/jpeg'
 * })
 * ```
 */

import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useBackend, type SetArtworkParams } from '../../contexts/BackendContext'
import {
  invalidateAfterAlbumArtworkChange,
  invalidateAfterArtistArtworkChange,
  invalidateAfterPlaylistArtworkChange,
} from './invalidationHelpers'
import { clearArtworkCache } from '../../components/ArtworkImage'

/**
 * Mutation hook for setting artwork on albums, artists, or playlists.
 *
 * Features:
 * - Automatic cache invalidation (React Query + component cache)
 * - Updates all views showing the artwork
 * - Type-safe entity type and ID
 *
 * @returns Mutation object with mutate/mutateAsync methods
 *
 * @example
 * ```typescript
 * const setArtwork = useSetArtwork()
 *
 * const handleArtworkChange = async (file: File) => {
 *   const base64 = await fileToBase64(file)
 *   await setArtwork.mutateAsync({
 *     entityType: 'album',
 *     entityId: String(albumId),
 *     artworkBase64: base64,
 *     mimeType: file.type,
 *     writeToFiles: true  // For albums: embed in track files
 *   })
 * }
 * ```
 */
export function useSetArtwork() {
  const backend = useBackend()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (params: SetArtworkParams) => {
      await backend.setArtwork(params)
      return params
    },
    onSuccess: (_data, params) => {
      const { entityType, entityId } = params

      // Clear component-level artwork cache (Map-based cache in ArtworkImage.tsx)
      clearArtworkCache(entityType, entityId)

      // Invalidate React Query caches based on entity type
      const numericId = entityType === 'playlist' ? entityId : Number(entityId)

      if (entityType === 'album') {
        invalidateAfterAlbumArtworkChange(queryClient, numericId as number)
      } else if (entityType === 'artist') {
        invalidateAfterArtistArtworkChange(queryClient, numericId as number)
      } else if (entityType === 'playlist') {
        invalidateAfterPlaylistArtworkChange(queryClient, entityId)
      }
    },
    onError: (error, params) => {
      console.error(`[useSetArtwork] Failed to set ${params.entityType} artwork:`, error)
    },
  })
}

/**
 * Mutation hook for removing artwork from albums, artists, or playlists.
 *
 * Features:
 * - Automatic cache invalidation
 * - Reverts to default/fallback artwork
 * - Updates all views
 *
 * @returns Mutation object with mutate/mutateAsync methods
 *
 * @example
 * ```typescript
 * const removeArtwork = useRemoveArtwork()
 *
 * const handleRemoveArtwork = async () => {
 *   await removeArtwork.mutateAsync({
 *     entityType: 'album',
 *     entityId: String(albumId)
 *   })
 * }
 * ```
 */
export function useRemoveArtwork() {
  const backend = useBackend()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (params: {
      entityType: 'album' | 'artist' | 'playlist'
      entityId: string
    }) => {
      await backend.removeArtwork(params.entityType, params.entityId)
      return params
    },
    onSuccess: (_data, params) => {
      const { entityType, entityId } = params

      // Clear component-level artwork cache
      clearArtworkCache(entityType, entityId)

      // Invalidate React Query caches (same logic as setArtwork)
      const numericId = entityType === 'playlist' ? entityId : Number(entityId)

      if (entityType === 'album') {
        invalidateAfterAlbumArtworkChange(queryClient, numericId as number)
      } else if (entityType === 'artist') {
        invalidateAfterArtistArtworkChange(queryClient, numericId as number)
      } else if (entityType === 'playlist') {
        invalidateAfterPlaylistArtworkChange(queryClient, entityId)
      }
    },
    onError: (error, params) => {
      console.error(`[useRemoveArtwork] Failed to remove ${params.entityType} artwork:`, error)
    },
  })
}
