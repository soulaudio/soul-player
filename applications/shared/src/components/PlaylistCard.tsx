/**
 * Shared PlaylistCard component - works on both desktop and marketing
 * Uses BackendContext for data operations
 * Similar to AlbumCard but for playlists
 */

import { useState, useEffect } from 'react'
import { Play, Pause, ListMusic } from 'lucide-react'
import { useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { usePlayerStore } from '../stores/player'
import { usePlayerCommands } from '../contexts/PlayerCommandsContext'
import { useBackend, type BackendPlaylist } from '../contexts/BackendContext'

interface PlaylistCardProps {
  playlist: BackendPlaylist
  /** Card width class (default: w-40) */
  className?: string
}

export function PlaylistCard({ playlist, className = 'w-40' }: PlaylistCardProps) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { isPlaying } = usePlayerStore()
  const commands = usePlayerCommands()
  const backend = useBackend()
  const [isThisPlaylistPlaying, setIsThisPlaylistPlaying] = useState(false)

  // Check if this playlist is the current playback context
  useEffect(() => {
    const checkContext = async () => {
      if (!isPlaying) {
        setIsThisPlaylistPlaying(false)
        return
      }

      try {
        const contexts = await backend.getRecentContexts(1)
        const context = contexts[0]
        const isActivePlaylist =
          context?.contextType === 'playlist' &&
          context?.contextId === playlist.id
        setIsThisPlaylistPlaying(isActivePlaylist)
      } catch {
        setIsThisPlaylistPlaying(false)
      }
    }

    checkContext()
  }, [isPlaying, playlist.id, backend])

  const handleClick = () => {
    navigate(`/playlists/${playlist.id}`)
  }

  const handlePlayPause = async (e: React.MouseEvent) => {
    e.stopPropagation()

    // If this playlist is currently playing, pause it
    if (isThisPlaylistPlaying) {
      try {
        await commands.pausePlayback()
      } catch (err) {
        console.error('Failed to pause:', err)
      }
      return
    }

    // Otherwise, play the playlist
    try {
      const tracks = await backend.getPlaylistTracks(playlist.id)

      const playableTracks = tracks.filter((t) => t.file_path)
      if (playableTracks.length === 0) {
        return
      }

      const queue = playableTracks.map((t) => ({
        trackId: String(t.id),
        title: t.title || 'Unknown',
        artist: t.artist_name || 'Unknown Artist',
        album: t.album_title || null,
        filePath: t.file_path!,
        durationSeconds: t.duration_seconds || null,
        trackNumber: t.track_number || null,
      }))

      // Record playback context
      await backend.recordContext({
        contextType: 'playlist',
        contextId: playlist.id,
        contextName: playlist.name,
        contextArtworkPath: null,
      })

      await backend.playQueue(queue, 0)
    } catch (err) {
      console.error('Failed to play playlist:', err)
    }
  }

  return (
    <div className={`flex-shrink-0 cursor-pointer group ${className}`}>
      <div
        className="aspect-square rounded-lg overflow-hidden bg-muted mb-2 shadow group-hover:shadow-md transition-shadow relative cursor-pointer"
        onClick={handleClick}
        role="button"
        tabIndex={0}
        onKeyDown={(e) => e.key === 'Enter' && handleClick()}
      >
        {/* Playlist icon as cover */}
        <div className="w-full h-full flex items-center justify-center bg-gradient-to-br from-primary/20 to-primary/5">
          <ListMusic className="w-12 h-12 text-primary/60 group-hover:text-primary transition-colors" />
        </div>
        {/* Play/Pause button - centered, visible on hover */}
        <button
          onClick={handlePlayPause}
          className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-14 h-14 flex items-center justify-center bg-black/50 hover:bg-black/70 rounded-xl opacity-0 group-hover:opacity-100 transition-all duration-200"
          aria-label={isThisPlaylistPlaying ? t('playback.pause') : t('playback.play')}
        >
          {isThisPlaylistPlaying ? (
            <Pause className="w-8 h-8 text-white drop-shadow-lg" fill="currentColor" />
          ) : (
            <Play className="w-8 h-8 text-white drop-shadow-lg" fill="currentColor" />
          )}
        </button>
      </div>
      <p
        className="font-medium truncate group-hover:text-primary transition-colors"
        title={playlist.name}
        onClick={handleClick}
      >
        {playlist.name}
      </p>
      <p
        className="text-sm text-muted-foreground truncate"
        onClick={handleClick}
      >
        {playlist.track_count} {t('library.tracks')}
      </p>
    </div>
  )
}
