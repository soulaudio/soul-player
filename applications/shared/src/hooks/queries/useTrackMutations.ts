/**
 * Track mutation hooks with optimistic updates
 * Provides instant UI feedback for track operations
 */

import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useBackend } from '../../contexts/BackendContext'
import { trackKeys, albumKeys, artistKeys, playlistKeys } from './queryKeys'

/**
 * Mutation hook for deleting a track with cache invalidation
 *
 * Note: Optimistic delete not implemented to avoid showing tracks that don't exist
 * if user navigates quickly. Instead, we show loading state and invalidate all caches.
 *
 * @example
 * ```tsx
 * const deleteTrackMutation = useDeleteTrack()
 *
 * deleteTrackMutation.mutate(trackId, {
 *   onSuccess: () => toast.success('Track deleted'),
 *   onError: () => toast.error('Failed to delete track'),
 * })
 * ```
 */
export function useDeleteTrack() {
  const backend = useBackend()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (trackId: number) => {
      await backend.deleteTrack(trackId)
    },

    onSuccess: () => {
      // Invalidate all track-related queries
      // This ensures any page showing tracks will refetch
      queryClient.invalidateQueries({ queryKey: trackKeys.all() })

      // Also invalidate album/artist/playlist queries since they contain track counts
      queryClient.invalidateQueries({ queryKey: albumKeys.all() })
      queryClient.invalidateQueries({ queryKey: artistKeys.all() })
      queryClient.invalidateQueries({ queryKey: playlistKeys.all() })
    },
  })
}
