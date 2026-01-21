/**
 * TracksPage - displays all tracks with search
 */

import { useState, useCallback, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { Music } from 'lucide-react'
import { TrackList, type Track } from '../components/TrackList'
import { TrackMenu } from '../components/TrackMenu'
import { AddToPlaylistDialog } from '../components/AddToPlaylistDialog'
import { LibraryPageLayout } from '../components/LibraryPageLayout'
import { SkeletonGrid } from '../components/SkeletonGrid'
import type { BackendTrack } from '../contexts/BackendContext'
import { usePlayerCommands, type QueueTrack } from '../contexts/PlayerCommandsContext'
import { removeConsecutiveDuplicates } from '../utils/queue'
import { useTracks } from '../hooks/queries/useLibraryQueries'
import { useDatabaseHealth } from '../hooks/queries/useLibraryQueries'
import { useDeleteTrack } from '../hooks/queries/useTrackMutations'
import { debug } from '../utils/debug';

export function TracksPage() {
  const { t } = useTranslation()
  const commands = usePlayerCommands()

  const [searchQuery, setSearchQuery] = useState('')

  // Add to playlist dialog state
  const [selectedTrackForPlaylist, setSelectedTrackForPlaylist] = useState<{
    id: number
    title: string
  } | null>(null)

  // Fetch data using React Query hooks
  const { data: tracks = [], isLoading, isError, error } = useTracks()
  const { data: health } = useDatabaseHealth()
  const deleteTrackMutation = useDeleteTrack()

  // Health warning from database health check
  const healthWarning = health?.issues.length ? health.issues.join(' ') : null

  // Filter tracks by search
  const filteredTracks = useMemo(() => {
    if (!searchQuery.trim()) return tracks
    const query = searchQuery.toLowerCase()
    return tracks.filter(
      t =>
        t.title?.toLowerCase().includes(query) ||
        (t.artist_name || '').toLowerCase().includes(query) ||
        (t.album_title || '').toLowerCase().includes(query)
    )
  }, [tracks, searchQuery])

  // Build queue from tracks (optimized: only first 50 tracks for immediate playback)
  const buildQueueFromTracks = useCallback((
    libraryTracks: BackendTrack[],
    clickedTrack: Track,
    clickedIndex: number
  ): QueueTrack[] => {
    const validClickedIndex = libraryTracks.findIndex(t => t.id === clickedTrack.id)
    const actualIndex = validClickedIndex !== -1 ? validClickedIndex : clickedIndex

    // For large libraries, only build queue for first 50 tracks to avoid lag
    // Backend will lazy-load more tracks as needed
    const INITIAL_QUEUE_SIZE = 50
    const totalTracks = libraryTracks.length
    const shouldLimitQueue = totalTracks > INITIAL_QUEUE_SIZE

    let tracksToQueue: BackendTrack[]
    if (shouldLimitQueue) {
      // Only take first 50 tracks starting from clicked position
      tracksToQueue = libraryTracks.slice(actualIndex, actualIndex + INITIAL_QUEUE_SIZE)
    } else {
      // Small library - build full queue
      tracksToQueue = [
        ...libraryTracks.slice(actualIndex),
        ...libraryTracks.slice(0, actualIndex),
      ]
    }

    const queue = tracksToQueue.map((t): QueueTrack => ({
      trackId: String(t.id),
      title: t.title || 'Unknown',
      artist: t.artist_name || 'Unknown Artist',
      album: t.album_title || null,
      albumId: t.album_id,
      filePath: t.file_path || '',
      durationSeconds: t.duration_seconds || null,
      trackNumber: t.track_number || null,
    }))

    return removeConsecutiveDuplicates(
      queue.filter(t => t.filePath !== ''),
      'trackId'
    )
  }, [])

  // Build queue callback for TrackList
  const buildQueue = useCallback(
    (_allTracks: Track[], clickedTrack: Track, clickedIndex: number): QueueTrack[] => {
      return buildQueueFromTracks(filteredTracks, clickedTrack, clickedIndex)
    },
    [buildQueueFromTracks, filteredTracks]
  )

  // Convert BackendTrack to QueueTrack
  const toQueueTrack = useCallback((track: BackendTrack): QueueTrack => ({
    trackId: String(track.id),
    title: track.title || 'Unknown',
    artist: track.artist_name || 'Unknown Artist',
    album: track.album_title || null,
    albumId: track.album_id,
    filePath: track.file_path || '',
    durationSeconds: track.duration_seconds || null,
    trackNumber: track.track_number || null,
    coverArtPath: track.cover_art_path,
  }), [])

  // Queue operation handlers
  const handlePlayNext = useCallback(async (track: BackendTrack) => {
    try {
      const queueTrack = toQueueTrack(track)
      await commands.addPlayNext(queueTrack)
    } catch (error) {
      debug.error('[TracksPage] Failed to add track to play next:', error)
    }
  }, [commands, toQueueTrack])

  const handleAddToQueue = useCallback(async (track: BackendTrack) => {
    try {
      const queueTrack = toQueueTrack(track)
      await commands.addToQueueEnd(queueTrack)
    } catch (error) {
      debug.error('[TracksPage] Failed to add track to queue:', error)
    }
  }, [commands, toQueueTrack])

  // Queue context for lazy loading (currently disabled pending backend implementation)
  // const queueContext = useMemo<QueueContext | undefined>(() => {
  //   // Only use lazy loading for non-filtered views with > 100 tracks
  //   if (searchQuery.trim() || filteredTracks.length <= 100) {
  //     return undefined
  //   }
  //   return {
  //     type: 'AllTracks',
  //     userId: 1, // Default user ID for desktop
  //     totalCount: filteredTracks.length,
  //   }
  // }, [searchQuery, filteredTracks.length])

  // Show error in LibraryPageLayout if present
  const errorContent = isError ? (
    <div className="flex items-center justify-center py-12">
      <div className="text-center text-destructive">
        <p className="font-medium mb-2">{t('library.loadFailed')}</p>
        <p className="text-sm">{error instanceof Error ? error.message : 'Failed to load tracks'}</p>
      </div>
    </div>
  ) : null

  return (
    <LibraryPageLayout
      searchQuery={searchQuery}
      setSearchQuery={setSearchQuery}
      itemCount={tracks.length}
      searchPlaceholderKey="library.search.tracksWithCount"
      healthWarning={healthWarning}
      isLoading={isLoading}
      itemType="track"
      gridClass="grid-cols-1"
      cacheKey="library-tracks-count"
    >
      {isLoading ? (
        <SkeletonGrid count={20} type="track" gridClass="grid-cols-1" />
      ) : errorContent || (filteredTracks.length > 0 ? (
        <TrackList
          tracks={filteredTracks.map(t => ({
            id: t.id,
            title: String(t.title || 'Unknown'),
            artist: t.artist_name,
            artistId: t.artist_id,
            album: t.album_title,
            albumId: t.album_id,
            duration: t.duration_seconds,
            trackNumber: t.track_number,
            isAvailable: !!t.file_path,
            format: t.file_format,
            bitrate: t.bit_rate,
            sampleRate: t.sample_rate,
            channels: t.channels,
          }))}
          buildQueue={buildQueue}
          virtualized={filteredTracks.length > 100}
          virtualItemSize={56}
          renderMenu={(track) => {
            const backendTrack = filteredTracks.find(t => t.id === track.id)
            if (!backendTrack) return null
            return (
              <TrackMenu
                track={backendTrack}
                onPlayNext={() => handlePlayNext(backendTrack)}
                onAddToQueue={() => handleAddToQueue(backendTrack)}
                onAddToPlaylist={() => {
                  setSelectedTrackForPlaylist({
                    id: backendTrack.id,
                    title: backendTrack.title,
                  })
                }}
                onDelete={() => {
                  deleteTrackMutation.mutate(backendTrack.id)
                }}
              />
            )
          }}
        />
      ) : (
        <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
          <Music className="w-12 h-12 mb-4 opacity-50" />
          <p className="font-medium">
            {searchQuery ? t('library.noSearchResults') : t('library.noTracks')}
          </p>
          <p className="text-sm mt-1">
            {searchQuery ? t('library.tryDifferentSearch') : t('library.addTracks')}
          </p>
        </div>
      ))}

      {/* Add to Playlist Dialog */}
      {selectedTrackForPlaylist && (
        <AddToPlaylistDialog
          open={true}
          trackId={selectedTrackForPlaylist.id}
          trackTitle={selectedTrackForPlaylist.title}
          onClose={() => setSelectedTrackForPlaylist(null)}
        />
      )}
    </LibraryPageLayout>
  )
}
