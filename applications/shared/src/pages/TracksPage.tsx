/**
 * TracksPage - displays all tracks with search
 */

import { useState, useEffect, useCallback, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { Music, Search, X } from 'lucide-react'
import { TrackList, type Track } from '../components/TrackList'
import { FeatureGate } from '../contexts/PlatformContext'
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

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <div className="animate-spin w-8 h-8 border-4 border-primary border-t-transparent rounded-full mx-auto mb-4"></div>
          <p className="text-muted-foreground">{t('common.loading')}</p>
        </div>
      </div>
    )
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-full">
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
    )
  }

  return (
    <div className="h-full flex flex-col">
      {/* Health warning */}
      <FeatureGate feature="hasHealthCheck">
        {healthWarning && (
          <div className="mb-4 p-4 bg-yellow-500/10 border border-yellow-500/20 rounded-lg">
            <div className="flex items-start gap-3">
              <div className="flex-shrink-0 w-5 h-5 rounded-full bg-yellow-500/20 flex items-center justify-center mt-0.5">
                <span className="text-yellow-600 dark:text-yellow-400 text-sm">!</span>
              </div>
              <div className="flex-1">
                <p className="text-sm text-yellow-800 dark:text-yellow-200 font-medium">
                  {t('library.databaseIssue')}
                </p>
                <p className="text-sm text-yellow-700 dark:text-yellow-300 mt-1">
                  {healthWarning}
                </p>
              </div>
            </div>
          </div>
        )}
      </FeatureGate>

      {/* Header */}
      <div className="mb-4">
        <h1 className="text-2xl sm:text-3xl font-bold">{t('library.tab.tracks')}</h1>
        <p className="text-muted-foreground text-sm mt-1">
          {tracks.length} {t('library.tracks')}
        </p>
      </div>

      {/* Search */}
      <div className="flex items-center gap-4 mb-4">
        <div className="relative flex-1 sm:max-w-md">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder={t('library.search.tracks')}
            className="w-full pl-10 pr-4 py-2 rounded-lg bg-muted border border-transparent focus:border-primary focus:outline-none text-sm"
          />
          {searchQuery && (
            <button
              onClick={() => setSearchQuery('')}
              className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
            >
              <X className="w-4 h-4" />
            </button>
          )}
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto">
        {filteredTracks.length > 0 ? (
          <TrackList
            tracks={filteredTracks.map(t => ({
              id: t.id,
              title: String(t.title || 'Unknown'),
              artist: t.artist_name,
              album: t.album_title,
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
        )}
      </div>
    </div>
  )
}
