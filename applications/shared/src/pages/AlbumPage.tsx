/**
 * Shared AlbumPage - works on both desktop and marketing demo
 * Uses BackendContext for data and PlatformContext for conditional features
 */

import { useEffect, useState, useCallback } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { ArrowLeft, Play, Clock, Disc3 } from 'lucide-react'
import { TrackList, type Track } from '../components/TrackList'
import { ArtworkImage } from '../components/ArtworkImage'
import { useBackend, type BackendTrack, type BackendAlbum, type QueueTrack } from '../contexts/BackendContext'
import { usePlatform } from '../contexts/PlatformContext'
import { getDeduplicatedTracks } from '../utils/trackGrouping'

export function AlbumPage() {
  const { t } = useTranslation()
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const { isDesktop } = usePlatform()
  const backend = useBackend()

  const [album, setAlbum] = useState<BackendAlbum | null>(null)
  const [tracks, setTracks] = useState<BackendTrack[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!id) return
    loadAlbum(parseInt(id, 10))
  }, [id])

  const loadAlbum = async (albumId: number) => {
    setLoading(true)
    setError(null)
    try {
      const foundAlbum = await backend.getAlbumById(albumId)
      if (!foundAlbum) {
        setError(t('album.notFound'))
        return
      }

      setAlbum(foundAlbum)
      const albumTracks = await backend.getAlbumTracks(albumId)
      setTracks(albumTracks)
    } catch (err) {
      console.error('Failed to load album:', err)
      setError(err instanceof Error ? err.message : 'Failed to load album')
    } finally {
      setLoading(false)
    }
  }

  // Helper to build queue from tracks
  const buildQueueFromTracks = useCallback(
    (tracksToQueue: BackendTrack[]): QueueTrack[] => {
      // Filter tracks with files and deduplicate
      const tracksWithFiles = tracksToQueue.filter((t) => t.file_path)
      const deduplicatedTracks = getDeduplicatedTracks(tracksWithFiles)

      return deduplicatedTracks.map((t) => ({
        trackId: String(t.id),
        title: t.title || 'Unknown',
        artist: t.artist_name || 'Unknown Artist',
        album: t.album_title || null,
        filePath: t.file_path!,
        durationSeconds: t.duration_seconds || null,
        trackNumber: t.track_number || null,
      }))
    },
    []
  )

  // Build queue callback for TrackList
  const buildQueue = useCallback(
    (_allTracks: Track[], clickedTrack: Track, _clickedIndex: number): QueueTrack[] => {
      const queue = buildQueueFromTracks(tracks)
      // Reorder so clicked track is first
      const clickedTrackIdx = queue.findIndex((t) => t.trackId === String(clickedTrack.id))
      if (clickedTrackIdx > 0) {
        return [...queue.slice(clickedTrackIdx), ...queue.slice(0, clickedTrackIdx)]
      }
      return queue
    },
    [buildQueueFromTracks, tracks]
  )

  // Play all tracks
  const handlePlayAll = async () => {
    if (tracks.length === 0) return

    try {
      const queue = buildQueueFromTracks(tracks)
      if (queue.length === 0) return

      // Record playback context
      if (album) {
        await backend.recordContext({
          contextType: 'album',
          contextId: String(album.id),
          contextName: album.title,
          contextArtworkPath: album.cover_art_path || null,
        })
      }

      await backend.playQueue(queue, 0)
    } catch (err) {
      console.error('Failed to play all tracks:', err)
    }
  }

  // Navigate to artist
  const handleArtistClick = () => {
    if (album?.artist_id) {
      navigate(`/artists/${album.artist_id}`)
    }
  }

  // Format duration
  const formatDuration = (seconds: number): string => {
    const hours = Math.floor(seconds / 3600)
    const minutes = Math.floor((seconds % 3600) / 60)
    if (hours > 0) {
      return `${hours}h ${minutes}m`
    }
    return `${minutes} min`
  }

  const totalDuration = tracks.reduce(
    (acc, t) => acc + (t.duration_seconds || 0),
    0
  )

  // Loading state
  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <div className="animate-spin w-8 h-8 border-4 border-primary border-t-transparent rounded-full mx-auto mb-4"></div>
          <p className="text-muted-foreground">{t('common.loading')}</p>
        </div>
      </div>
    )
  }

  // Error state
  if (error || !album) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center text-destructive">
          <p className="font-medium mb-2">{error || t('album.notFound')}</p>
          <button
            onClick={() => navigate('/library?tab=albums')}
            className="mt-4 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90"
          >
            {t('common.back')}
          </button>
        </div>
      </div>
    )
  }

  // Get cover image source
  const coverUrl = album.cover_art_path
  const hasDesktopArtwork = isDesktop && typeof album.id === 'number'

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="mb-6">
        <button
          onClick={() => navigate('/library?tab=albums')}
          className="flex items-center gap-2 text-muted-foreground hover:text-foreground mb-4"
        >
          <ArrowLeft className="w-4 h-4" />
          <span>{t('album.backToAlbums')}</span>
        </button>

        <div className="flex items-start gap-6">
          {/* Album Cover */}
          <div className="w-48 h-48 bg-muted rounded-lg overflow-hidden shadow-lg flex-shrink-0 flex items-center justify-center">
            {hasDesktopArtwork ? (
              <ArtworkImage
                albumId={album.id}
                alt={album.title}
                className="w-full h-full object-cover"
                fallbackClassName="w-full h-full flex items-center justify-center"
              />
            ) : coverUrl ? (
              <img
                src={coverUrl}
                alt={album.title}
                className="w-full h-full object-cover"
              />
            ) : (
              <Disc3 className="w-16 h-16 text-muted-foreground" />
            )}
          </div>

          {/* Album Info */}
          <div className="flex-1">
            <p className="text-sm text-muted-foreground uppercase tracking-wider mb-1">
              {t('library.album')}
            </p>
            <h1 className="text-4xl font-bold mb-2">{album.title}</h1>
            <p className="text-lg mb-2">
              <button
                onClick={handleArtistClick}
                className="hover:underline"
                disabled={!album.artist_id}
              >
                {album.artist_name || t('common.unknownArtist')}
              </button>
              {album.year && (
                <span className="text-muted-foreground"> • {album.year}</span>
              )}
            </p>
            <p className="text-sm text-muted-foreground flex items-center gap-2 mb-4">
              <Clock className="w-4 h-4" />
              {tracks.length} {t('library.tracks')} • {formatDuration(totalDuration)}
            </p>

            <button
              onClick={handlePlayAll}
              disabled={tracks.filter(t => t.file_path).length === 0}
              className="flex items-center gap-2 px-6 py-3 bg-primary text-primary-foreground rounded-full hover:bg-primary/90 disabled:opacity-50"
            >
              <Play className="w-5 h-5" fill="currentColor" />
              <span>{t('common.playAll')}</span>
            </button>
          </div>
        </div>
      </div>

      {/* Track List */}
      <div className="flex-1 overflow-auto">
        <TrackList
          tracks={tracks.map(t => ({
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
      </div>
    </div>
  )
}
