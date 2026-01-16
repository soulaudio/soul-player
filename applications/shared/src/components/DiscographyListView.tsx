import React, { useState, useEffect, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { useBackend } from '../contexts/BackendContext'
import { TrackList } from './TrackList'
import { ArtworkImage } from './ArtworkImage'
import type { BackendAlbum, BackendTrack } from '../contexts/BackendContext'

interface DiscographyListViewProps {
  albums: BackendAlbum[]
  onAlbumClick?: (album: BackendAlbum) => void
}

export function DiscographyListView({ albums, onAlbumClick }: DiscographyListViewProps) {
  const { t } = useTranslation()
  const backend = useBackend()
  const [albumTracksCache, setAlbumTracksCache] = useState<Map<number, BackendTrack[]>>(new Map())
  const [loadingAlbums, setLoadingAlbums] = useState<Set<number>>(new Set())

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
              console.error(`Failed to load tracks for album ${album.id}:`, error)
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
    // If clicking on the artwork or album title area, navigate to album page
    if (onAlbumClick && (e.target as HTMLElement).closest('[data-album-info]')) {
      onAlbumClick(album)
    }
  }, [onAlbumClick])

  if (albums.length === 0) {
    return (
      <div className="text-center py-12 text-neutral-400">
        {t('library.noAlbums')}
      </div>
    )
  }

  const isLoading = loadingAlbums.size > 0

  return (
    <div className="space-y-6">
      {isLoading ? (
        <div className="p-8 text-center text-neutral-400">
          {t('common.loading')}...
        </div>
      ) : (
        albums.map(album => {
          const tracks = albumTracksCache.get(album.id) || []
          const trackCount = album.track_count || tracks.length

          return (
            <div key={album.id} className="space-y-3">
              {/* Album Header */}
              <div
                className="flex items-center gap-4 cursor-pointer"
                onClick={(e) => handleAlbumClick(album, e)}
              >
                {/* Album Artwork */}
                <div data-album-info className="flex-shrink-0">
                  <div className="w-16 h-16 rounded overflow-hidden bg-neutral-800">
                    <ArtworkImage
                      albumId={album.id}
                      alt={album.title}
                      className="w-16 h-16 object-cover"
                    />
                  </div>
                </div>

                {/* Album Info */}
                <div data-album-info className="flex-1 min-w-0">
                  <h3 className="font-semibold text-white truncate hover:underline">
                    {album.title}
                  </h3>
                  <p className="text-sm text-neutral-400 truncate">
                    {album.year && `${album.year} • `}
                    {trackCount} {trackCount === 1 ? t('common.track') : t('common.tracks')}
                  </p>
                </div>
              </div>

              {/* Track List */}
              {tracks.length > 0 ? (
                <div className="ml-0">
                  <TrackList
                    tracks={tracks}
                    virtualized={false}
                    buildQueue={(tracks) => {
                      // Build queue from current album's tracks
                      // Lookup backend tracks to get file paths
                      return tracks.map(t => {
                        const backendTrack = albumTracksCache.get(album.id)?.find(bt => bt.id === t.id)
                        return {
                          trackId: String(t.id),
                          title: t.title,
                          artist: t.artist || 'Unknown Artist',
                          album: t.album || null,
                          filePath: backendTrack?.file_path || '',
                          durationSeconds: t.duration || null,
                          trackNumber: t.trackNumber || null,
                          coverArtPath: backendTrack?.cover_art_path || undefined,
                        }
                      });
                    }}
                  />
                </div>
              ) : (
                <div className="p-4 text-center text-neutral-400 text-sm">
                  {t('library.noTracks')}
                </div>
              )}
            </div>
          )
        })
      )}
    </div>
  )
}
