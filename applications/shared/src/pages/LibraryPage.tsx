/**
 * Shared LibraryPage - works on both desktop and marketing demo
 * Uses BackendContext for data operations
 */

import { useState, useEffect, useCallback, useMemo, useDeferredValue } from 'react'
import { useSearchParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useQueryClient } from '@tanstack/react-query'
import { useNavigateWithHistory } from '../hooks/useNavigateWithHistory'
import { Music, Disc3, ListMusic, Users, Search, X, Plus } from 'lucide-react'
import { TrackList, type Track } from '../components/TrackList'
import { TrackMenu } from '../components/TrackMenu'
import { AlbumCard } from '../components/AlbumCard'
import { PlaylistCard } from '../components/PlaylistCard'
import { ArtistCard } from '../components/ArtistCard'
import { AddToPlaylistDialog } from '../components/AddToPlaylistDialog'
import { FeatureGate, usePlatform } from '../contexts/PlatformContext'
import { type BackendTrack } from '../contexts/BackendContext'
import { usePlayerCommands, type QueueTrack } from '../contexts/PlayerCommandsContext'
import { removeConsecutiveDuplicates } from '../utils/queue'
import { useCreatePlaylist } from '../hooks/queries/usePlaylistMutations'
import { useDeleteTrack } from '../hooks/queries/useTrackMutations'
import { useLibraryData } from '../hooks/queries/useLibraryQueries'
import { debug } from '../utils/debug';

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
  const { navigate } = useNavigateWithHistory()
  const [searchParams, setSearchParams] = useSearchParams()
  const tabParam = searchParams.get('tab') as TabId | null

  const queryClient = useQueryClient()
  const commands = usePlayerCommands()
  const { features } = usePlatform()
  const createPlaylistMutation = useCreatePlaylist()
  const deleteTrackMutation = useDeleteTrack()

  // Load library data with React Query - progressive loading
  const {
    tracks,
    albums,
    artists,
    playlists,
    health,
    isTracksLoading,
    isAlbumsLoading,
    isArtistsLoading,
    isPlaylistsLoading,
    error,
  } = useLibraryData()

  const [activeTab, setActiveTab] = useState<TabId>(tabParam || 'albums')
  const [searchQuery, setSearchQuery] = useState('')
  // Debounce search query to prevent filtering on every keystroke
  const deferredSearchQuery = useDeferredValue(searchQuery)

  // Add to playlist dialog state
  const [selectedTrackForPlaylist, setSelectedTrackForPlaylist] = useState<{
    id: number
    title: string
  } | null>(null)

  // Derive health warning from health data
  const healthWarning = useMemo(() => {
    if (!health || health.issues.length === 0) return null
    return health.issues.join(' ')
  }, [health])

  // Refresh library data (invalidate queries)
  const refreshLibrary = useCallback(() => {
    queryClient.invalidateQueries({ queryKey: ['tracks'] })
    queryClient.invalidateQueries({ queryKey: ['albums'] })
    queryClient.invalidateQueries({ queryKey: ['artists'] })
    queryClient.invalidateQueries({ queryKey: ['playlists'] })
    queryClient.invalidateQueries({ queryKey: ['library'] })
  }, [queryClient])

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

  // Filter data by search query (using deferred value for debouncing)
  const filteredAlbums = useMemo(() => {
    if (!deferredSearchQuery.trim()) return albums
    const query = deferredSearchQuery.toLowerCase()
    return albums.filter(
      a =>
        a.title.toLowerCase().includes(query) ||
        (a.artist_name || '').toLowerCase().includes(query)
    )
  }, [albums, deferredSearchQuery])

  const filteredArtists = useMemo(() => {
    if (!deferredSearchQuery.trim()) return artists
    const query = deferredSearchQuery.toLowerCase()
    return artists.filter(a => a.name.toLowerCase().includes(query))
  }, [artists, deferredSearchQuery])

  const filteredTracks = useMemo(() => {
    if (!deferredSearchQuery.trim()) return tracks
    const query = deferredSearchQuery.toLowerCase()
    return tracks.filter(
      t =>
        t.title?.toLowerCase().includes(query) ||
        (t.artist_name || '').toLowerCase().includes(query) ||
        (t.album_title || '').toLowerCase().includes(query)
    )
  }, [tracks, deferredSearchQuery])

  const filteredPlaylists = useMemo(() => {
    if (!deferredSearchQuery.trim()) return playlists
    const query = deferredSearchQuery.toLowerCase()
    return playlists.filter(p => p.name.toLowerCase().includes(query))
  }, [playlists, deferredSearchQuery])

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
      debug.error('[LibraryPage] Failed to add track to play next:', error)
    }
  }, [commands, toQueueTrack])

  const handleAddToQueue = useCallback(async (track: BackendTrack) => {
    try {
      const queueTrack = toQueueTrack(track)
      await commands.addToQueueEnd(queueTrack)
    } catch (error) {
      debug.error('[LibraryPage] Failed to add track to queue:', error)
    }
  }, [commands, toQueueTrack])

  const handleCreatePlaylist = () => {
    createPlaylistMutation.mutate(
      { name: t('playlist.newPlaylistName', 'New Playlist') },
      {
        onSuccess: (playlist) => {
          navigate(`/playlists/${playlist.id}`)
        },
        onError: (err) => {
          debug.error('Failed to create playlist:', err)
        },
      }
    )
  }

  // Error state - only show if ALL queries have failed
  if (error && !tracks.length && !albums.length && !artists.length && !playlists.length) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center text-destructive">
          <p className="font-medium mb-2">{t('library.loadFailed')}</p>
          <p className="text-sm">{error instanceof Error ? error.message : String(error)}</p>
          <button
            onClick={refreshLibrary}
            className="mt-4 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:opacity-[var(--hover-button-opacity)] transition-opacity"
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
                  : 'hover:bg-foreground/[var(--hover-bg-opacity)]'
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
              className="flex-shrink-0 p-2 bg-primary text-primary-foreground rounded-lg hover:opacity-[var(--hover-button-opacity)] transition-opacity"
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
              className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:opacity-[var(--hover-text-opacity)] transition-opacity duration-[var(--transition-duration)]"
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
          isAlbumsLoading ? (
            <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3 sm:gap-4">
              {Array.from({ length: 12 }).map((_, i) => (
                <div key={i} className="animate-pulse">
                  <div className="aspect-square bg-muted rounded-lg mb-2"></div>
                  <div className="h-4 bg-muted rounded w-3/4 mb-1"></div>
                  <div className="h-3 bg-muted rounded w-1/2"></div>
                </div>
              ))}
            </div>
          ) : filteredAlbums.length > 0 ? (
            <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3 sm:gap-4">
              {filteredAlbums.map((album, index) => (
                <AlbumCard
                  key={album.id}
                  album={{
                    id: album.id,
                    title: album.title,
                    artist_name: album.artist_name,
                    artist_id: album.artist_id,
                    year: album.year,
                    cover_art_path: album.cover_art_path,
                  }}
                  showArtist={true}
                  className="w-full"
                  priority={index < 20}
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
          isPlaylistsLoading ? (
            <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3 sm:gap-4">
              {Array.from({ length: 12 }).map((_, i) => (
                <div key={i} className="animate-pulse">
                  <div className="aspect-square bg-muted rounded-lg mb-2"></div>
                  <div className="h-4 bg-muted rounded w-3/4 mb-1"></div>
                  <div className="h-3 bg-muted rounded w-1/2"></div>
                </div>
              ))}
            </div>
          ) : filteredPlaylists.length > 0 ? (
            <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3 sm:gap-4">
              {filteredPlaylists.map((playlist, index) => (
                <PlaylistCard
                  key={playlist.id}
                  playlist={playlist}
                  className="w-full"
                  priority={index < 20}
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
                  className="mt-4 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:opacity-[var(--hover-button-opacity)] transition-opacity"
                >
                  {t('playlist.create')}
                </button>
              </FeatureGate>
            </div>
          )
        )}

        {/* Artists Tab */}
        {activeTab === 'artists' && (
          isArtistsLoading ? (
            <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3 sm:gap-4">
              {Array.from({ length: 12 }).map((_, i) => (
                <div key={i} className="animate-pulse">
                  <div className="aspect-square bg-muted rounded-full mb-2"></div>
                  <div className="h-4 bg-muted rounded w-3/4 mb-1"></div>
                  <div className="h-3 bg-muted rounded w-1/2"></div>
                </div>
              ))}
            </div>
          ) : filteredArtists.length > 0 ? (
            <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3 sm:gap-4">
              {filteredArtists.map((artist, index) => (
                <ArtistCard
                  key={artist.id}
                  artist={artist}
                  priority={index < 20}
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
          isTracksLoading ? (
            <div className="space-y-2">
              {Array.from({ length: 15 }).map((_, i) => (
                <div key={i} className="flex items-center gap-4 p-3 animate-pulse">
                  <div className="w-10 h-10 bg-muted rounded"></div>
                  <div className="flex-1">
                    <div className="h-4 bg-muted rounded w-1/3 mb-2"></div>
                    <div className="h-3 bg-muted rounded w-1/4"></div>
                  </div>
                  <div className="h-3 bg-muted rounded w-16"></div>
                </div>
              ))}
            </div>
          ) : filteredTracks.length > 0 ? (
            <TrackList
              tracks={filteredTracks.map(t => ({
                id: t.id,
                title: String(t.title || 'Unknown'),
                artist: t.artist_name,
                artistId: t.artist_id,
                album: t.album_title,
                albumId: t.album_id,
                duration: t.duration_seconds,
                trackNumber: t.track_number,
                isAvailable: !!t.file_path,
                format: t.file_format,
                bitrate: t.bit_rate,
                sampleRate: t.sample_rate,
                channels: t.channels,
              }))}
              buildQueue={buildQueue}
              virtualized={filteredTracks.length > 100}
              virtualItemSize={56}
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
                    onDelete={() => {
                      deleteTrackMutation.mutate(backendTrack.id, {
                        onSuccess: () => refreshLibrary()
                      })
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
