/**
 * Shared ArtistPage - works on both desktop and marketing demo
 * Uses BackendContext for data and PlatformContext for conditional features
 */

import { useEffect, useState, useCallback } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { ArrowLeft, Play, Users, Disc3, Music } from 'lucide-react'
import { TrackList, type Track } from '../components/TrackList'
import { ArtworkImage } from '../components/ArtworkImage'
import { useBackend, type BackendTrack, type BackendAlbum, type BackendArtist, type QueueTrack } from '../contexts/BackendContext'
import { usePlatform } from '../contexts/PlatformContext'
import { getDeduplicatedTracks } from '../utils/trackGrouping'

// Album Card for artist page
function ArtistAlbumCard({
  album,
  onClick,
  isDesktop,
}: {
  album: BackendAlbum
  onClick: () => void
  isDesktop: boolean
}) {
  const { t } = useTranslation()

  const coverUrl = album.cover_art_path
  const hasDesktopArtwork = isDesktop && typeof album.id === 'number'

  return (
    <div
      className="group cursor-pointer border rounded-lg p-4 hover:bg-muted/50 transition-colors"
      onClick={onClick}
    >
      <div className="aspect-square bg-muted rounded-lg mb-3 flex items-center justify-center overflow-hidden">
        {hasDesktopArtwork ? (
          <ArtworkImage
            albumId={album.id}
            alt={album.title}
            className="w-full h-full object-cover"
          />
        ) : coverUrl ? (
          <img
            src={coverUrl}
            alt={album.title}
            className="w-full h-full object-cover"
          />
        ) : (
          <Disc3 className="w-12 h-12 text-muted-foreground" />
        )}
      </div>
      <h3 className="font-medium truncate">{album.title}</h3>
      <p className="text-xs text-muted-foreground mt-1">
        {album.year && `${album.year} • `}
        {album.track_count || 0} {t('library.tracks')}
      </p>
    </div>
  )
}

export function ArtistPage() {
  const { t } = useTranslation()
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const { isDesktop } = usePlatform()
  const backend = useBackend()

  const [artist, setArtist] = useState<BackendArtist | null>(null)
  const [albums, setAlbums] = useState<BackendAlbum[]>([])
  const [tracks, setTracks] = useState<BackendTrack[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [activeTab, setActiveTab] = useState<'albums' | 'tracks'>('albums')

  useEffect(() => {
    if (!id) return
    loadArtist(parseInt(id, 10))
  }, [id])

  const loadArtist = async (artistId: number) => {
    setLoading(true)
    setError(null)
    try {
      const foundArtist = await backend.getArtistById(artistId)
      if (!foundArtist) {
        setError(t('artist.notFound'))
        return
      }

      setArtist(foundArtist)

      const [artistTracks, artistAlbums] = await Promise.all([
        backend.getArtistTracks(artistId),
        backend.getArtistAlbums(artistId),
      ])

      setTracks(artistTracks)
      setAlbums(artistAlbums)
    } catch (err) {
      console.error('Failed to load artist:', err)
      setError(err instanceof Error ? err.message : 'Failed to load artist')
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
      if (artist) {
        await backend.recordContext({
          contextType: 'artist',
          contextId: String(artist.id),
          contextName: artist.name,
          contextArtworkPath: null,
        })
      }

      await backend.playQueue(queue, 0)
    } catch (err) {
      console.error('Failed to play all tracks:', err)
    }
  }

  // Navigate to album
  const handleAlbumClick = (album: BackendAlbum) => {
    navigate(`/albums/${album.id}`)
  }

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
  if (error || !artist) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center text-destructive">
          <p className="font-medium mb-2">{error || t('artist.notFound')}</p>
          <button
            onClick={() => navigate('/library?tab=artists')}
            className="mt-4 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90"
          >
            {t('common.back')}
          </button>
        </div>
      </div>
    )
  }

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="mb-6">
        <button
          onClick={() => navigate('/library?tab=artists')}
          className="flex items-center gap-2 text-muted-foreground hover:text-foreground mb-4"
        >
          <ArrowLeft className="w-4 h-4" />
          <span>{t('artist.backToArtists')}</span>
        </button>

        <div className="flex items-start gap-6">
          {/* Artist Avatar */}
          <div className="w-32 h-32 bg-muted rounded-full flex items-center justify-center flex-shrink-0">
            <Users className="w-16 h-16 text-muted-foreground" />
          </div>

          {/* Artist Info */}
          <div className="flex-1">
            <p className="text-sm text-muted-foreground uppercase tracking-wider mb-1">
              {t('library.artist')}
            </p>
            <h1 className="text-4xl font-bold mb-2">{artist.name}</h1>
            <p className="text-muted-foreground mb-4">
              {artist.album_count} {t('library.albums')} • {artist.track_count} {t('library.tracks')}
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

      {/* Tab Navigation */}
      <div className="flex items-center gap-1 bg-muted rounded-lg p-1 mb-6 w-fit">
        <button
          onClick={() => setActiveTab('albums')}
          className={`px-4 py-2 rounded-md transition-colors flex items-center gap-2 ${
            activeTab === 'albums' ? 'bg-background shadow-sm' : 'hover:bg-background/50'
          }`}
        >
          <Disc3 className="w-4 h-4" />
          <span className="text-sm font-medium">{t('library.tab.albums')}</span>
        </button>
        <button
          onClick={() => setActiveTab('tracks')}
          className={`px-4 py-2 rounded-md transition-colors flex items-center gap-2 ${
            activeTab === 'tracks' ? 'bg-background shadow-sm' : 'hover:bg-background/50'
          }`}
        >
          <Music className="w-4 h-4" />
          <span className="text-sm font-medium">{t('library.tab.tracks')}</span>
        </button>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto">
        {/* Albums Tab */}
        {activeTab === 'albums' && (
          albums.length > 0 ? (
            <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4">
              {albums.map((album) => (
                <ArtistAlbumCard
                  key={album.id}
                  album={album}
                  onClick={() => handleAlbumClick(album)}
                  isDesktop={isDesktop}
                />
              ))}
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
              <Disc3 className="w-12 h-12 mb-4 opacity-50" />
              <p className="font-medium">{t('library.noAlbums')}</p>
            </div>
          )
        )}

        {/* Tracks Tab */}
        {activeTab === 'tracks' && (
          tracks.length > 0 ? (
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
          ) : (
            <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
              <Music className="w-12 h-12 mb-4 opacity-50" />
              <p className="font-medium">{t('library.noTracks')}</p>
            </div>
          )
        )}
      </div>
    </div>
  )
}
