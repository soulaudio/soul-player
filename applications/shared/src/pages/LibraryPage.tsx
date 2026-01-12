/**
 * Shared LibraryPage - works on both desktop and marketing demo
 * Uses BackendContext for data operations
 */

import { useState, useEffect, useCallback, useMemo } from 'react'
import { useSearchParams, useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { Music, Disc3, ListMusic, Users, Search, X, Plus } from 'lucide-react'
import { TrackList, type Track } from '../components/TrackList'
import { TrackMenu } from '../components/TrackMenu'
import { AlbumCard } from '../components/AlbumCard'
import { PlaylistCard } from '../components/PlaylistCard'
import { ArtistCard } from '../components/ArtistCard'
import { AddToPlaylistDialog } from '../components/AddToPlaylistDialog'
import { FeatureGate, usePlatform } from '../contexts/PlatformContext'
import { useBackend, type BackendAlbum, type BackendArtist, type BackendTrack, type BackendPlaylist } from '../contexts/BackendContext'
import { usePlayerCommands, type QueueTrack } from '../contexts/PlayerCommandsContext'
import { removeConsecutiveDuplicates } from '../utils/queue'

type TabId = 'albums' | 'playlists' | 'artists' | 'tracks'

interface Tab {
  id: TabId
  labelKey: string
  icon: React.ReactNode
}

const TABS: Tab[] = [
  { id: 'albums', labelKey: 'library.tab.albums', icon: <Disc3 className="w-4 h-4" /> },
  { id: 'playlists', labelKey: 'library.tab.playlists', icon: <ListMusic className="w-4 h-4" /> },
  { id: 'artists', labelKey: 'library.tab.artists', icon: <Users className="w-4 h-4" /> },
  { id: 'tracks', labelKey: 'library.tab.tracks', icon: <Music className="w-4 h-4" /> },
]

export function LibraryPage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const [searchParams, setSearchParams] = useSearchParams()
  const tabParam = searchParams.get('tab') as TabId | null

  const backend = useBackend()
  const commands = usePlayerCommands()
  const { features } = usePlatform()

  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [healthWarning, setHealthWarning] = useState<string | null>(null)
  const [tracks, setTracks] = useState<BackendTrack[]>([])
  const [albums, setAlbums] = useState<BackendAlbum[]>([])
  const [artists, setArtists] = useState<BackendArtist[]>([])
  const [playlists, setPlaylists] = useState<BackendPlaylist[]>([])
  const [activeTab, setActiveTab] = useState<TabId>(tabParam || 'albums')
  const [searchQuery, setSearchQuery] = useState('')

  // Add to playlist dialog state
  const [selectedTrackForPlaylist, setSelectedTrackForPlaylist] = useState<{
    id: number
    title: string
  } | null>(null)

  // Load library data
  const loadLibrary = useCallback(async () => {
    setIsLoading(true)
    setError(null)
    setHealthWarning(null)
    try {
      const [tracksData, albumsData, artistsData, playlistsData, health] = await Promise.all([
        backend.getAllTracks(),
        backend.getAllAlbums(),
        backend.getAllArtists(),
        backend.getAllPlaylists(),
        backend.checkDatabaseHealth(),
      ])

      setTracks(tracksData)
      setAlbums(albumsData)
      setArtists(artistsData)
      setPlaylists(playlistsData)

      // Check for issues
      if (health.issues.length > 0) {
        setHealthWarning(health.issues.join(' '))
      }
    } catch (err) {
      console.error('Failed to load library:', err)
      setError(err instanceof Error ? err.message : 'Failed to load library')
    } finally {
      setIsLoading(false)
    }
  }, [backend])

  useEffect(() => {
    loadLibrary()
  }, [loadLibrary])

  // Update active tab when URL param changes
  useEffect(() => {
    if (tabParam && TABS.some(t => t.id === tabParam)) {
      setActiveTab(tabParam)
    }
  }, [tabParam])

  // Update URL when tab changes
  const handleTabChange = (tabId: TabId) => {
    setActiveTab(tabId)
    setSearchQuery('') // Reset search when changing tabs
    if (tabId === 'albums') {
      setSearchParams({})
    } else {
      setSearchParams({ tab: tabId })
    }
  }

  // Filter data by search query
  const filteredAlbums = useMemo(() => {
    if (!searchQuery.trim()) return albums
    const query = searchQuery.toLowerCase()
    return albums.filter(
      a =>
        a.title.toLowerCase().includes(query) ||
        (a.artist_name || '').toLowerCase().includes(query)
    )
  }, [albums, searchQuery])

  const filteredArtists = useMemo(() => {
    if (!searchQuery.trim()) return artists
    const query = searchQuery.toLowerCase()
    return artists.filter(a => a.name.toLowerCase().includes(query))
  }, [artists, searchQuery])

  const filteredTracks = useMemo(() => {
    if (!searchQuery.trim()) return tracks
    const query = searchQuery.toLowerCase()
    return tracks.filter(
      t =>
        t.title?.toLowerCase().includes(query) ||
        (t.artist_name || '').toLowerCase().includes(query) ||
        (t.album_title || '').toLowerCase().includes(query)
    )
  }, [tracks, searchQuery])

  const filteredPlaylists = useMemo(() => {
    if (!searchQuery.trim()) return playlists
    const query = searchQuery.toLowerCase()
    return playlists.filter(p => p.name.toLowerCase().includes(query))
  }, [playlists, searchQuery])

  // Build queue from tracks
  const buildQueueFromTracks = useCallback((
    libraryTracks: BackendTrack[],
    clickedTrack: Track,
    clickedIndex: number
  ): QueueTrack[] => {
    const validClickedIndex = libraryTracks.findIndex(t => t.id === clickedTrack.id)
    const actualIndex = validClickedIndex !== -1 ? validClickedIndex : clickedIndex

    const queue = [
      ...libraryTracks.slice(actualIndex),
      ...libraryTracks.slice(0, actualIndex),
    ].map((t): QueueTrack => ({
      trackId: String(t.id),
      title: t.title || 'Unknown',
      artist: t.artist_name || 'Unknown Artist',
      album: t.album_title || null,
      albumId: t.album_id,
      filePath: t.file_path || '',
      durationSeconds: t.duration_seconds || null,
      trackNumber: t.track_number || null,
    }))

    return removeConsecutiveDuplicates(
      queue.filter(t => t.filePath !== ''),
      'trackId'
    )
  }, [])

  // Build queue callback for TrackList
  const buildQueue = useCallback(
    (_allTracks: Track[], clickedTrack: Track, clickedIndex: number): QueueTrack[] => {
      return buildQueueFromTracks(filteredTracks, clickedTrack, clickedIndex)
    },
    [buildQueueFromTracks, filteredTracks]
  )

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
      console.error('[LibraryPage] Failed to add track to play next:', error)
    }
  }, [commands, toQueueTrack])

  const handleAddToQueue = useCallback(async (track: BackendTrack) => {
    try {
      const queueTrack = toQueueTrack(track)
      await commands.addToQueueEnd(queueTrack)
    } catch (error) {
      console.error('[LibraryPage] Failed to add track to queue:', error)
    }
  }, [commands, toQueueTrack])

  const handleCreatePlaylist = async () => {
    try {
      const playlist = await backend.createPlaylist(t('playlist.newPlaylistName', 'New Playlist'))
      navigate(`/playlists/${playlist.id}`)
    } catch (err) {
      console.error('Failed to create playlist:', err)
    }
  }

  // Loading state
  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <div className="animate-spin w-8 h-8 border-4 border-primary border-t-transparent rounded-full mx-auto mb-4"></div>
          <p className="text-muted-foreground">{t('common.loading')}</p>
        </div>
      </div>
    )
  }

  // Error state
  if (error) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center text-destructive">
          <p className="font-medium mb-2">{t('library.loadFailed')}</p>
          <p className="text-sm">{error}</p>
          <button
            onClick={loadLibrary}
            className="mt-4 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90"
          >
            {t('common.retry')}
          </button>
        </div>
      </div>
    )
  }

  return (
    <div className="h-full flex flex-col">
      {/* Health warning banner (Desktop only) */}
      <FeatureGate feature="hasHealthCheck">
        {healthWarning && (
          <div className="mb-4 p-4 bg-yellow-500/10 border border-yellow-500/20 rounded-lg">
            <div className="flex items-start gap-3">
              <div className="flex-shrink-0 w-5 h-5 rounded-full bg-yellow-500/20 flex items-center justify-center mt-0.5">
                <span className="text-yellow-600 dark:text-yellow-400 text-sm">!</span>
              </div>
              <div className="flex-1">
                <p className="text-sm text-yellow-800 dark:text-yellow-200 font-medium">
                  {t('library.databaseIssue')}
                </p>
                <p className="text-sm text-yellow-700 dark:text-yellow-300 mt-1">
                  {healthWarning}
                </p>
              </div>
            </div>
          </div>
        )}
      </FeatureGate>

      {/* Tab Navigation - responsive with horizontal scroll on mobile */}
      <div className="flex items-center gap-2 sm:gap-4 mb-4 sm:mb-6">
        <div className="flex items-center gap-1 bg-muted rounded-lg p-1 overflow-x-auto flex-shrink min-w-0">
          {TABS.map((tab) => (
            <button
              key={tab.id}
              onClick={() => handleTabChange(tab.id)}
              className={`px-2 sm:px-4 py-2 rounded-md transition-colors flex items-center gap-1.5 sm:gap-2 flex-shrink-0 ${
                activeTab === tab.id
                  ? 'bg-background shadow-sm'
                  : 'hover:bg-background/50'
              }`}
              aria-label={t(tab.labelKey)}
            >
              {tab.icon}
              <span className="text-xs sm:text-sm font-medium whitespace-nowrap">{t(tab.labelKey)}</span>
            </button>
          ))}
        </div>
        {/* Create playlist button - shown only on playlists tab */}
        {activeTab === 'playlists' && (
          <FeatureGate feature="canCreatePlaylists">
            <button
              onClick={handleCreatePlaylist}
              className="flex-shrink-0 p-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90"
              aria-label={t('playlist.create')}
            >
              <Plus className="w-4 h-4" />
            </button>
          </FeatureGate>
        )}
      </div>

      {/* Search Bar - responsive full width on mobile */}
      <div className="flex items-center gap-4 mb-4">
        <div className="relative flex-1 sm:max-w-md">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder={
              activeTab === 'albums' ? t('library.search.albumsWithCount', { count: albums.length }) :
              activeTab === 'playlists' ? t('library.search.playlistsWithCount', { count: playlists.length }) :
              activeTab === 'artists' ? t('library.search.artistsWithCount', { count: artists.length }) :
              t('library.search.tracksWithCount', { count: tracks.length })
            }
            className="w-full pl-10 pr-4 py-2 rounded-lg bg-muted border border-transparent focus:border-primary focus:outline-none text-sm"
          />
          {searchQuery && (
            <button
              onClick={() => setSearchQuery('')}
              className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
            >
              <X className="w-4 h-4" />
            </button>
          )}
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto">
        {/* Albums Tab */}
        {activeTab === 'albums' && (
          filteredAlbums.length > 0 ? (
            <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3 sm:gap-4">
              {filteredAlbums.map((album) => (
                <AlbumCard
                  key={album.id}
                  album={{
                    id: album.id,
                    title: album.title,
                    artist_name: album.artist_name,
                    year: album.year,
                    cover_art_path: album.cover_art_path,
                  }}
                  showArtist={true}
                  className="w-full"
                />
              ))}
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
              <Disc3 className="w-12 h-12 mb-4 opacity-50" />
              <p className="font-medium">
                {searchQuery ? t('library.noSearchResults') : t('library.noAlbums')}
              </p>
              <p className="text-sm mt-1">
                {searchQuery ? t('library.tryDifferentSearch') : t('library.noAlbumsHint')}
              </p>
            </div>
          )
        )}

        {/* Playlists Tab */}
        {activeTab === 'playlists' && (
          filteredPlaylists.length > 0 ? (
            <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3 sm:gap-4">
              {filteredPlaylists.map((playlist) => (
                <PlaylistCard
                  key={playlist.id}
                  playlist={playlist}
                  className="w-full"
                />
              ))}
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
              <ListMusic className="w-12 h-12 mb-4 opacity-50" />
              <p className="font-medium">
                {searchQuery ? t('library.noSearchResults') : t('playlist.noPlaylists')}
              </p>
              <p className="text-sm mt-1">
                {searchQuery ? t('library.tryDifferentSearch') : t('playlist.createHint')}
              </p>
              <FeatureGate feature="canCreatePlaylists">
                <button
                  onClick={handleCreatePlaylist}
                  className="mt-4 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90"
                >
                  {t('playlist.create')}
                </button>
              </FeatureGate>
            </div>
          )
        )}

        {/* Artists Tab */}
        {activeTab === 'artists' && (
          filteredArtists.length > 0 ? (
            <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3 sm:gap-4">
              {filteredArtists.map((artist) => (
                <ArtistCard
                  key={artist.id}
                  artist={artist}
                />
              ))}
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
              <Users className="w-12 h-12 mb-4 opacity-50" />
              <p className="font-medium">
                {searchQuery ? t('library.noSearchResults') : t('artist.noArtists')}
              </p>
              <p className="text-sm mt-1">
                {searchQuery ? t('library.tryDifferentSearch') : t('artist.noArtistsHint')}
              </p>
            </div>
          )
        )}

        {/* Tracks Tab */}
        {activeTab === 'tracks' && (
          filteredTracks.length > 0 ? (
            <TrackList
              tracks={filteredTracks.map(t => ({
                id: t.id,
                title: String(t.title || 'Unknown'),
                artist: t.artist_name,
                album: t.album_title,
                duration: t.duration_seconds,
                trackNumber: t.track_number,
                isAvailable: !!t.file_path,
                format: t.file_format,
                bitrate: t.bit_rate,
                sampleRate: t.sample_rate,
                channels: t.channels,
              }))}
              buildQueue={buildQueue}
              renderMenu={(track) => {
                const backendTrack = filteredTracks.find(t => t.id === track.id)
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
                    onDelete={async () => {
                      await backend.deleteTrack(backendTrack.id)
                      loadLibrary()
                    }}
                  />
                )
              }}
            />
          ) : (
            <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
              <Music className="w-12 h-12 mb-4 opacity-50" />
              <p className="font-medium">
                {searchQuery ? t('library.noSearchResults') : t('library.noTracks')}
              </p>
              <p className="text-sm mt-1">
                {searchQuery ? t('library.tryDifferentSearch') : t('library.addTracks')}
              </p>
            </div>
          )
        )}

      </div>

      {/* Add to Playlist Dialog (Desktop only) */}
      {features.canCreatePlaylists && selectedTrackForPlaylist && (
        <AddToPlaylistDialog
          open={!!selectedTrackForPlaylist}
          onClose={() => setSelectedTrackForPlaylist(null)}
          trackId={selectedTrackForPlaylist.id}
          trackTitle={selectedTrackForPlaylist.title}
        />
      )}
    </div>
  )
}
