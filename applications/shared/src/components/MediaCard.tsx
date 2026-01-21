/**
 * Unified MediaCard component - works for albums, artists, and playlists
 * Supports different shapes (square for albums/playlists, circle for artists)
 * Has play/pause functionality based on current playback context
 */

import { useState, useEffect, memo, type ReactNode } from 'react'
import { Play, Pause, Disc3, Users, ListMusic } from 'lucide-react'
import { useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { ArtworkImage } from './ArtworkImage'
import { ProgressiveImage } from './ProgressiveImage'
import { usePlayerStore } from '../stores/player'
import { usePlayerCommands, type QueueContext } from '../contexts/PlayerCommandsContext'
import { useBackend } from '../contexts/BackendContext'
import { usePlatform } from '../contexts/PlatformContext'
import { getDeduplicatedTracks } from '../utils/trackGrouping'
import { ArtistLink } from './ArtistLink'
import { debug } from '../utils/debug'

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
  /** Artist ID for albums - enables clickable artist name */
  artistId?: number
  /** Cover art URL for non-desktop environments */
  coverUrl?: string
  /** Card width class (default: w-full for responsive grid) */
  className?: string
  /** Additional info like year */
  additionalInfo?: string
  /** Priority: if true, loads artwork immediately without lazy loading. Use for above-the-fold items (first ~20-30 items) */
  priority?: boolean
  /** OPTIMIZATION: If provided, skips the redundant context check and uses this value instead.
   * Parent components should fetch context once and pass it down to avoid N queries for N cards. */
  isActiveContext?: boolean
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

const MediaCardComponent = ({
  type,
  id,
  title,
  subtitle,
  artistId,
  coverUrl,
  className = 'w-40',
  additionalInfo,
  priority = false,
  isActiveContext: isActiveContextProp,
}: MediaCardProps) => {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { isPlaying, currentTrack } = usePlayerStore()
  const commands = usePlayerCommands()
  const backend = useBackend()
  const { isDesktop } = usePlatform()
  const [isActiveContextState, setIsActiveContextState] = useState(false)

  // Debug logging for album cards
  if (type === 'album') {
    debug.log('[MediaCard] Album card props:', { id, title, subtitle, artistId, type })
  }

  const isCircle = type === 'artist'
  const FallbackIcon = getFallbackIcon(type)

  // OPTIMIZATION: If parent provides isActiveContext prop, use it. Otherwise fetch.
  // This allows parent components to fetch context once and pass down to avoid N queries for N cards.
  const isActiveContext = isActiveContextProp ?? isActiveContextState

  // Check if this entity is the current playback context (regardless of play/pause state)
  // Only runs if parent didn't provide isActiveContext prop
  useEffect(() => {
    // Skip fetching if prop is provided
    if (isActiveContextProp !== undefined) {
      return
    }

    const checkContext = async () => {
      try {
        const contexts = await backend.getRecentContexts(1)
        const context = contexts[0]
        const isActive =
          context?.contextType === type &&
          context?.contextId === String(id)
        setIsActiveContextState(isActive)
      } catch {
        setIsActiveContextState(false)
      }
    }

    checkContext()
  }, [id, type, backend, currentTrack, isActiveContextProp]) // Re-check when track changes

  const handleClick = () => {
    navigate(getRoute(type, id))
  }

  const handlePlayPause = async (e: React.MouseEvent) => {
    e.stopPropagation()

    // If this context is active AND there's a track loaded, use pause/resume logic
    // CRITICAL: Check currentTrack to prevent resume on empty player (fixes first play ignored bug)
    if (isActiveContext && currentTrack) {
      try {
        if (isPlaying) {
          await commands.pausePlayback()
        } else {
          await commands.resumePlayback()
        }
      } catch (err) {
        debug.error(`[MediaCard] Failed to pause/resume:`, err)
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
        debug.warn(`[MediaCard] No playable tracks found for ${type} ${id}`)
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

      // Build queue context for lazy loading
      let context: QueueContext | undefined
      switch (type) {
        case 'album':
          context = {
            type: 'Album',
            albumId: Number(id),
            totalCount: queue.length,
          }
          break
        case 'artist':
          context = {
            type: 'Artist',
            artistId: Number(id),
            totalCount: queue.length,
          }
          break
        case 'playlist':
          // Playlists need owner_id - we don't have it here, so skip context
          // This is fine since playlist pages use their own handlers
          context = undefined
          break
      }

      await commands.playQueue(queue, 0, context)
    } catch (err) {
      debug.error(`[MediaCard] Failed to play ${type}:`, err)
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
          priority={priority}
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
    <div className={`group ${className}`}>
      {/* Artwork - always clickable */}
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

      {/* Title - always clickable */}
      <p
        className={`font-medium truncate group-hover:text-primary transition-colors cursor-pointer ${isCircle ? 'text-center' : ''}`}
        title={title}
        onClick={handleClick}
      >
        {title}
      </p>

      {/* Subtitle - independent element, NOT nested in album click handler */}
      {subtitle && type === 'album' ? (
        // Album card - artist link is completely independent, NO onClick on parent
        <p className={`text-sm text-muted-foreground ${isCircle ? 'text-center' : ''}`}>
          <ArtistLink
            artistId={artistId}
            artistName={subtitle}
            className="text-sm text-muted-foreground hover:text-foreground hover:underline"
          />
          {additionalInfo && (
            <span className="cursor-default"> • {additionalInfo}</span>
          )}
        </p>
      ) : subtitle ? (
        // Artist or playlist - entire subtitle navigates to that entity
        <p
          className={`text-sm text-muted-foreground cursor-pointer ${isCircle ? 'text-center' : ''}`}
          title={subtitle}
          onClick={handleClick}
        >
          {subtitle}
          {additionalInfo && ` • ${additionalInfo}`}
        </p>
      ) : null}
    </div>
  )
}

export const MediaCard = memo(MediaCardComponent)
MediaCard.displayName = 'MediaCard'
