import React, { useState, useEffect, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { useBackend } from '../contexts/BackendContext'
import { type QueueTrack } from '../contexts/PlayerCommandsContext'
import { TrackList, type Track } from './TrackList'
import { TrackMenu } from './TrackMenu'
import { AddToPlaylistDialog } from './AddToPlaylistDialog'
import { ArtworkImage } from './ArtworkImage'
import type { BackendAlbum, BackendTrack } from '../contexts/BackendContext'
import type { TrackNumberDisplay } from '../hooks/useTrackNumberDisplay'
import { debug } from '../utils/debug';

interface DiscographyListViewProps {
  albums: BackendAlbum[]
  onAlbumClick?: (album: BackendAlbum) => void
  trackNumberDisplay: TrackNumberDisplay
}

export function DiscographyListView({ albums, onAlbumClick, trackNumberDisplay }: DiscographyListViewProps) {
  const { t } = useTranslation()
  const backend = useBackend()
  const [albumTracksCache, setAlbumTracksCache] = useState<Map<number, BackendTrack[]>>(new Map())
  const [loadingAlbums, setLoadingAlbums] = useState<Set<number>>(new Set())
  const [selectedTrackForPlaylist, setSelectedTrackForPlaylist] = useState<{
    id: number
    title: string
  } | null>(null)

  // Fetch all album tracks on mount
  useEffect(() => {
    const fetchAllTracks = async () => {
      const albumIds = albums.map(a => a.id)
      setLoadingAlbums(new Set(albumIds))

      try {
        const tracksPromises = albums.map(album =>
          backend.getAlbumTracks(album.id)
            .then(tracks => ({ albumId: album.id, tracks }))
            .catch(error => {
              debug.error(`Failed to load tracks for album ${album.id}:`, error)
              return { albumId: album.id, tracks: [] }
            })
        )

        const results = await Promise.all(tracksPromises)
        const newCache = new Map<number, BackendTrack[]>()
        results.forEach(({ albumId, tracks }) => {
          newCache.set(albumId, tracks)
        })
        setAlbumTracksCache(newCache)
      } finally {
        setLoadingAlbums(new Set())
      }
    }

    if (albums.length > 0) {
      fetchAllTracks()
    }
  }, [albums, backend])

  const handleAlbumClick = useCallback((album: BackendAlbum, e: React.MouseEvent) => {
    if (onAlbumClick && (e.target as HTMLElement).closest('[data-album-info]')) {
      onAlbumClick(album)
    }
  }, [onAlbumClick])

  if (albums.length === 0) {
    return (
      <div className="text-center py-12 text-muted-foreground">
        {t('library.noAlbums')}
      </div>
    )
  }

  const isLoading = loadingAlbums.size > 0
  const showTrackNumber = trackNumberDisplay !== 'hide'
  const vinylSides = trackNumberDisplay === 'vinyl'

  return (
    <div className="space-y-6">
      {isLoading ? (
        <div className="p-8 text-center text-muted-foreground">
          {t('common.loading')}...
        </div>
      ) : (
        albums.map(album => {
          const backendTracks = albumTracksCache.get(album.id) || []
          const trackCount = album.track_count || backendTracks.length
          const trackMap = new Map(backendTracks.map(bt => [bt.id, bt]))

          const tracks: Track[] = backendTracks.map(t => ({
            id: t.id,
            title: String(t.title || 'Unknown'),
            artist: t.artist_name,
            artistId: t.artist_id,
            artists: t.artists,
            album: t.album_title,
            albumId: t.album_id,
            duration: t.duration_seconds,
            trackNumber: t.track_number,
            discNumber: t.disc_number,
            isAvailable: !!t.file_path,
            format: t.file_format,
            bitrate: t.bit_rate,
            sampleRate: t.sample_rate,
            channels: t.channels,
          }))

          const buildQueue = (allTracks: Track[]): QueueTrack[] =>
            allTracks
              .filter(t => trackMap.get(Number(t.id))?.file_path)
              .map(t => {
                const bt = trackMap.get(Number(t.id))!
                return {
                  trackId: String(t.id),
                  title: bt.title || 'Unknown',
                  artist: bt.artist_name || 'Unknown Artist',
                  album: bt.album_title || null,
                  albumId: bt.album_id,
                  filePath: bt.file_path!,
                  durationSeconds: bt.duration_seconds || null,
                  trackNumber: bt.track_number || null,
                }
              })

          return (
            <div key={album.id} className="space-y-3">
              {/* Album Header */}
              <div
                className="flex items-center gap-4 cursor-pointer"
                onClick={(e) => handleAlbumClick(album, e)}
              >
                <div data-album-info className="flex-shrink-0">
                  <div className="w-16 h-16 rounded-lg overflow-hidden bg-muted">
                    <ArtworkImage
                      albumId={album.id}
                      alt={album.title}
                      className="w-full h-full object-cover"
                      fallbackClassName="w-full h-full flex items-center justify-center bg-muted"
                      fallbackIconSize="sm"
                    />
                  </div>
                </div>

                <div data-album-info className="flex-1 min-w-0">
                  <h3 className="font-semibold text-foreground truncate hover:underline">
                    {album.title}
                  </h3>
                  <p className="text-sm text-muted-foreground truncate">
                    {album.year && `${album.year} • `}
                    {t('library.tracks', { count: trackCount })}
                  </p>
                </div>
              </div>

              {/* Track List */}
              {tracks.length > 0 ? (
                <TrackList
                  tracks={tracks}
                  virtualized={false}
                  showTrackNumber={showTrackNumber}
                  vinylSides={vinylSides}
                  groupByContent={false}
                  buildQueue={(allTracks) => buildQueue(allTracks)}
                  onBeforePlay={async () => {
                    await backend.recordContext({
                      contextType: 'album',
                      contextId: String(album.id),
                      contextName: album.title,
                      contextArtworkPath: album.cover_art_path || null,
                    })
                  }}
                  renderMenu={(track) => {
                    const bt = trackMap.get(Number(track.id))
                    if (!bt) return null
                    return (
                      <TrackMenu
                        track={bt}
                        onAddToPlaylist={() => setSelectedTrackForPlaylist({ id: bt.id, title: bt.title })}
                      />
                    )
                  }}
                />
              ) : (
                <div className="p-4 text-center text-muted-foreground text-sm">
                  {t('library.noTracks')}
                </div>
              )}
            </div>
          )
        })
      )}

      {selectedTrackForPlaylist && (
        <AddToPlaylistDialog
          open={true}
          onClose={() => setSelectedTrackForPlaylist(null)}
          mode="track"
          trackId={selectedTrackForPlaylist.id}
          trackTitle={selectedTrackForPlaylist.title}
        />
      )}
    </div>
  )
}
