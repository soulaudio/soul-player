/**
 * Shared HomePage - works on both desktop and marketing demo
 * Uses BackendContext for data operations
 */

import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { Play, Pause, SkipBack, SkipForward, Music, ListMusic, Users, Guitar, Library } from 'lucide-react'
import { ArtworkImage } from '../components/ArtworkImage'
import { AlbumCard } from '../components/AlbumCard'
import { usePlayerStore } from '../stores/player'
import { usePlayerCommands } from '../contexts/PlayerCommandsContext'
import { useBackend, type PlaybackContext } from '../contexts/BackendContext'
import { usePlatform } from '../contexts/PlatformContext'
import { getDeduplicatedTracks } from '../utils/trackGrouping'

type ContextType = 'album' | 'artist' | 'playlist' | 'genre' | 'tracks'

/** Get icon for context type */
function getContextIcon(contextType: ContextType) {
  switch (contextType) {
    case 'playlist':
      return ListMusic
    case 'artist':
      return Users
    case 'genre':
      return Guitar
    case 'tracks':
      return Library
    default:
      return Music
  }
}

/** Get route for context */
function getContextRoute(context: PlaybackContext): string {
  switch (context.contextType) {
    case 'playlist':
      return `/playlists/${context.contextId}`
    case 'artist':
      return `/artists/${context.contextId}`
    case 'genre':
      return `/genres/${context.contextId}`
    case 'tracks':
      return '/library'
    default:
      return '/library'
  }
}

interface ContextCardProps {
  context: PlaybackContext
}

/** Card for non-album contexts (playlist, artist, genre, tracks) */
function ContextCard({ context }: ContextCardProps) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const backend = useBackend()
  const Icon = getContextIcon(context.contextType)

  /** Get context type label */
  const getContextTypeLabel = (contextType: ContextType): string => {
    switch (contextType) {
      case 'playlist':
        return t('library.playlist', 'Playlist')
      case 'artist':
        return t('library.artist', 'Artist')
      case 'genre':
        return t('library.genre', 'Genre')
      case 'tracks':
        return t('library.allTracks', 'All Tracks')
      default:
        return ''
    }
  }

  /** Play all tracks from this context */
  const handlePlay = async (e: React.MouseEvent) => {
    e.stopPropagation()

    try {
      let tracks: Awaited<ReturnType<typeof backend.getAllTracks>> = []

      switch (context.contextType) {
        case 'playlist':
          if (context.contextId) {
            tracks = await backend.getPlaylistTracks(context.contextId)
          }
          break
        case 'artist':
          if (context.contextId) {
            tracks = await backend.getArtistTracks(parseInt(context.contextId, 10))
          }
          break
        case 'genre':
          if (context.contextId) {
            tracks = await backend.getGenreTracks(parseInt(context.contextId, 10))
          }
          break
        case 'tracks':
          tracks = await backend.getAllTracks()
          break
      }

      // Deduplicate tracks
      const deduplicatedTracks = getDeduplicatedTracks(tracks.filter((t) => t.file_path))
      if (deduplicatedTracks.length === 0) return

      const queue = deduplicatedTracks.map((t) => ({
        trackId: String(t.id),
        title: t.title || 'Unknown',
        artist: t.artist_name || 'Unknown Artist',
        album: t.album_title || null,
        filePath: t.file_path!,
        durationSeconds: t.duration_seconds || null,
        trackNumber: t.track_number || null,
      }))

      await backend.recordContext({
        contextType: context.contextType,
        contextId: context.contextId,
        contextName: context.contextName,
        contextArtworkPath: context.contextArtworkPath,
      })

      await backend.playQueue(queue, 0)
    } catch (err) {
      console.error('Failed to play context:', err)
    }
  }

  const handleClick = () => {
    navigate(getContextRoute(context))
  }

  return (
    <div className="flex-shrink-0 w-40 cursor-pointer group">
      <div
        className="w-40 h-40 rounded-lg overflow-hidden bg-muted mb-2 shadow group-hover:shadow-md transition-shadow relative cursor-pointer"
        onClick={handleClick}
        role="button"
        tabIndex={0}
        onKeyDown={(e) => e.key === 'Enter' && handleClick()}
      >
        <div className="w-full h-full flex items-center justify-center bg-gradient-to-br from-primary/20 to-primary/5 group-hover:from-primary/30 group-hover:to-primary/10 transition-colors">
          <Icon className="w-16 h-16 text-primary/60" />
        </div>
        {/* Play button */}
        <button
          onClick={handlePlay}
          className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-14 h-14 flex items-center justify-center bg-black/50 hover:bg-black/70 rounded-xl opacity-0 group-hover:opacity-100 transition-all duration-200"
          aria-label={t('playback.play')}
        >
          <Play className="w-8 h-8 text-white drop-shadow-lg" fill="currentColor" />
        </button>
      </div>
      <p className="font-medium truncate group-hover:text-primary transition-colors" onClick={handleClick}>
        {context.contextName || t('common.unknown', 'Unknown')}
      </p>
      <p className="text-sm text-muted-foreground truncate" onClick={handleClick}>
        {getContextTypeLabel(context.contextType)}
      </p>
    </div>
  )
}

export function HomePage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { currentTrack, isPlaying } = usePlayerStore()
  const { resumePlayback, pausePlayback, skipNext, skipPrevious } = usePlayerCommands()
  const backend = useBackend()
  const { isDesktop } = usePlatform()
  const [recentContexts, setRecentContexts] = useState<PlaybackContext[]>([])

  const hasPlayingTrack = currentTrack !== null

  useEffect(() => {
    // Fetch recent playback contexts for "Jump back into" section
    backend.getRecentContexts(8)
      .then((contexts) => {
        setRecentContexts(contexts)
      })
      .catch((err) => {
        console.error('Failed to fetch recent contexts:', err)
      })
  }, [backend])

  const handlePlayPause = async () => {
    if (isPlaying) {
      await pausePlayback()
    } else {
      await resumePlayback()
    }
  }

  return (
    <div className="h-full flex flex-col">
      {/* Now Playing Section */}
      <section className="mb-8">
        {hasPlayingTrack ? (
          <div className="flex items-start gap-8">
            {/* Large Album Artwork */}
            <div
              className="w-64 h-64 rounded-lg overflow-hidden bg-muted flex-shrink-0 cursor-pointer shadow-lg hover:shadow-xl transition-shadow"
              onClick={() => navigate('/now-playing')}
            >
              {isDesktop && currentTrack.id ? (
                <ArtworkImage
                  trackId={currentTrack.id}
                  coverArtPath={currentTrack.coverArtPath}
                  alt={currentTrack.album || currentTrack.title}
                  className="w-full h-full object-cover"
                  fallbackClassName="w-full h-full flex items-center justify-center bg-muted"
                />
              ) : currentTrack.coverArtPath ? (
                <img
                  src={currentTrack.coverArtPath}
                  alt={currentTrack.album || currentTrack.title}
                  className="w-full h-full object-cover"
                />
              ) : (
                <div className="w-full h-full flex items-center justify-center bg-muted">
                  <Music className="w-16 h-16 text-muted-foreground" />
                </div>
              )}
            </div>

            {/* Track Info and Controls */}
            <div className="flex flex-col justify-center py-4">
              <p className="text-sm text-muted-foreground mb-1">{t('sidebar.nowPlaying')}</p>
              <h1
                className="text-3xl font-bold mb-2 cursor-pointer hover:text-primary transition-colors"
                onClick={() => navigate('/now-playing')}
              >
                {currentTrack.title}
              </h1>
              <p className="text-lg text-muted-foreground mb-1">{currentTrack.artist}</p>
              {currentTrack.album && (
                <p className="text-sm text-muted-foreground">{currentTrack.album}</p>
              )}

              {/* Playback Controls */}
              <div className="flex items-center gap-3 mt-6">
                <button
                  onClick={skipPrevious}
                  className="p-2 rounded-full hover:bg-accent/30 transition-colors"
                  aria-label={t('playback.previous')}
                >
                  <SkipBack className="w-5 h-5" />
                </button>
                <button
                  onClick={handlePlayPause}
                  className="p-3 rounded-full bg-primary text-primary-foreground hover:bg-primary/80 transition-colors"
                  aria-label={isPlaying ? t('playback.pause') : t('playback.play')}
                >
                  {isPlaying ? <Pause className="w-5 h-5" /> : <Play className="w-5 h-5 ml-0.5" />}
                </button>
                <button
                  onClick={skipNext}
                  className="p-2 rounded-full hover:bg-accent/30 transition-colors"
                  aria-label={t('playback.next')}
                >
                  <SkipForward className="w-5 h-5" />
                </button>
              </div>
            </div>
          </div>
        ) : (
          /* Welcome Message */
          <div className="py-12">
            <div className="flex items-center gap-4 mb-4">
              <div className="w-16 h-16 rounded-xl bg-primary/10 flex items-center justify-center">
                <Music className="w-8 h-8 text-primary" />
              </div>
              <div>
                <h1 className="text-3xl font-bold">{t('home.welcome')}</h1>
                <p className="text-muted-foreground">{t('home.welcomeSubtitle')}</p>
              </div>
            </div>
          </div>
        )}
      </section>

      {/* Jump Back Into Section */}
      {recentContexts.length > 0 && (
        <section>
          <h2 className="text-xl font-bold mb-4">{t('home.jumpBackInto')}</h2>
          <div className="flex gap-4 overflow-x-auto pb-4 -mx-2 px-2 scrollbar-thin scrollbar-thumb-muted scrollbar-track-transparent">
            {recentContexts.map((context) =>
              context.contextType === 'album' && context.contextId ? (
                <AlbumCard
                  key={context.id}
                  album={{
                    id: parseInt(context.contextId, 10),
                    title: context.contextName || 'Unknown Album',
                    artist_name: undefined,
                    cover_art_path: context.contextArtworkPath ?? undefined,
                  }}
                  showArtist={false}
                />
              ) : (
                <ContextCard key={context.id} context={context} />
              )
            )}
          </div>
        </section>
      )}
    </div>
  )
}
