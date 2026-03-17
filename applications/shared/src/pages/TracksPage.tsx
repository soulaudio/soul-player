/**
 * TracksPage - displays all tracks with search
 */

import { useState, useCallback, useMemo, useDeferredValue, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { Music, SlidersHorizontal, X } from 'lucide-react'
import { TrackList, type Track } from '../components/TrackList'
import { TrackMenu } from '../components/TrackMenu'
import { AddToPlaylistDialog } from '../components/AddToPlaylistDialog'
import { LibraryPageLayout } from '../components/LibraryPageLayout'
import { SkeletonGrid } from '../components/SkeletonGrid'
import { useBackend, type BackendTrack, type BackendGenre } from '../contexts/BackendContext'
import { usePlayerCommands, type QueueTrack } from '../contexts/PlayerCommandsContext'
import { cn } from '../lib/utils'
import { removeConsecutiveDuplicates } from '../utils/queue'
import { useTracks } from '../hooks/queries/useLibraryQueries'
import { useDatabaseHealth } from '../hooks/queries/useLibraryQueries'
import { useDeleteTrack } from '../hooks/queries/useTrackMutations'
import { debug } from '../utils/debug';

export function TracksPage() {
  const { t } = useTranslation()
  const backend = useBackend()
  const commands = usePlayerCommands()

  const [searchQuery, setSearchQuery] = useState('')
  const deferredSearchQuery = useDeferredValue(searchQuery)

  // Add to playlist dialog state
  const [selectedTrackForPlaylist, setSelectedTrackForPlaylist] = useState<{
    id: number
    title: string
  } | null>(null)

  const [genres, setGenres] = useState<BackendGenre[]>([])
  const [selectedGenreId, setSelectedGenreId] = useState<number | null>(null)
  const [showFilters, setShowFilters] = useState(false)
  const [genreTracks, setGenreTracks] = useState<BackendTrack[]>([])
  const [genreTracksLoading, setGenreTracksLoading] = useState(false)

  useEffect(() => {
    backend.getAllGenres().then(setGenres).catch(() => {})
  }, [backend])

  useEffect(() => {
    if (selectedGenreId === null) {
      setGenreTracks([])
      return
    }
    setGenreTracksLoading(true)
    backend.getGenreTracks(selectedGenreId)
      .then(setGenreTracks)
      .catch(() => setGenreTracks([]))
      .finally(() => setGenreTracksLoading(false))
  }, [selectedGenreId, backend])

  // Fetch data using React Query hooks
  const { data: allTracks = [], isLoading: tracksLoading, isError, error } = useTracks()
  const tracks = selectedGenreId !== null ? genreTracks : allTracks
  const isLoading = selectedGenreId !== null ? genreTracksLoading : tracksLoading

  const { data: health } = useDatabaseHealth()
  const deleteTrackMutation = useDeleteTrack()

  // Health warning from database health check
  const healthWarning = health?.issues.length ? health.issues.join(' ') : null

  const filterPanel = genres.length === 0 ? null : (
    <div className={`overflow-hidden transition-all duration-200 ${showFilters ? 'max-h-20 opacity-100' : 'max-h-0 opacity-0'}`}>
      <div className="flex flex-wrap gap-2 pt-2 pb-1">
        {genres.map((genre) => (
          <button
            key={genre.id}
            data-testid={`genre-chip-${genre.id}${selectedGenreId === genre.id ? '-active' : ''}`}
            onClick={() => setSelectedGenreId(selectedGenreId === genre.id ? null : genre.id)}
            className={cn(
              'px-3 py-1 rounded-full text-sm transition-all border',
              selectedGenreId === genre.id
                ? 'bg-primary text-primary-foreground border-primary'
                : 'bg-muted text-muted-foreground border-transparent hover:border-muted-foreground/30'
            )}
          >
            {genre.name}
          </button>
        ))}
      </div>
    </div>
  )

  const filtersButton = (
    <button
      data-testid="filter-toggle-button"
      onClick={() => {
        setShowFilters(v => !v)
        if (showFilters) setSelectedGenreId(null)
      }}
      className={cn(
        'flex items-center gap-1.5 px-3 py-2 rounded-lg text-sm transition-all border',
        showFilters || selectedGenreId !== null
          ? 'border-primary text-primary bg-primary/10'
          : 'border-transparent text-muted-foreground bg-muted hover:opacity-[var(--hover-text-opacity)]'
      )}
    >
      {selectedGenreId !== null ? <X className="w-3.5 h-3.5" /> : <SlidersHorizontal className="w-3.5 h-3.5" />}
      <span>{t('common.filters')}</span>
      {selectedGenreId !== null && (
        <span className="w-2 h-2 rounded-full bg-primary" />
      )}
    </button>
  )

  // Filter tracks by search
  const filteredTracks = useMemo(() => {
    if (!deferredSearchQuery.trim()) return tracks
    const query = deferredSearchQuery.toLowerCase()
    return tracks.filter(
      t =>
        t.title?.toLowerCase().includes(query) ||
        (t.artist_name || '').toLowerCase().includes(query) ||
        (t.album_title || '').toLowerCase().includes(query)
    )
  }, [tracks, deferredSearchQuery])

  // Build queue from tracks (optimized: only first 50 tracks for immediate playback)
  const buildQueueFromTracks = useCallback((
    libraryTracks: BackendTrack[],
    _clickedTrack: Track,
    _clickedIndex: number
  ): QueueTrack[] => {
    // Return the full queue in original order
    // The startIndex passed to playQueue() will determine which track plays first
    // Note: For large libraries, we build the full queue here. Performance optimization
    // via lazy loading should be handled at the queue/playback layer, not by reordering.
    const queue = libraryTracks.map((t): QueueTrack => ({
      trackId: String(t.id),
      title: t.title || 'Unknown',
      artist: t.artist_name || 'Unknown Artist',
      album: t.album_title || null,
      albumId: t.album_id,
      artistId: t.artist_id || undefined,
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
    artistId: track.artist_id || undefined,
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
      additionalButtons={filtersButton}
      filterPanel={filterPanel}
      filterPanelVisible={showFilters}
      isLoading={isLoading}
      itemType="track"
      gridClass="grid-cols-1"
      cacheKey="library-tracks-count"
      pageTestId="tracks-page"
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
            artists: t.artists,
            album: t.album_title,
            albumId: t.album_id,
            duration: t.duration_seconds,
            isAvailable: !!t.file_path,
            format: t.file_format,
            bitrate: t.bit_rate,
            sampleRate: t.sample_rate,
            channels: t.channels,
          }))}
          buildQueue={buildQueue}
          showAlbumArt={true}
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
        <div data-testid="empty-state" className="flex flex-col items-center justify-center py-12 text-muted-foreground">
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
          mode="track"
          trackId={selectedTrackForPlaylist.id}
          trackTitle={selectedTrackForPlaylist.title}
          onClose={() => setSelectedTrackForPlaylist(null)}
        />
      )}
    </LibraryPageLayout>
  )
}
