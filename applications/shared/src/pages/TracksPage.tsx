/**
 * TracksPage - displays all tracks with search
 */

import { useState, useEffect, useCallback, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { Music } from 'lucide-react'
import { TrackList, type Track } from '../components/TrackList'
import { LibraryPageLayout } from '../components/LibraryPageLayout'
import { useBackend, type BackendTrack } from '../contexts/BackendContext'
import { type QueueTrack } from '../contexts/PlayerCommandsContext'
import { removeConsecutiveDuplicates } from '../utils/queue'

export function TracksPage() {
  const { t } = useTranslation()
  const backend = useBackend()

  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [tracks, setTracks] = useState<BackendTrack[]>([])
  const [searchQuery, setSearchQuery] = useState('')
  const [healthWarning, setHealthWarning] = useState<string | null>(null)

  // Load tracks
  const loadTracks = useCallback(async () => {
    setIsLoading(true)
    setError(null)
    setHealthWarning(null)
    try {
      const [tracksData, health] = await Promise.all([
        backend.getAllTracks(),
        backend.checkDatabaseHealth(),
      ])
      setTracks(tracksData)
      if (health.issues.length > 0) {
        setHealthWarning(health.issues.join(' '))
      }
    } catch (err) {
      console.error('Failed to load tracks:', err)
      setError(err instanceof Error ? err.message : 'Failed to load tracks')
    } finally {
      setIsLoading(false)
    }
  }, [backend])

  useEffect(() => {
    loadTracks()
  }, [loadTracks])

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

  // Build queue from tracks
  const buildQueueFromTracks = useCallback((
    libraryTracks: BackendTrack[],
    clickedTrack: Track,
    clickedIndex: number
  ): QueueTrack[] => {
    const validClickedIndex = libraryTracks.findIndex(t => t.id === clickedTrack.id)
    const actualIndex = validClickedIndex !== -1 ? validClickedIndex : clickedIndex

    const queue = [
      ...libraryTracks.slice(actualIndex),
      ...libraryTracks.slice(0, actualIndex),
    ].map((t): QueueTrack => ({
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

  // Show error in LibraryPageLayout if present
  const errorContent = error ? (
    <div className="flex items-center justify-center py-12">
      <div className="text-center text-destructive">
        <p className="font-medium mb-2">{t('library.loadFailed')}</p>
        <p className="text-sm">{error}</p>
        <button
          onClick={loadTracks}
          className="mt-4 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90"
        >
          {t('common.retry')}
        </button>
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
      {errorContent || (filteredTracks.length > 0 ? (
        <div>
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
          />
        </div>
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
    </LibraryPageLayout>
  )
}
