/**
 * Playlist mutation hooks with optimistic updates
 * Provides instant UI feedback for playlist operations
 *
 * @see https://tanstack.com/query/v5/docs/framework/react/guides/optimistic-updates
 */

import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useBackend } from '../../contexts/BackendContext'
import { playlistKeys, trackKeys } from './queryKeys'
import type { BackendPlaylist, BackendTrack } from '../../contexts/BackendContext'

/**
 * Mutation hook for adding a track to a playlist with optimistic updates
 *
 * Benefits:
 * - UI updates immediately (before backend confirms)
 * - Rollback if operation fails
 * - Shows user instant feedback
 *
 * @example
 * ```tsx
 * const addTrackMutation = useAddTrackToPlaylist()
 *
 * addTrackMutation.mutate(
 *   { playlistId: '123', trackId: 456 },
 *   {
 *     onSuccess: () => toast.success('Added to playlist'),
 *     onError: () => toast.error('Failed to add track'),
 *   }
 * )
 * ```
 */
export function useAddTrackToPlaylist() {
  const backend = useBackend()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({ playlistId, trackId }: { playlistId: string; trackId: number }) => {
      await backend.addTrackToPlaylist(playlistId, trackId)
    },

    // Optimistic update - runs BEFORE mutation
    onMutate: async ({ playlistId, trackId }) => {
      // Cancel any outgoing refetches (so they don't overwrite our optimistic update)
      await queryClient.cancelQueries({ queryKey: playlistKeys.tracks(playlistId) })

      // Snapshot the previous value
      const previousTracks = queryClient.getQueryData<BackendTrack[]>(
        playlistKeys.tracks(playlistId)
      )

      // Get track data from all tracks cache (if available)
      const allTracks = queryClient.getQueryData<BackendTrack[]>(trackKeys.list()) ?? []
      const trackToAdd = allTracks.find(t => t.id === trackId)

      // Optimistically update to the new value
      if (trackToAdd) {
        queryClient.setQueryData<BackendTrack[]>(
          playlistKeys.tracks(playlistId),
          old => [...(old ?? []), trackToAdd]
        )
      }

      // Return context with the snapshotted value
      return { previousTracks, playlistId }
    },

    // If mutation fails, use the context we returned above to rollback
    onError: (_error, _variables, context) => {
      if (context?.previousTracks) {
        queryClient.setQueryData(
          playlistKeys.tracks(context.playlistId),
          context.previousTracks
        )
      }
    },

    // Always refetch after error or success to ensure we're in sync with server
    onSettled: (_data, _error, { playlistId }) => {
      queryClient.invalidateQueries({ queryKey: playlistKeys.tracks(playlistId) })
      queryClient.invalidateQueries({ queryKey: playlistKeys.detail(playlistId) })
    },
  })
}

/**
 * Mutation hook for removing a track from a playlist with optimistic updates
 */
export function useRemoveTrackFromPlaylist() {
  const backend = useBackend()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({ playlistId, trackId }: { playlistId: string; trackId: number }) => {
      await backend.removeTrackFromPlaylist(playlistId, trackId)
    },

    onMutate: async ({ playlistId, trackId }) => {
      await queryClient.cancelQueries({ queryKey: playlistKeys.tracks(playlistId) })

      const previousTracks = queryClient.getQueryData<BackendTrack[]>(
        playlistKeys.tracks(playlistId)
      )

      // Optimistically remove the track
      queryClient.setQueryData<BackendTrack[]>(
        playlistKeys.tracks(playlistId),
        old => (old ?? []).filter(t => t.id !== trackId)
      )

      return { previousTracks, playlistId }
    },

    onError: (_error, _variables, context) => {
      if (context?.previousTracks) {
        queryClient.setQueryData(
          playlistKeys.tracks(context.playlistId),
          context.previousTracks
        )
      }
    },

    onSettled: (_data, _error, { playlistId }) => {
      queryClient.invalidateQueries({ queryKey: playlistKeys.tracks(playlistId) })
      queryClient.invalidateQueries({ queryKey: playlistKeys.detail(playlistId) })
    },
  })
}

/**
 * Mutation hook for creating a new playlist
 */
export function useCreatePlaylist() {
  const backend = useBackend()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({ name, description }: { name: string; description?: string }) => {
      return backend.createPlaylist(name, description)
    },

    onSuccess: () => {
      // Invalidate playlists list to refetch
      queryClient.invalidateQueries({ queryKey: playlistKeys.all() })
    },
  })
}

/**
 * Mutation hook for deleting a playlist
 */
export function useDeletePlaylist() {
  const backend = useBackend()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (playlistId: string) => {
      await backend.deletePlaylist(playlistId)
    },

    onMutate: async (playlistId) => {
      // Cancel outgoing refetches
      await queryClient.cancelQueries({ queryKey: playlistKeys.all() })

      // Snapshot previous value
      const previousPlaylists = queryClient.getQueryData<BackendPlaylist[]>(
        playlistKeys.list()
      )

      // Optimistically remove from list
      queryClient.setQueryData<BackendPlaylist[]>(
        playlistKeys.list(),
        old => (old ?? []).filter(p => p.id !== playlistId)
      )

      return { previousPlaylists }
    },

    onError: (_error, _variables, context) => {
      if (context?.previousPlaylists) {
        queryClient.setQueryData(playlistKeys.list(), context.previousPlaylists)
      }
    },

    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: playlistKeys.all() })
    },
  })
}
