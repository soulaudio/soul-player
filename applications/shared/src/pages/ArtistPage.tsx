/**
 * Shared ArtistPage - works on both desktop and marketing demo
 * Uses BackendContext for data and PlatformContext for conditional features
 */

import { useEffect, useState, useCallback } from 'react'
import { useParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useNavigateWithHistory } from '../hooks/useNavigateWithHistory'
import { Play, Users, Disc3, Pencil } from 'lucide-react'
import { TrackList, type Track } from '../components/TrackList'
import { TrackMenu } from '../components/TrackMenu'
import { AlbumCard } from '../components/AlbumCard'
import { ArtworkImage } from '../components/ArtworkImage'
import { EditArtworkDialog } from '../components/EditArtworkDialog'
import { AddToPlaylistDialog } from '../components/AddToPlaylistDialog'
import { ViewToggle } from '../components/ViewToggle'
import { DiscographyListView } from '../components/DiscographyListView'
import { SkeletonDetailPage } from '../components/SkeletonDetailPage'
import { DetailPageLayout } from '../components/DetailPageLayout'
import { useBackend, type BackendTrack, type BackendAlbum } from '../contexts/BackendContext'
import { usePlayerCommands, type QueueTrack, type QueueContext } from '../contexts/PlayerCommandsContext'
import { usePlatform } from '../contexts/PlatformContext'
import { useArtistWithData, useArtistArtwork } from '../hooks/queries/useArtistQueries'
import { useDeleteTrack } from '../hooks/queries/useTrackMutations'
import { useTrackNumberDisplay } from '../hooks/useTrackNumberDisplay'
import { getDeduplicatedTracks } from '../utils/trackGrouping'
import { debug } from '../utils/debug';

export function ArtistPage() {
  const { t } = useTranslation()
  const { id } = useParams<{ id: string }>()
  const { navigate, goBack } = useNavigateWithHistory()
  const { isDesktop, features } = usePlatform()
  const backend = useBackend()
  const commands = usePlayerCommands()

  // React Query hooks - replaces manual loading state
  const artistId = id ? parseInt(id, 10) : 0
  const { artist, tracks = [], albums = [], topTracks = [], isLoading, isError, error } = useArtistWithData(artistId)
  const deleteTrackMutation = useDeleteTrack()

  // Load artist artwork separately (only for desktop)
  const { data: artistArtworkUrl } = useArtistArtwork(isDesktop ? artistId : undefined)

  const [editArtworkOpen, setEditArtworkOpen] = useState(false)
  const [artworkVersion, setArtworkVersion] = useState(0)

  const trackNumberDisplay = useTrackNumberDisplay()

  // Discography view toggle (grid or list)
  const [discographyView, setDiscographyView] = useState<'grid' | 'list'>(() => {
    if (typeof window === 'undefined') return 'grid'
    return (localStorage.getItem('artist-discography-view') as 'grid' | 'list') || 'grid'
  })

  // Persist view preference to localStorage
  useEffect(() => {
    localStorage.setItem('artist-discography-view', discographyView)
  }, [discographyView])

  // Add to playlist dialog state
  const [selectedTrackForPlaylist, setSelectedTrackForPlaylist] = useState<{
    id: number
    title: string
  } | null>(null)

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
        albumId: t.album_id,
        artistId: t.artist_id || undefined,
        filePath: t.file_path!,
        durationSeconds: t.duration_seconds || null,
        trackNumber: t.track_number || null,
      }))
    },
    []
  )

  // Build queue callback for TrackList
  // Must use topTracks (the displayed list) so the queue order matches display order.
  // TrackList passes clickedIndex from the displayed list to playQueue(), so the
  // queue and the index must be aligned on the same array.
  const buildQueue = useCallback(
    (_allTracks: Track[], _clickedTrack: Track, _clickedIndex: number): QueueTrack[] => {
      return buildQueueFromTracks(topTracks)
    },
    [buildQueueFromTracks, topTracks]
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
      debug.error('[ArtistPage] Failed to add track to play next:', error)
    }
  }, [commands, toQueueTrack])

  const handleAddToQueue = useCallback(async (track: BackendTrack) => {
    try {
      const queueTrack = toQueueTrack(track)
      await commands.addToQueueEnd(queueTrack)
    } catch (error) {
      debug.error('[ArtistPage] Failed to add track to queue:', error)
    }
  }, [commands, toQueueTrack])

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

      // Build queue context for lazy loading
      const context: QueueContext = {
        type: 'Artist',
        artistId: artist!.id,
        totalCount: queue.length,
      }

      await commands.playQueue(queue, 0, context)
    } catch (err) {
      debug.error('Failed to play all tracks:', err)
    }
  }

  // Navigate to album
  const handleAlbumClick = (album: BackendAlbum) => {
    navigate(`/albums/${album.id}`)
  }

  // Loading state
  // Loading state - use skeleton
  if (isLoading) {
    return <SkeletonDetailPage type="artist" />
  }

  // Error state
  if (isError || !artist) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center text-destructive">
          <p className="font-medium mb-2">
            {error instanceof Error ? error.message : t('artist.notFound')}
          </p>
          <button
            onClick={() => goBack('/artists')}
            className="mt-4 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:opacity-[var(--hover-button-opacity)] transition-opacity"
          >
            {t('common.back')}
          </button>
        </div>
      </div>
    )
  }

  return (
    <div data-testid="artist-detail-page" className="h-full">
      <DetailPageLayout
        header={
          <div className="flex items-start gap-4 sm:gap-6">
            {/* Artist Avatar */}
            <div className="group relative w-20 h-20 sm:w-32 sm:h-32 bg-muted rounded-full flex items-center justify-center flex-shrink-0 overflow-hidden">
              {artistArtworkUrl ? (
                <img
                  key={artworkVersion}
                  src={artistArtworkUrl}
                  alt={artist.name}
                  className="w-full h-full object-cover"
                />
              ) : isDesktop ? (
                <ArtworkImage
                  artistId={artistId}
                  alt={artist.name}
                  blurredAlbumFallback
                  fallbackClassName="w-full h-full flex items-center justify-center"
                  priority
                />
              ) : (
                <Users className="w-16 h-16 text-muted-foreground" />
              )}
              {/* Edit button overlay */}
              {isDesktop && (
                <button
                  onClick={() => setEditArtworkOpen(true)}
                  className="absolute inset-0 flex items-center justify-center bg-black/50 opacity-0 group-hover:opacity-100 transition-opacity rounded-full"
                >
                  <Pencil className="w-6 h-6 text-white" />
                </button>
              )}
            </div>

            {/* Artist Info */}
            <div className="flex-1">
              <p className="text-sm text-muted-foreground uppercase tracking-wider mb-1">
                {t('library.artist')}
              </p>
              <h1 className="text-xl sm:text-4xl font-bold mb-1 sm:mb-2" data-testid="artist-name">{artist.name}</h1>
              <p className="text-muted-foreground mb-4" data-testid="artist-stats">
                {t('library.albums', { count: artist.album_count })} • {t('library.tracks', { count: artist.track_count })}
              </p>

              <button
                onClick={handlePlayAll}
                onMouseDown={(e) => e.preventDefault()}
                disabled={tracks.filter(t => t.file_path).length === 0}
                data-testid="artist-play-all-button"
                className="flex items-center gap-2 px-4 py-2 sm:px-6 sm:py-3 text-sm sm:text-base bg-primary text-primary-foreground rounded-full hover:opacity-[var(--hover-button-opacity)] transition-opacity disabled:opacity-[var(--disabled-opacity)]"
              >
                <Play className="w-5 h-5" fill="currentColor" />
                <span>{t('common.playAll')}</span>
              </button>
            </div>
          </div>
        }
      >
        {/* Top Songs Section */}
        {topTracks.length > 0 && (
          <div className="mb-8">
            <h2 className="text-2xl font-bold mb-4">{t('artist.topSongs')}</h2>
            <TrackList
              tracks={topTracks.map(t => ({
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
              showAlbumArt={true}
              virtualized={false}
              buildQueue={buildQueue}
              renderMenu={(track) => {
                const backendTrack = topTracks.find(t => t.id === track.id)
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
        )}

        {/* Discography Section */}
        <div className="mb-0">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-2xl font-bold">{t('artist.discography')}</h2>
            <ViewToggle view={discographyView} onViewChange={setDiscographyView} />
          </div>

          {albums.length > 0 ? (
            discographyView === 'grid' ? (
              <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4">
                {albums.map((album, index) => (
                  <AlbumCard
                    key={album.id}
                    album={album}
                    priority={index < 20}
                  />
                ))}
              </div>
            ) : (
              <DiscographyListView
                albums={albums}
                onAlbumClick={handleAlbumClick}
                trackNumberDisplay={trackNumberDisplay}
              />
            )
          ) : (
            <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
              <Disc3 className="w-12 h-12 mb-4 opacity-50" />
              <p className="font-medium">{t('library.noAlbums')}</p>
            </div>
          )}
        </div>
      </DetailPageLayout>

      {/* Edit Artwork Dialog */}
      <EditArtworkDialog
        open={editArtworkOpen}
        onClose={() => setEditArtworkOpen(false)}
        entityType="artist"
        entityId={String(artist.id)}
        entityName={artist.name}
        currentArtworkUrl={artistArtworkUrl}
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
    </div>
  )
}
