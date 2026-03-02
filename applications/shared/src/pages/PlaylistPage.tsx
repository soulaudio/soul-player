/**
 * Shared PlaylistPage - works on both desktop and marketing demo
 * Shows playlist details with track list
 */

import { useState, useCallback } from 'react'
import { useParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useNavigateWithHistory } from '../hooks/useNavigateWithHistory'
import { ArrowLeft, Play, ListMusic, Clock, Trash2, Pencil } from 'lucide-react'
import { SkeletonDetailPage } from '../components/SkeletonDetailPage'
import { TrackList } from '../components/TrackList'
import { useBackend, type BackendTrack } from '../contexts/BackendContext'
import { usePlayerCommands, type QueueTrack, type QueueContext } from '../contexts/PlayerCommandsContext'
import { usePlatform } from '../contexts/PlatformContext'
import { usePlaylistWithTracks, usePlaylistArtwork } from '../hooks/queries/useLibraryQueries'
import { useDeleteTrack } from '../hooks/queries/useTrackMutations'
import { useDeletePlaylist } from '../hooks/queries/usePlaylistMutations'
import { ConfirmDialog } from '../components/ui/Dialog'
import { EditArtworkDialog } from '../components/EditArtworkDialog'
import { AddToPlaylistDialog } from '../components/AddToPlaylistDialog'
import { TrackMenu } from '../components/TrackMenu'
import { debug } from '../utils/debug';

export function PlaylistPage() {
  const { t } = useTranslation()
  const { id } = useParams<{ id: string }>()
  const { goBack, hasHistory } = useNavigateWithHistory()
  const backend = useBackend()
  const commands = usePlayerCommands()
  const { features, isDesktop } = usePlatform()

  // React Query hooks - replaces manual loading state
  const { playlist, tracks = [], isLoading, isError, error } = usePlaylistWithTracks(id)
  const deleteTrackMutation = useDeleteTrack()
  const deletePlaylistMutation = useDeletePlaylist()

  // Load playlist artwork separately (only for desktop)
  const { data: playlistArtworkUrl } = usePlaylistArtwork(isDesktop && id ? id : undefined)

  const [deleteConfirm, setDeleteConfirm] = useState<{ type: 'playlist' | 'track'; trackId?: number } | null>(null)
  const [editArtworkOpen, setEditArtworkOpen] = useState(false)
  const [artworkVersion, setArtworkVersion] = useState(0)

  // Add to playlist dialog state
  const [selectedTrackForPlaylist, setSelectedTrackForPlaylist] = useState<{
    id: number
    title: string
  } | null>(null)

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
      debug.error('[PlaylistPage] Failed to add track to play next:', error)
    }
  }, [commands, toQueueTrack])

  const handleAddToQueue = useCallback(async (track: BackendTrack) => {
    try {
      const queueTrack = toQueueTrack(track)
      await commands.addToQueueEnd(queueTrack)
    } catch (error) {
      debug.error('[PlaylistPage] Failed to add track to queue:', error)
    }
  }, [commands, toQueueTrack])

  const handlePlayAll = async () => {
    if (tracks.length === 0) return

    const queue = tracks
      .filter((t) => t.file_path)
      .map((t) => ({
        trackId: String(t.id),
        title: t.title || 'Unknown',
        artist: t.artist_name || 'Unknown Artist',
        album: t.album_title || null,
        albumId: t.album_id,
        filePath: t.file_path || '',
        durationSeconds: t.duration_seconds || null,
        trackNumber: t.track_number || null,
      }))

    if (queue.length === 0) return

    try {
      // Record playback context if supported
      if (features.hasPlaybackContext && playlist) {
        await backend.recordContext({
          contextType: 'playlist',
          contextId: playlist.id,
          contextName: playlist.name,
          contextArtworkPath: null,
        })
      }

      // Build queue context for lazy loading (playlists require ownerId)
      const context: QueueContext | undefined = playlist ? {
        type: 'Playlist',
        playlistId: parseInt(playlist.id, 10),
        ownerId: playlist.owner_id,
        totalCount: queue.length,
      } : undefined

      await commands.playQueue(queue, 0, context)
    } catch (err) {
      debug.error('Failed to play playlist:', err)
    }
  }

  const handleDeletePlaylist = async () => {
    if (!playlist) return

    try {
      await deletePlaylistMutation.mutateAsync(playlist.id)
      goBack('/playlists')
    } catch (err) {
      debug.error('Failed to delete playlist:', err)
    }
    setDeleteConfirm(null)
  }

  const formatDuration = (seconds: number): string => {
    const hours = Math.floor(seconds / 3600)
    const minutes = Math.floor((seconds % 3600) / 60)
    if (hours > 0) {
      return `${hours}h ${minutes}m`
    }
    return `${minutes} min`
  }

  const totalDuration = tracks.reduce((acc, t) => acc + (t.duration_seconds || 0), 0)

  // Loading state - use skeleton
  if (isLoading) {
    return <SkeletonDetailPage type="playlist" />
  }

  // Error state
  if (isError || !playlist) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center text-destructive">
          <p className="font-medium mb-2">
            {error instanceof Error ? error.message : t('playlist.notFound', 'Playlist not found')}
          </p>
          <button
            onClick={() => goBack('/playlists')}
            className="mt-4 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:opacity-[var(--hover-button-opacity)] transition-opacity"
          >
            {t('common.back', 'Back')}
          </button>
        </div>
      </div>
    )
  }

  return (
    <div data-testid="playlist-detail-page" className="h-full flex flex-col overflow-hidden">
      {/* Scrollable Content */}
      <div className="flex-1 overflow-y-auto pr-6">
        {/* Header */}
        <div className="mb-6">
        <button
          onClick={() => goBack('/playlists')}
          className="flex items-center gap-2 text-muted-foreground hover:opacity-[var(--hover-text-opacity)] transition-opacity duration-[var(--transition-duration)] mb-4"
        >
          <ArrowLeft className="w-4 h-4" />
          <span>{hasHistory ? t('common.back', 'Back') : t('playlist.backToPlaylists', 'Back to Playlists')}</span>
        </button>

        <div className="flex items-start gap-6">
          {/* Playlist Cover */}
          <div className="group relative w-48 h-48 bg-gradient-to-br from-primary/30 to-primary/5 rounded-lg flex items-center justify-center flex-shrink-0 overflow-hidden">
            {playlistArtworkUrl ? (
              <img
                key={artworkVersion}
                src={playlistArtworkUrl}
                alt={playlist.name}
                className="w-full h-full object-cover"
              />
            ) : (
              <ListMusic className="w-24 h-24 text-primary" />
            )}
            {/* Edit button overlay */}
            {isDesktop && (
              <button
                onClick={() => setEditArtworkOpen(true)}
                className="absolute inset-0 flex items-center justify-center bg-black/50 opacity-0 group-hover:opacity-100 transition-opacity"
              >
                <Pencil className="w-8 h-8 text-white" />
              </button>
            )}
          </div>

          {/* Playlist Info */}
          <div className="flex-1">
            <p className="text-sm text-muted-foreground uppercase tracking-wider mb-1">
              {t('library.playlist', 'Playlist')}
            </p>
            <h1 data-testid="playlist-title" className="text-4xl font-bold mb-2">{playlist.name}</h1>
            {playlist.description && (
              <p className="text-muted-foreground mb-2">{playlist.description}</p>
            )}
            <p className="text-sm text-muted-foreground flex items-center gap-2 mb-4">
              <Clock className="w-4 h-4" />
              {tracks.length} {t('library.tracks', 'tracks')} • {formatDuration(totalDuration)}
            </p>

            <div className="flex items-center gap-3">
              <button
                onClick={handlePlayAll}
                onMouseDown={(e) => e.preventDefault()} // Prevent focus on click to avoid space key conflict
                disabled={tracks.length === 0}
                className="flex items-center gap-2 px-6 py-3 bg-primary text-primary-foreground rounded-full hover:opacity-[var(--hover-button-opacity)] transition-opacity disabled:opacity-[var(--disabled-opacity)]"
              >
                <Play className="w-5 h-5" fill="currentColor" />
                <span>{t('common.playAll', 'Play All')}</span>
              </button>

              {features.canCreatePlaylists && (
                <button
                  data-testid="delete-playlist-button"
                  onClick={() => setDeleteConfirm({ type: 'playlist' })}
                  className="p-3 rounded-full hover:bg-destructive/10 text-destructive"
                  title={t('playlist.delete', 'Delete Playlist')}
                >
                  <Trash2 className="w-5 h-5" />
                </button>
              )}
            </div>
          </div>
        </div>
        </div>

        {/* Track List */}
        {tracks.length === 0 ? (
          <div data-testid="playlist-empty-state" className="flex flex-col items-center justify-center py-12 text-muted-foreground">
            <ListMusic className="w-12 h-12 mb-4 opacity-50" />
            <p className="font-medium">{t('playlist.empty', 'This playlist is empty')}</p>
            <p className="text-sm mt-1">{t('playlist.emptyHint', 'Add tracks from your library')}</p>
          </div>
        ) : (
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
            showTrackNumber={true}
            buildQueue={(_allTracks, clickedTrack, _clickedIndex) => {
              // Build queue from all tracks, starting at clicked position
              const queue = tracks
                .filter((t) => t.file_path)
                .map((t) => ({
                  trackId: String(t.id),
                  title: t.title || 'Unknown',
                  artist: t.artist_name || 'Unknown Artist',
                  album: t.album_title || null,
                  albumId: t.album_id,
                  filePath: t.file_path || '',
                  durationSeconds: t.duration_seconds || null,
                  trackNumber: t.track_number || null,
                }))

              // Reorder so clicked track is first
              const clickedTrackIdx = queue.findIndex((t) => t.trackId === String(clickedTrack.id))
              if (clickedTrackIdx > 0) {
                return [...queue.slice(clickedTrackIdx), ...queue.slice(0, clickedTrackIdx)]
              }
              return queue
            }}
            virtualized={tracks.length > 50}
            virtualItemSize={56}
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
        )}
      </div>

      {/* Delete confirmation dialogs */}
      <ConfirmDialog
        open={deleteConfirm?.type === 'playlist'}
        title={t('playlist.deleteConfirmTitle', 'Delete Playlist')}
        message={t('playlist.deleteConfirmMessage', `Are you sure you want to delete "${playlist.name}"? This cannot be undone.`)}
        confirmText={t('common.delete', 'Delete')}
        variant="destructive"
        onConfirm={handleDeletePlaylist}
        onClose={() => setDeleteConfirm(null)}
      />

      <ConfirmDialog
        open={deleteConfirm?.type === 'track'}
        title={t('playlist.removeTrackTitle', 'Remove Track')}
        message={t('playlist.removeTrackMessage', 'Remove this track from the playlist?')}
        confirmText={t('common.remove', 'Remove')}
        variant="destructive"
        onConfirm={() => {
          // Track removal would be handled here if supported
          setDeleteConfirm(null)
        }}
        onClose={() => setDeleteConfirm(null)}
      />

      {/* Edit Artwork Dialog */}
      <EditArtworkDialog
        open={editArtworkOpen}
        onClose={() => setEditArtworkOpen(false)}
        entityType="playlist"
        entityId={playlist.id}
        entityName={playlist.name}
        currentArtworkUrl={playlistArtworkUrl}
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
