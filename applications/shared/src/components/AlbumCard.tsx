/**
 * Shared AlbumCard component - works on both desktop and marketing
 * Uses BackendContext for data operations
 */

import { useState, useEffect } from 'react'
import { Play, Pause, Disc3 } from 'lucide-react'
import { useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { ArtworkImage } from './ArtworkImage'
import { usePlayerStore } from '../stores/player'
import { usePlayerCommands } from '../contexts/PlayerCommandsContext'
import { useBackend } from '../contexts/BackendContext'
import { getDeduplicatedTracks } from '../utils/trackGrouping'
import { usePlatform } from '../contexts/PlatformContext'

export interface AlbumCardAlbum {
  id: number
  title: string
  artist_name?: string
  year?: number
  cover_art_path?: string
  coverUrl?: string
}

interface AlbumCardProps {
  album: AlbumCardAlbum
  /** Card width class (default: w-40) */
  className?: string
  /** Show artist and year below title */
  showArtist?: boolean
}

export function AlbumCard({ album, className = 'w-40', showArtist = true }: AlbumCardProps) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { isPlaying } = usePlayerStore()
  const commands = usePlayerCommands()
  const backend = useBackend()
  const { isDesktop } = usePlatform()
  const [isThisAlbumPlaying, setIsThisAlbumPlaying] = useState(false)

  // Check if this album is the current playback context
  useEffect(() => {
    const checkContext = async () => {
      if (!isPlaying) {
        setIsThisAlbumPlaying(false)
        return
      }

      try {
        const contexts = await backend.getRecentContexts(1)
        const context = contexts[0]
        const isActiveAlbum =
          context?.contextType === 'album' &&
          context?.contextId === String(album.id)
        setIsThisAlbumPlaying(isActiveAlbum)
      } catch {
        setIsThisAlbumPlaying(false)
      }
    }

    checkContext()
  }, [isPlaying, album.id, backend])

  const handleClick = () => {
    navigate(`/albums/${album.id}`)
  }

  const handlePlayPause = async (e: React.MouseEvent) => {
    e.stopPropagation()
    console.log('[AlbumCard] handlePlayPause triggered for album:', album.id, album.title)

    // If this album is currently playing, pause it
    if (isThisAlbumPlaying) {
      try {
        console.log('[AlbumCard] Pausing current album')
        await commands.pausePlayback()
      } catch (err) {
        console.error('[AlbumCard] Failed to pause:', err)
      }
      return
    }

    // Otherwise, play the album
    try {
      console.log('[AlbumCard] Fetching album tracks...')
      const tracks = await backend.getAlbumTracks(album.id)
      console.log('[AlbumCard] Got tracks:', tracks.length)

      // Deduplicate tracks (selects best quality version for each unique track)
      const tracksWithPath = tracks.filter((t) => t.file_path)
      console.log('[AlbumCard] Tracks with file_path:', tracksWithPath.length)

      const deduplicatedTracks = getDeduplicatedTracks(tracksWithPath)
      console.log('[AlbumCard] Deduplicated tracks:', deduplicatedTracks.length)

      if (deduplicatedTracks.length === 0) {
        console.warn('[AlbumCard] No playable tracks found!')
        return
      }

      const queue = deduplicatedTracks.map((t) => ({
        trackId: String(t.id),
        title: t.title || 'Unknown',
        artist: t.artist_name || 'Unknown Artist',
        album: t.album_title || album.title,
        filePath: t.file_path!,
        durationSeconds: t.duration_seconds || null,
        trackNumber: t.track_number || null,
      }))
      console.log('[AlbumCard] Built queue with', queue.length, 'tracks')

      // Record playback context
      console.log('[AlbumCard] Recording playback context...')
      await backend.recordContext({
        contextType: 'album',
        contextId: String(album.id),
        contextName: album.title,
        contextArtworkPath: album.cover_art_path || null,
      })

      console.log('[AlbumCard] Calling backend.playQueue...')
      await backend.playQueue(queue, 0)
      console.log('[AlbumCard] playQueue completed successfully')
    } catch (err) {
      console.error('[AlbumCard] Failed to play album:', err)
    }
  }

  // Determine image source
  const coverUrl = album.coverUrl || album.cover_art_path
  const hasDesktopArtwork = isDesktop && album.id > 0

  return (
    <div className={`flex-shrink-0 cursor-pointer group ${className}`}>
      <div
        className="aspect-square rounded-lg overflow-hidden bg-muted mb-2 shadow group-hover:shadow-md transition-shadow relative cursor-pointer"
        onClick={handleClick}
        role="button"
        tabIndex={0}
        onKeyDown={(e) => e.key === 'Enter' && handleClick()}
      >
        {hasDesktopArtwork ? (
          <ArtworkImage
            albumId={album.id}
            alt={album.title}
            className="w-full h-full object-cover group-hover:scale-105 transition-transform duration-200"
            fallbackClassName="w-full h-full flex items-center justify-center bg-muted"
          />
        ) : coverUrl ? (
          <img
            src={coverUrl}
            alt={album.title}
            className="w-full h-full object-cover group-hover:scale-105 transition-transform duration-200"
          />
        ) : (
          <div className="w-full h-full flex items-center justify-center bg-muted">
            <Disc3 className="w-12 h-12 text-muted-foreground" />
          </div>
        )}
        {/* Play/Pause button - centered, visible on hover */}
        <button
          onClick={handlePlayPause}
          className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-14 h-14 flex items-center justify-center bg-black/50 hover:bg-black/70 rounded-xl opacity-0 group-hover:opacity-100 transition-all duration-200"
          aria-label={isThisAlbumPlaying ? t('playback.pause') : t('playback.play')}
        >
          {isThisAlbumPlaying ? (
            <Pause className="w-8 h-8 text-white drop-shadow-lg" fill="currentColor" />
          ) : (
            <Play className="w-8 h-8 text-white drop-shadow-lg" fill="currentColor" />
          )}
        </button>
      </div>
      <p
        className="font-medium truncate group-hover:text-primary transition-colors"
        title={album.title}
        onClick={handleClick}
      >
        {album.title}
      </p>
      {showArtist && (
        <p
          className="text-sm text-muted-foreground truncate"
          title={album.artist_name}
          onClick={handleClick}
        >
          {album.artist_name || 'Unknown Artist'}
          {album.year && ` • ${album.year}`}
        </p>
      )}
    </div>
  )
}
