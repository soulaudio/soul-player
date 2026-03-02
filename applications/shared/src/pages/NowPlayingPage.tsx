/**
 * Shared NowPlayingPage - works on both desktop and marketing demo
 * Shows current track artwork with tracklist from playback context
 */

import { useEffect, useState, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigateWithHistory } from '../hooks/useNavigateWithHistory'
import { usePlayerPlayback } from '../stores/player'
import { usePlayerCommands, usePlaybackEvents, type QueueTrack } from '../contexts/PlayerCommandsContext'
import { useBackend } from '../contexts/BackendContext'
import { usePlatform } from '../contexts/PlatformContext'
import { ArtworkImage } from '../components/ArtworkImage'
import { ArtistLink } from '../components/ArtistLink'
import { groupTracks } from '../utils/trackGrouping'
import type { TrackForGrouping, GroupedTrack } from '../utils/trackGrouping'
import {
  Music,
  Library,
  Disc3,
  ListMusic,
  Users,
  Guitar,
  ChevronDown,
} from 'lucide-react'
import { debug } from '../utils/debug'

type ContextType = 'album' | 'playlist' | 'artist' | 'genre' | 'library'

interface PlaybackContext {
  contextType: ContextType
  contextId: string | number
  contextName: string
  contextArtworkPath?: string | null
}

/** Get format badge styling */
function getFormatStyle(format: string): { bg: string; text: string } {
  const formatUpper = format.toUpperCase()

  if (formatUpper.startsWith('DSD') || formatUpper === 'DSF' || formatUpper === 'DFF') {
    return { bg: 'bg-purple-500/15', text: 'text-purple-400' }
  }
  if (['FLAC', 'ALAC', 'WAV', 'AIFF', 'APE', 'WV'].includes(formatUpper)) {
    return { bg: 'bg-blue-500/15', text: 'text-blue-400' }
  }
  if (['OPUS', 'AAC'].includes(formatUpper)) {
    return { bg: 'bg-emerald-500/15', text: 'text-emerald-400' }
  }
  if (['MP3', 'OGG', 'M4A', 'WMA'].includes(formatUpper)) {
    return { bg: 'bg-zinc-500/15', text: 'text-zinc-400' }
  }
  return { bg: 'bg-zinc-500/10', text: 'text-zinc-500' }
}

/** Format dropdown component */
function FormatDropdown({
  versions,
  activeVersion,
  onSelect,
}: {
  versions: TrackForGrouping[]
  activeVersion: TrackForGrouping
  onSelect: (track: TrackForGrouping) => void
}) {
  const [isOpen, setIsOpen] = useState(false)
  const style = getFormatStyle(activeVersion.file_format || '')

  if (versions.length <= 1) {
    return activeVersion.file_format ? (
      <span className={`text-[10px] font-medium px-1.5 py-0.5 rounded ${style.bg} ${style.text}`}>
        {activeVersion.file_format.toUpperCase()}
      </span>
    ) : null
  }

  return (
    <div className="relative">
      <button
        onClick={(e) => {
          e.stopPropagation()
          setIsOpen(!isOpen)
        }}
        className={`inline-flex items-center gap-0.5 text-[10px] font-medium px-1.5 py-0.5 rounded transition-colors ${style.bg} ${style.text} hover:opacity-[var(--hover-text-opacity)]`}
      >
        {activeVersion.file_format?.toUpperCase()}
        <ChevronDown className="w-3 h-3" />
      </button>

      {isOpen && (
        <>
          <div className="fixed inset-0 z-40" onClick={() => setIsOpen(false)} />
          <div className="absolute right-0 top-full mt-1 z-50 bg-popover border border-border rounded-md shadow-lg py-1 min-w-[140px]">
            {versions.map((version) => {
              const vStyle = getFormatStyle(version.file_format || '')
              const isActive = version.id === activeVersion.id
              const qualityInfo = []
              if (version.sample_rate) {
                qualityInfo.push(`${Math.round(version.sample_rate / 1000)}kHz`)
              }
              if (version.bit_rate) {
                qualityInfo.push(`${version.bit_rate}kbps`)
              }

              return (
                <button
                  key={version.id}
                  onClick={(e) => {
                    e.stopPropagation()
                    onSelect(version)
                    setIsOpen(false)
                  }}
                  className={`w-full px-3 py-1.5 text-left text-xs flex items-center justify-between gap-2 hover:bg-foreground/[var(--hover-bg-opacity)] ${
                    isActive ? 'bg-muted/30' : ''
                  }`}
                >
                  <span className={`font-medium ${vStyle.text}`}>
                    {version.file_format?.toUpperCase()}
                  </span>
                  {qualityInfo.length > 0 && (
                    <span className="text-muted-foreground text-[10px]">{qualityInfo.join(' ')}</span>
                  )}
                </button>
              )
            })}
          </div>
        </>
      )}
    </div>
  )
}

export function NowPlayingPage() {
  const { t } = useTranslation()
  const { navigate } = useNavigateWithHistory()
  const { features } = usePlatform()
  const { currentTrack, isPlaying } = usePlayerPlayback()
  const commands = usePlayerCommands()
  const events = usePlaybackEvents()
  const backend = useBackend()

  const [tracks, setTracks] = useState<TrackForGrouping[]>([])
  const [selectedVersions, setSelectedVersions] = useState<Map<string, TrackForGrouping>>(new Map())
  const [playbackContext, setPlaybackContext] = useState<PlaybackContext | null>(null)
  const [loading, setLoading] = useState(false)

  // Fetch current playback context (desktop only)
  useEffect(() => {
    if (!features.hasPlaybackContext || !currentTrack) {
      setPlaybackContext(null)
      return
    }

    // Try to get context from recent contexts
    backend.getRecentContexts(1)
      .then((contexts) => {
        if (contexts.length > 0) {
          const ctx = contexts[0]
          if (ctx.contextId && ctx.contextName) {
            setPlaybackContext({
              contextType: ctx.contextType as ContextType,
              contextId: ctx.contextId,
              contextName: ctx.contextName,
              contextArtworkPath: ctx.contextArtworkPath,
            })
          } else {
            setPlaybackContext(null)
          }
        } else {
          setPlaybackContext(null)
        }
      })
      .catch(() => setPlaybackContext(null))
  }, [currentTrack?.id, features.hasPlaybackContext, backend])

  // Fetch tracks based on context
  useEffect(() => {
    const loadTracks = async () => {
      if (!currentTrack) {
        setTracks([])
        return
      }

      setLoading(true)
      try {
        let fetchedTracks: TrackForGrouping[] = []

        // Try to get tracks based on context
        if (playbackContext?.contextType === 'album' && playbackContext.contextId) {
          const albumTracks = await backend.getAlbumTracks(
            typeof playbackContext.contextId === 'string'
              ? parseInt(playbackContext.contextId)
              : playbackContext.contextId
          )
          fetchedTracks = albumTracks.map((t) => ({
            id: t.id,
            title: t.title,
            artist_name: t.artist_name,
            artist_id: t.artist_id,
            album_title: t.album_title,
            track_number: t.track_number,
            duration_seconds: t.duration_seconds,
            file_path: t.file_path,
            file_format: t.file_format,
            bit_rate: t.bit_rate,
            sample_rate: t.sample_rate,
          }))
        } else if (playbackContext?.contextType === 'artist' && playbackContext.contextId) {
          const artistTracks = await backend.getArtistTracks(
            typeof playbackContext.contextId === 'string'
              ? parseInt(playbackContext.contextId)
              : playbackContext.contextId
          )
          fetchedTracks = artistTracks.map((t) => ({
            id: t.id,
            title: t.title,
            artist_name: t.artist_name,
            artist_id: t.artist_id,
            album_title: t.album_title,
            track_number: t.track_number,
            duration_seconds: t.duration_seconds,
            file_path: t.file_path,
            file_format: t.file_format,
            bit_rate: t.bit_rate,
            sample_rate: t.sample_rate,
          }))
        } else if (playbackContext?.contextType === 'genre' && playbackContext.contextId) {
          const genreTracks = await backend.getGenreTracks(
            typeof playbackContext.contextId === 'string'
              ? parseInt(playbackContext.contextId)
              : playbackContext.contextId
          )
          fetchedTracks = genreTracks.map((t) => ({
            id: t.id,
            title: t.title,
            artist_name: t.artist_name,
            artist_id: t.artist_id,
            album_title: t.album_title,
            track_number: t.track_number,
            duration_seconds: t.duration_seconds,
            file_path: t.file_path,
            file_format: t.file_format,
            bit_rate: t.bit_rate,
            sample_rate: t.sample_rate,
          }))
        } else if (playbackContext?.contextType === 'playlist' && playbackContext.contextId) {
          const playlistTracks = await backend.getPlaylistTracks(String(playbackContext.contextId))
          fetchedTracks = playlistTracks.map((t) => ({
            id: t.id,
            title: t.title,
            artist_name: t.artist_name,
            artist_id: t.artist_id,
            album_title: t.album_title,
            track_number: t.track_number,
            duration_seconds: t.duration_seconds,
            file_path: t.file_path,
            file_format: t.file_format,
            bit_rate: t.bit_rate,
            sample_rate: t.sample_rate,
          }))
        } else if (currentTrack.albumId) {
          // Fallback to album if available
          const albumTracks = await backend.getAlbumTracks(currentTrack.albumId)
          fetchedTracks = albumTracks.map((t) => ({
            id: t.id,
            title: t.title,
            artist_name: t.artist_name,
            artist_id: t.artist_id,
            album_title: t.album_title,
            track_number: t.track_number,
            duration_seconds: t.duration_seconds,
            file_path: t.file_path,
            file_format: t.file_format,
            bit_rate: t.bit_rate,
            sample_rate: t.sample_rate,
          }))
        } else {
          // Fallback: get queue and show those tracks
          const queue = await commands.getQueue()
          if (queue.length > 0) {
            fetchedTracks = queue.map((q, idx) => ({
              id: typeof q.trackId === 'string' ? parseInt(q.trackId) : (q.trackId as number),
              title: q.title,
              artist_name: q.artist,
              album_title: q.album || undefined,
              duration_seconds: q.durationSeconds || undefined,
              track_number: idx + 1,
              file_path: q.filePath,
            }))
          }
        }

        setTracks(fetchedTracks)
      } catch (err) {
        debug.error('Failed to load tracks:', err)
        setTracks([])
      } finally {
        setLoading(false)
      }
    }

    loadTracks()

    const unsubscribe = events.onQueueUpdate(() => {
      loadTracks()
    })
    return unsubscribe
  }, [currentTrack?.id, currentTrack?.albumId, playbackContext, commands, events, backend])

  // Group tracks
  const groupedTracks = useMemo(() => groupTracks(tracks), [tracks])

  // Get active version for a group
  const getActiveVersion = (group: GroupedTrack<TrackForGrouping>): TrackForGrouping => {
    return selectedVersions.get(group.groupKey) || group.bestVersion
  }

  // Handle format selection - plays the selected format
  const handleFormatSelect = async (groupKey: string, track: TrackForGrouping) => {
    setSelectedVersions((prev) => new Map(prev).set(groupKey, track))

    // If this is the currently playing track group, switch to this format
    const currentGroup = groupedTracks.find((g) =>
      g.versions.some((v) => v.id === currentTrack?.id)
    )
    if (currentGroup?.groupKey === groupKey) {
      try {
        await commands.playTrack(track.id)
      } catch (err) {
        debug.error('Failed to switch format:', err)
      }
    }
  }

  // Handle track click — build full queue so playback context is preserved
  const handleTrackClick = async (_group: GroupedTrack<TrackForGrouping>, groupIndex: number) => {
    try {
      const queue: QueueTrack[] = groupedTracks.map((g) => {
        const v = getActiveVersion(g)
        return {
          trackId: String(v.id),
          title: v.title,
          artist: v.artist_name || '',
          album: v.album_title || null,
          filePath: v.file_path ?? '',
          durationSeconds: v.duration_seconds ?? null,
          trackNumber: v.track_number ?? null,
        }
      })
      await commands.playQueue(queue, groupIndex)
    } catch (err) {
      debug.error('Failed to play track:', err)
    }
  }

  const formatTime = (seconds: number | undefined) => {
    if (!seconds || !isFinite(seconds)) return '--:--'
    const mins = Math.floor(seconds / 60)
    const secs = Math.floor(seconds % 60)
    return `${mins}:${secs.toString().padStart(2, '0')}`
  }

  // Empty state
  if (!currentTrack) {
    return (
      <div className="h-full flex flex-col items-center justify-center">
        <div className="w-24 h-24 rounded-full bg-muted flex items-center justify-center mb-6">
          <Music className="w-12 h-12 text-muted-foreground" />
        </div>
        <h2 className="text-xl font-medium text-muted-foreground mb-2">
          {t('nowPlaying.nothingPlaying', 'Nothing playing')}
        </h2>
        <p className="text-sm text-muted-foreground mb-6">
          {t('nowPlaying.selectTrack', 'Select a track from your library to start listening')}
        </p>
        <button
          onClick={() => navigate('/albums')}
          className="px-6 py-3 bg-primary text-primary-foreground rounded-lg hover:opacity-[var(--hover-button-opacity)] transition-opacity transition-colors"
        >
          {t('common.browse', 'Browse Library')}
        </button>
      </div>
    )
  }

  // Context helpers
  const getContextIcon = (contextType: ContextType | undefined) => {
    switch (contextType) {
      case 'album':
        return <Disc3 className="w-4 h-4 text-muted-foreground" />
      case 'playlist':
        return <ListMusic className="w-4 h-4 text-muted-foreground" />
      case 'artist':
        return <Users className="w-4 h-4 text-muted-foreground" />
      case 'genre':
        return <Guitar className="w-4 h-4 text-muted-foreground" />
      default:
        return <Library className="w-4 h-4 text-muted-foreground" />
    }
  }

  const getContextLabel = (contextType: ContextType | undefined): string => {
    switch (contextType) {
      case 'album':
        return t('nowPlaying.playingFromAlbum', 'Playing from album')
      case 'playlist':
        return t('nowPlaying.playingFromPlaylist', 'Playing from playlist')
      case 'artist':
        return t('nowPlaying.playingFromArtist', 'Playing from artist')
      case 'genre':
        return t('nowPlaying.playingFromGenre', 'Playing from genre')
      default:
        return t('nowPlaying.fromLibrary', 'From Library')
    }
  }

  const headerTitle =
    playbackContext?.contextName || currentTrack.album || t('nowPlaying.fromLibrary', 'From Library')
  const headerSubtitle = getContextLabel(playbackContext?.contextType)
  const headerIcon = getContextIcon(playbackContext?.contextType)

  // Handle context navigation
  const handleContextClick = () => {
    if (!playbackContext) return

    const { contextType, contextId } = playbackContext
    if (contextType === 'album' && contextId) {
      navigate(`/albums/${contextId}`)
    } else if (contextType === 'artist' && contextId) {
      navigate(`/artists/${contextId}`)
    } else if (contextType === 'playlist' && contextId) {
      navigate(`/playlists/${contextId}`)
    }
  }

  const isContextClickable = playbackContext && ['album', 'artist', 'playlist'].includes(playbackContext.contextType)

  return (
    <div data-testid="now-playing-page" className="h-full flex items-center justify-center px-4 sm:px-6 lg:px-8">
      <div className="flex gap-12 w-full max-w-[2000px] items-center">
        {/* Left Side - Artwork (2 parts) */}
        <div className="basis-2/5 flex-shrink-0">
          <div data-testid="now-playing-artwork" className="aspect-square w-full rounded-2xl overflow-hidden shadow-2xl bg-muted">
            <ArtworkImage
              trackId={currentTrack.id}
              coverArtPath={currentTrack.coverArtPath}
              alt={currentTrack.album || currentTrack.title}
              className="w-full h-full object-cover"
              fallbackClassName="w-full h-full flex items-center justify-center bg-muted"
            />
          </div>
        </div>

        {/* Right Side - Tracklist (3 parts) */}
        <div className="basis-3/5 flex flex-col min-w-0 max-h-[800px] overflow-hidden">
          <div className="mb-3">
            <div className="flex items-center gap-2 text-xs text-muted-foreground uppercase tracking-wide mb-1">
              {headerIcon}
              <span data-testid="now-playing-artist">{headerSubtitle}</span>
            </div>
            <h2
              data-testid="now-playing-track-title"
              className={`text-lg font-bold ${isContextClickable ? 'hover:text-primary cursor-pointer hover:underline' : ''}`}
              onClick={isContextClickable ? handleContextClick : undefined}
            >
              {headerTitle}
            </h2>
            <p className="text-sm text-muted-foreground">
              {groupedTracks.length} {t('library.tracks', 'tracks')}
            </p>
          </div>

          <div className="flex-1 overflow-y-auto -mx-2">
            {loading ? (
              <div className="flex items-center justify-center h-full">
                <div className="animate-spin w-6 h-6 border-2 border-primary border-t-transparent rounded-full" />
              </div>
            ) : groupedTracks.length === 0 ? (
              <div className="flex flex-col items-center justify-center h-full text-muted-foreground">
                <Music className="w-12 h-12 mb-4 opacity-50" />
                <p>{t('nowPlaying.emptyQueue', 'Queue is empty')}</p>
              </div>
            ) : (
              <div data-testid="now-playing-queue-list" className="space-y-0.5">
                {groupedTracks.map((group, idx) => {
                  const activeVersion = getActiveVersion(group)
                  const isCurrentTrack = group.versions.some((v) => v.id === currentTrack.id)

                  return (
                    <div
                      key={group.groupKey}
                      data-testid={`now-playing-queue-item-${idx}`}
                      onClick={() => handleTrackClick(group, idx)}
                      className={`w-full flex items-center gap-3 px-3 py-2.5 rounded-lg transition-colors cursor-pointer ${
                        isCurrentTrack
                          ? 'bg-primary/10 border border-primary/20'
                          : 'hover:bg-foreground/[var(--hover-bg-opacity)]'
                      }`}
                    >
                      {/* Track Number or Playing Indicator */}
                      <div className="w-6 text-center flex-shrink-0">
                        {isCurrentTrack && isPlaying ? (
                          <div className="flex items-center justify-center gap-0.5">
                            <span className="w-0.5 h-3 bg-primary rounded-full animate-pulse" />
                            <span
                              className="w-0.5 h-4 bg-primary rounded-full animate-pulse"
                              style={{ animationDelay: '0.2s' }}
                            />
                            <span
                              className="w-0.5 h-2 bg-primary rounded-full animate-pulse"
                              style={{ animationDelay: '0.4s' }}
                            />
                          </div>
                        ) : isCurrentTrack ? (
                          <div className="flex items-center justify-center gap-0.5">
                            <span className="w-0.5 h-2 bg-primary/60 rounded-full" />
                            <span className="w-0.5 h-3 bg-primary/60 rounded-full" />
                            <span className="w-0.5 h-2 bg-primary/60 rounded-full" />
                          </div>
                        ) : (
                          <span className="text-sm text-muted-foreground">
                            {activeVersion.track_number || idx + 1}
                          </span>
                        )}
                      </div>

                      {/* Track Info */}
                      <div className="flex-1 min-w-0">
                        <p
                          className={`truncate ${isCurrentTrack ? 'text-primary font-semibold' : 'text-sm'}`}
                        >
                          {activeVersion.title}
                        </p>
                        <div
                          className={`text-xs truncate ${isCurrentTrack ? 'text-primary/70' : 'text-muted-foreground'}`}
                        >
                          <ArtistLink
                            artistId={activeVersion.artist_id}
                            artistName={activeVersion.artist_name || currentTrack.artist}
                            className={`text-xs ${isCurrentTrack ? 'text-primary/70' : 'text-muted-foreground'} hover:underline`}
                          />
                        </div>
                      </div>

                      {/* Format dropdown */}
                      <FormatDropdown
                        versions={group.versions}
                        activeVersion={activeVersion}
                        onSelect={(track) => handleFormatSelect(group.groupKey, track)}
                      />

                      {/* Duration */}
                      <span
                        className={`text-xs flex-shrink-0 w-12 text-right ${isCurrentTrack ? 'text-primary/70' : 'text-muted-foreground'}`}
                      >
                        {formatTime(activeVersion.duration_seconds)}
                      </span>
                    </div>
                  )
                })}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}
