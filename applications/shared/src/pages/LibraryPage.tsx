/**
 * Shared LibraryPage - works on both desktop and marketing demo
 * Uses BackendContext for data operations
 */

import { useState, useEffect, useCallback, useMemo } from 'react'
import { useSearchParams, useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { Music, Disc3, ListMusic, Users, Play, Search, X, Plus } from 'lucide-react'
import { TrackList, type Track } from '../components/TrackList'
import { AlbumCard } from '../components/AlbumCard'
import { PlaylistCard } from '../components/PlaylistCard'
import { FeatureGate } from '../contexts/PlatformContext'
import { useBackend, type BackendAlbum, type BackendArtist, type BackendTrack, type BackendPlaylist } from '../contexts/BackendContext'
import { type QueueTrack } from '../contexts/PlayerCommandsContext'
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

// Artist Card Component
function ArtistCard({
  artist,
  onClick,
}: {
  artist: BackendArtist
  onClick: () => void
}) {
  const { t } = useTranslation()

  return (
    <div className="group cursor-pointer" onClick={onClick}>
      <div className="relative aspect-square mb-3 bg-muted rounded-full overflow-hidden shadow-md hover:shadow-xl transition-shadow flex items-center justify-center group-hover:bg-primary/10">
        <Users className="w-12 h-12 text-muted-foreground group-hover:text-primary transition-colors" />
        <div className="absolute inset-0 bg-black/40 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">
          <div className="w-10 h-10 bg-primary rounded-full flex items-center justify-center">
            <Play className="w-5 h-5 text-primary-foreground ml-0.5" fill="currentColor" />
          </div>
        </div>
      </div>
      <div className="text-center">
        <h3 className="font-medium truncate" title={artist.name}>
          {artist.name}
        </h3>
        <p className="text-sm text-muted-foreground">
          {artist.album_count} {t('library.albums')} • {artist.track_count} {t('library.tracks')}
        </p>
      </div>
    </div>
  )
}

export function LibraryPage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const [searchParams, setSearchParams] = useSearchParams()
  const tabParam = searchParams.get('tab') as TabId | null

  const backend = useBackend()

  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [healthWarning, setHealthWarning] = useState<string | null>(null)
  const [tracks, setTracks] = useState<BackendTrack[]>([])
  const [albums, setAlbums] = useState<BackendAlbum[]>([])
  const [artists, setArtists] = useState<BackendArtist[]>([])
  const [playlists, setPlaylists] = useState<BackendPlaylist[]>([])
  const [activeTab, setActiveTab] = useState<TabId>(tabParam || 'albums')
  const [searchQuery, setSearchQuery] = useState('')

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

  // Navigation handlers
  const handleArtistClick = (artist: BackendArtist) => {
    navigate(`/artists/${artist.id}`)
  }

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

      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between mb-4 sm:mb-6 gap-2">
        <div>
          <h1 className="text-2xl sm:text-3xl font-bold">{t('nav.library')}</h1>
          <p className="text-muted-foreground text-sm sm:text-base mt-1">
            <span className="hidden sm:inline">
              {albums.length} {t('library.albums')} • {artists.length} {t('library.artists')} • {tracks.length} {t('library.tracks')}
            </span>
            <span className="sm:hidden">
              {albums.length} {t('library.albums')} • {tracks.length} {t('library.tracks')}
            </span>
          </p>
        </div>
      </div>

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
            placeholder={t(`library.search.${activeTab}`, `Search ${activeTab}...`)}
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
                  onClick={() => handleArtistClick(artist)}
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
    </div>
  )
}
