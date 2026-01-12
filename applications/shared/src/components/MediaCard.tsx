/**
 * Unified MediaCard component - works for albums, artists, and playlists
 * Supports different shapes (square for albums/playlists, circle for artists)
 * Has play/pause functionality based on current playback context
 */

import { useState, useEffect, type ReactNode } from 'react'
import { Play, Pause, Disc3, Users, ListMusic } from 'lucide-react'
import { useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { ArtworkImage } from './ArtworkImage'
import { ProgressiveImage } from './ProgressiveImage'
import { usePlayerStore } from '../stores/player'
import { usePlayerCommands } from '../contexts/PlayerCommandsContext'
import { useBackend } from '../contexts/BackendContext'
import { usePlatform } from '../contexts/PlatformContext'
import { getDeduplicatedTracks } from '../utils/trackGrouping'

export type MediaType = 'album' | 'artist' | 'playlist'

export interface MediaCardProps {
  /** Type of media - determines shape and behavior */
  type: MediaType
  /** Unique ID for the entity */
  id: number | string
  /** Primary display name (album title, artist name, playlist name) */
  title: string
  /** Secondary text (artist for albums, track count for playlists/artists) */
  subtitle?: string
  /** Cover art URL for non-desktop environments */
  coverUrl?: string
  /** Card width class (default: w-full for responsive grid) */
  className?: string
  /** Additional info like year */
  additionalInfo?: string
}

/** Get fallback icon for media type */
function getFallbackIcon(type: MediaType) {
  switch (type) {
    case 'artist':
      return Users
    case 'playlist':
      return ListMusic
    default:
      return Disc3
  }
}

/** Get route for media type */
function getRoute(type: MediaType, id: number | string): string {
  switch (type) {
    case 'album':
      return `/albums/${id}`
    case 'artist':
      return `/artists/${id}`
    case 'playlist':
      return `/playlists/${id}`
  }
}

export function MediaCard({
  type,
  id,
  title,
  subtitle,
  coverUrl,
  className = 'w-40',
  additionalInfo,
}: MediaCardProps) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { isPlaying, currentTrack } = usePlayerStore()
  const commands = usePlayerCommands()
  const backend = useBackend()
  const { isDesktop } = usePlatform()
  const [isActiveContext, setIsActiveContext] = useState(false)

  const isCircle = type === 'artist'
  const FallbackIcon = getFallbackIcon(type)

  // Check if this entity is the current playback context (regardless of play/pause state)
  useEffect(() => {
    const checkContext = async () => {
      try {
        const contexts = await backend.getRecentContexts(1)
        const context = contexts[0]
        const isActive =
          context?.contextType === type &&
          context?.contextId === String(id)
        setIsActiveContext(isActive)
      } catch {
        setIsActiveContext(false)
      }
    }

    checkContext()
  }, [id, type, backend, currentTrack]) // Re-check when track changes

  const handleClick = () => {
    navigate(getRoute(type, id))
  }

  const handlePlayPause = async (e: React.MouseEvent) => {
    e.stopPropagation()

    // If this context is active, use pause/resume logic (same as PlayerControls)
    if (isActiveContext) {
      try {
        if (isPlaying) {
          await commands.pausePlayback()
        } else {
          await commands.resumePlayback()
        }
      } catch (err) {
        console.error(`[MediaCard] Failed to pause/resume:`, err)
      }
      return
    }

    // Otherwise, play the entity from beginning
    try {
      let tracks: Awaited<ReturnType<typeof backend.getAllTracks>> = []

      switch (type) {
        case 'album':
          tracks = await backend.getAlbumTracks(Number(id))
          break
        case 'artist':
          tracks = await backend.getArtistTracks(Number(id))
          break
        case 'playlist':
          tracks = await backend.getPlaylistTracks(String(id))
          break
      }

      // Deduplicate tracks (selects best quality version for each unique track)
      const tracksWithPath = tracks.filter((t) => t.file_path)
      const deduplicatedTracks = getDeduplicatedTracks(tracksWithPath)

      if (deduplicatedTracks.length === 0) {
        console.warn(`[MediaCard] No playable tracks found for ${type} ${id}`)
        return
      }

      const queue = deduplicatedTracks.map((t) => ({
        trackId: String(t.id),
        title: t.title || 'Unknown',
        artist: t.artist_name || 'Unknown Artist',
        album: t.album_title || null,
        albumId: t.album_id,
        filePath: t.file_path!,
        durationSeconds: t.duration_seconds || null,
        trackNumber: t.track_number || null,
      }))

      // Record playback context
      await backend.recordContext({
        contextType: type,
        contextId: String(id),
        contextName: title,
        contextArtworkPath: coverUrl || null,
      })

      await commands.playQueue(queue, 0)
    } catch (err) {
      console.error(`[MediaCard] Failed to play ${type}:`, err)
    }
  }

  // Determine if we should use ArtworkImage (desktop with valid ID)
  const hasDesktopArtwork = isDesktop && typeof id === 'number' && id > 0

  // Render artwork content
  const renderArtwork = (): ReactNode => {
    if (hasDesktopArtwork) {
      const artworkProps = type === 'album'
        ? { albumId: id as number }
        : type === 'artist'
        ? { artistId: id as number }
        : { playlistId: String(id) }

      return (
        <ArtworkImage
          {...artworkProps}
          alt={title}
          className="w-full h-full object-cover group-hover:scale-105 transition-transform duration-200"
          fallbackClassName="w-full h-full flex items-center justify-center bg-muted"
          fallbackIcon={type === 'artist' ? 'users' : type === 'playlist' ? 'playlist' : 'music'}
          shape={isCircle ? 'circular' : 'rounded'}
        />
      )
    }

    if (coverUrl) {
      return (
        <ProgressiveImage
          src={coverUrl}
          alt={title}
          className="w-full h-full object-cover group-hover:scale-105 transition-transform duration-200"
          shape={isCircle ? 'circular' : 'rounded'}
        />
      )
    }

    // Fallback with gradient and icon
    return (
      <div className="w-full h-full flex items-center justify-center bg-gradient-to-br from-primary/20 to-primary/5 group-hover:from-primary/30 group-hover:to-primary/10 transition-colors">
        <FallbackIcon className="w-12 h-12 text-primary/60 group-hover:text-primary transition-colors" />
      </div>
    )
  }

  const shapeClasses = isCircle ? 'rounded-full' : 'rounded-lg'

  return (
    <div className={`cursor-pointer group ${className}`}>
      <div
        className={`aspect-square ${shapeClasses} overflow-hidden bg-muted mb-2 shadow group-hover:shadow-md transition-shadow relative cursor-pointer`}
        onClick={handleClick}
        role="button"
        tabIndex={0}
        onKeyDown={(e) => e.key === 'Enter' && handleClick()}
      >
        {renderArtwork()}
        {/* Play/Pause button - centered, visible on hover */}
        <button
          onClick={handlePlayPause}
          onMouseDown={(e) => e.preventDefault()} // Prevent focus on click to avoid space key conflict
          className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-14 h-14 flex items-center justify-center bg-black/50 hover:bg-black/70 rounded-xl opacity-0 group-hover:opacity-100 transition-all duration-200"
          aria-label={(isActiveContext && isPlaying) ? t('playback.pause') : t('playback.play')}
        >
          {(isActiveContext && isPlaying) ? (
            <Pause className="w-8 h-8 text-white drop-shadow-lg" fill="currentColor" />
          ) : (
            <Play className="w-8 h-8 text-white drop-shadow-lg" fill="currentColor" />
          )}
        </button>
      </div>
      <p
        className={`font-medium truncate group-hover:text-primary transition-colors ${isCircle ? 'text-center' : ''}`}
        title={title}
        onClick={handleClick}
      >
        {title}
      </p>
      {subtitle && (
        <p
          className={`text-sm text-muted-foreground truncate ${isCircle ? 'text-center' : ''}`}
          title={subtitle}
          onClick={handleClick}
        >
          {subtitle}
          {additionalInfo && ` • ${additionalInfo}`}
        </p>
      )}
    </div>
  )
}
