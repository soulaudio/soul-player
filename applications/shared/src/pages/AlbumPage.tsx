/**
 * Shared AlbumPage - works on both desktop and marketing demo
 * Uses BackendContext for data and PlatformContext for conditional features
 */

import { useState, useCallback } from 'react'
import { useParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useNavigateWithHistory } from '../hooks/useNavigateWithHistory'
import { ArrowLeft, Play, Clock, Disc3, Pencil, ListPlus } from 'lucide-react'
import { TrackList, type Track } from '../components/TrackList'
import { TrackMenu } from '../components/TrackMenu'
import { ArtworkImage } from '../components/ArtworkImage'
import { EditArtworkDialog } from '../components/EditArtworkDialog'
import { AddToPlaylistDialog } from '../components/AddToPlaylistDialog'
import { Dialog } from '../components/ui/Dialog'
import { ArtistLink } from '../components/ArtistLink'
import { SkeletonDetailPage } from '../components/SkeletonDetailPage'
import { useBackend, type BackendTrack } from '../contexts/BackendContext'
import { usePlayerCommands, type QueueTrack, type QueueContext } from '../contexts/PlayerCommandsContext'
import { usePlatform } from '../contexts/PlatformContext'
import { useAlbumWithTracks } from '../hooks/queries/useAlbumQueries'
import { useDeleteTrack } from '../hooks/queries/useTrackMutations'
import { getDeduplicatedTracks } from '../utils/trackGrouping'
import { debug } from '../utils/debug';

export function AlbumPage() {
  const { t } = useTranslation()
  const { id } = useParams<{ id: string }>()
  const { goBack, hasHistory } = useNavigateWithHistory()
  const { isDesktop, features } = usePlatform()
  const backend = useBackend()
  const commands = usePlayerCommands()

  // React Query hook - replaces manual loading state
  const albumId = id ? parseInt(id, 10) : 0
  const { album, tracks = [], isLoading, isError, error } = useAlbumWithTracks(albumId)
  const deleteTrackMutation = useDeleteTrack()

  const [editArtworkOpen, setEditArtworkOpen] = useState(false)
  const [artworkVersion, setArtworkVersion] = useState(0)
  const [lightboxOpen, setLightboxOpen] = useState(false)

  // Add to playlist dialog state
  const [selectedTrackForPlaylist, setSelectedTrackForPlaylist] = useState<{
    id: number
    title: string
  } | null>(null)
  const [albumForPlaylist, setAlbumForPlaylist] = useState(false)

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
        albumId: album?.id,
        filePath: t.file_path!,
        durationSeconds: t.duration_seconds || null,
        trackNumber: t.track_number || null,
      }))
    },
    [album?.id]
  )

  // Build queue callback for TrackList
  const buildQueue = useCallback(
    (_allTracks: Track[], _clickedTrack: Track, _clickedIndex: number): QueueTrack[] => {
      // Return the full queue in original order
      // The startIndex passed to playQueue() will determine which track plays first
      return buildQueueFromTracks(tracks)
    },
    [buildQueueFromTracks, tracks]
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
      debug.error('[AlbumPage] Failed to add track to play next:', error)
    }
  }, [commands, toQueueTrack])

  const handleAddToQueue = useCallback(async (track: BackendTrack) => {
    try {
      const queueTrack = toQueueTrack(track)
      await commands.addToQueueEnd(queueTrack)
    } catch (error) {
      debug.error('[AlbumPage] Failed to add track to queue:', error)
    }
  }, [commands, toQueueTrack])

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

      // Build queue context for lazy loading
      const context: QueueContext = {
        type: 'Album',
        albumId: album!.id,
        totalCount: queue.length,
      }

      await commands.playQueue(queue, 0, context)
    } catch (err) {
      debug.error('Failed to play all tracks:', err)
    }
  }

  // Unused - artist click is handled by ArtistLink component
  // const handleArtistClick = () => {
  //   if (album?.artist_id) {
  //     navigate(`/artists/${album.artist_id}`)
  //   }
  // }

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

  // Loading state - use skeleton
  if (isLoading) {
    return <SkeletonDetailPage type="album" />
  }

  // Error state
  if (isError || !album) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center text-destructive">
          <p className="font-medium mb-2">
            {error instanceof Error ? error.message : t('album.notFound')}
          </p>
          <button
            onClick={() => goBack('/albums')}
            className="mt-4 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:opacity-[var(--hover-button-opacity)] transition-opacity"
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
    <div className="h-full flex flex-col overflow-hidden" data-testid="album-detail-page">
      {/* Scrollable Content */}
      <div className="flex-1 overflow-y-auto pr-6">
        {/* Header */}
        <div className="mb-6">
        <button
          onClick={() => goBack('/albums')}
          className="flex items-center gap-2 text-muted-foreground hover:opacity-[var(--hover-text-opacity)] transition-opacity duration-[var(--transition-duration)] mb-4"
        >
          <ArrowLeft className="w-4 h-4" />
          <span>{hasHistory ? t('common.back') : t('album.backToAlbums')}</span>
        </button>

        <div className="flex items-start gap-6">
          {/* Album Cover */}
          <div
            className="group relative w-48 h-48 bg-muted rounded-lg overflow-hidden shadow-lg flex-shrink-0 flex items-center justify-center cursor-pointer"
            onClick={() => setLightboxOpen(true)}
            role="button"
            tabIndex={0}
            onKeyDown={(e) => e.key === 'Enter' && setLightboxOpen(true)}
          >
            {hasDesktopArtwork ? (
              <ArtworkImage
                key={artworkVersion}
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
            {/* Hover overlay hint */}
            <div className="absolute inset-0 bg-black/30 opacity-0 group-hover:opacity-100 transition-opacity" />
          </div>

          {/* Album Info */}
          <div className="flex-1">
            <p className="text-sm text-muted-foreground uppercase tracking-wider mb-1">
              {t('library.album')}
            </p>
            <h1 className="text-4xl font-bold mb-2" data-testid="album-title">{album.title}</h1>
            <p className="text-lg mb-2" data-testid="album-artist">
              <ArtistLink
                artistId={album.artist_id}
                artistName={album.artist_name}
                className="text-lg hover:text-primary"
              />
              {album.year && (
                <span className="text-muted-foreground"> • {album.year}</span>
              )}
            </p>
            <p className="text-sm text-muted-foreground flex items-center gap-2 mb-4" data-testid="album-track-count">
              <Clock className="w-4 h-4" />
              {t('library.tracks', { count: tracks.length })} • {formatDuration(totalDuration)}
            </p>

            <div className="flex items-center gap-3">
              <button
                onClick={handlePlayAll}
                onMouseDown={(e) => e.preventDefault()} // Prevent focus on click to avoid space key conflict
                disabled={tracks.filter(t => t.file_path).length === 0}
                data-testid="album-play-all-button"
                className="flex items-center gap-2 px-6 py-3 bg-primary text-primary-foreground rounded-full hover:opacity-[var(--hover-button-opacity)] transition-opacity disabled:opacity-[var(--disabled-opacity)]"
              >
                <Play className="w-5 h-5" fill="currentColor" />
                <span>{t('common.playAll')}</span>
              </button>

              {features.canCreatePlaylists && (
                <button
                  onClick={() => setAlbumForPlaylist(true)}
                  onMouseDown={(e) => e.preventDefault()} // Prevent focus on click to avoid space key conflict
                  data-testid="album-page-add-to-playlist"
                  className="flex items-center px-4 py-3 rounded-full border border-border hover:bg-foreground/[var(--hover-bg-opacity)] transition-colors duration-[var(--transition-duration)]"
                  aria-label={t('playlist.addAlbumToPlaylist', 'Add Album to Playlist')}
                  title={t('playlist.addAlbumToPlaylist', 'Add Album to Playlist')}
                >
                  <ListPlus className="w-5 h-5" />
                </button>
              )}
            </div>
          </div>
        </div>
        </div>

        {/* Track List */}
        <TrackList
          tracks={tracks.map(t => ({
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
          virtualized={tracks.length > 50}
          virtualItemSize={56}
          showTrackNumber={true}
          renderMenu={(track) => {
            const backendTrack = tracks.find(t => t.id === track.id)
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
      </div>

      {/* Artwork Lightbox */}
      <Dialog open={lightboxOpen} onClose={() => setLightboxOpen(false)}>
        <div onClick={(e) => e.stopPropagation()} className="flex flex-col items-start gap-3">
          <div className="rounded-xl overflow-hidden shadow-2xl bg-muted" style={{ width: '85vh', maxWidth: '85vw', maxHeight: '85vh' }}>
            {hasDesktopArtwork ? (
              <ArtworkImage
                key={artworkVersion}
                albumId={album.id}
                alt={album.title}
                className="w-full h-full object-contain"
                fallbackClassName="w-96 h-96 flex items-center justify-center"
                priority
              />
            ) : coverUrl ? (
              <img
                src={coverUrl}
                alt={album.title}
                className="w-full h-full object-contain"
              />
            ) : (
              <div className="w-96 h-96 flex items-center justify-center">
                <Disc3 className="w-24 h-24 text-muted-foreground" />
              </div>
            )}
          </div>
          {isDesktop && (
            <button
              onClick={() => { setLightboxOpen(false); setEditArtworkOpen(true) }}
              className="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
            >
              <Pencil className="w-4 h-4" />
              <span>{t('artwork.editArtwork', 'Edit Artwork')}</span>
            </button>
          )}
        </div>
      </Dialog>

      {/* Edit Artwork Dialog */}
      <EditArtworkDialog
        open={editArtworkOpen}
        onClose={() => setEditArtworkOpen(false)}
        entityType="album"
        entityId={String(album.id)}
        entityName={album.title}
        currentArtworkUrl={coverUrl}
        onArtworkChanged={() => setArtworkVersion(v => v + 1)}
      />

      {/* Add to Playlist Dialog (Desktop only) */}
      {features.canCreatePlaylists && selectedTrackForPlaylist && (
        <AddToPlaylistDialog
          open={!!selectedTrackForPlaylist}
          onClose={() => setSelectedTrackForPlaylist(null)}
          mode="track"
          trackId={selectedTrackForPlaylist.id}
          trackTitle={selectedTrackForPlaylist.title}
        />
      )}

      {/* Add Album to Playlist Dialog */}
      {features.canCreatePlaylists && album && albumForPlaylist && (
        <AddToPlaylistDialog
          open={albumForPlaylist}
          onClose={() => setAlbumForPlaylist(false)}
          mode="entity"
          entityType="album"
          entityId={album.id}
          entityName={album.title}
        />
      )}
    </div>
  )
}
