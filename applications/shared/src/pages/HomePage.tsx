/**
 * Shared HomePage - works on both desktop and marketing demo
 * Uses BackendContext for data operations
 */

import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { Play, Pause, SkipBack, SkipForward, Music } from 'lucide-react'
import { ArtworkImage } from '../components/ArtworkImage'
import { MediaCard, type MediaType } from '../components/MediaCard'
import { usePlayerStore } from '../stores/player'
import { usePlayerCommands } from '../contexts/PlayerCommandsContext'
import { useBackend, type PlaybackContext } from '../contexts/BackendContext'
import { usePlatform } from '../contexts/PlatformContext'

/** Get subtitle for context type */
function getContextSubtitle(contextType: string, t: (key: string, fallback?: string) => string): string {
  switch (contextType) {
    case 'playlist':
      return t('library.playlist', 'Playlist')
    case 'artist':
      return t('library.artist', 'Artist')
    case 'genre':
      return t('library.genre', 'Genre')
    case 'tracks':
      return t('library.allTracks', 'All Tracks')
    case 'album':
      return t('library.album', 'Album')
    default:
      return ''
  }
}

/** Convert context type to MediaType */
function toMediaType(contextType: string): MediaType | null {
  if (contextType === 'album' || contextType === 'artist' || contextType === 'playlist') {
    return contextType as MediaType
  }
  return null
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

  // Filter contexts to only those that MediaCard supports
  const supportedContexts = recentContexts.filter(
    (ctx) => ctx.contextId && toMediaType(ctx.contextType) !== null
  )

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
      {supportedContexts.length > 0 && (
        <section>
          <h2 className="text-xl font-bold mb-4">{t('home.jumpBackInto')}</h2>
          <div className="flex gap-4 overflow-x-auto pb-4 -mx-2 px-2 scrollbar-thin scrollbar-thumb-muted scrollbar-track-transparent">
            {supportedContexts.map((context) => {
              const mediaType = toMediaType(context.contextType)!
              const id = mediaType === 'playlist'
                ? context.contextId!
                : parseInt(context.contextId!, 10)

              return (
                <MediaCard
                  key={context.id}
                  type={mediaType}
                  id={id}
                  title={context.contextName || t('common.unknown', 'Unknown')}
                  subtitle={getContextSubtitle(context.contextType, t)}
                  coverUrl={context.contextArtworkPath ?? undefined}
                />
              )
            })}
          </div>
        </section>
      )}
    </div>
  )
}
