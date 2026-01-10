'use client'

import { useState, useEffect, useCallback } from 'react'
import { useSearchParams } from 'react-router-dom'
import { TrackList, usePlayerCommands, removeConsecutiveDuplicates, type Track, type QueueTrack } from '@soul-player/shared'
import { getDemoStorage } from '@/lib/demo/storage'
import { DemoTrack, DemoAlbum } from '@/lib/demo/types'
import { Music, Disc3, ListMusic, Users, Guitar } from 'lucide-react'

type TabId = 'tracks' | 'albums' | 'playlists' | 'artists' | 'genres'

interface Tab {
  id: TabId
  label: string
  icon: React.ReactNode
}

const TABS: Tab[] = [
  { id: 'tracks', label: 'Tracks', icon: <Music className="w-4 h-4" /> },
  { id: 'albums', label: 'Albums', icon: <Disc3 className="w-4 h-4" /> },
  { id: 'playlists', label: 'Playlists', icon: <ListMusic className="w-4 h-4" /> },
  { id: 'artists', label: 'Artists', icon: <Users className="w-4 h-4" /> },
  { id: 'genres', label: 'Genres', icon: <Guitar className="w-4 h-4" /> },
]

export function LibraryPage() {
  const [searchParams, setSearchParams] = useSearchParams()
  const tabParam = searchParams.get('tab') as TabId | null
  const [activeTab, setActiveTab] = useState<TabId>(tabParam || 'tracks')

  const [demoTracks, setDemoTracks] = useState<DemoTrack[]>([])
  const [albums, setAlbums] = useState<DemoAlbum[]>([])

  const commands = usePlayerCommands()
  const storage = getDemoStorage()

  // Update active tab when URL param changes
  useEffect(() => {
    if (tabParam && TABS.some(t => t.id === tabParam)) {
      setActiveTab(tabParam)
    }
  }, [tabParam])

  // Update URL when tab changes
  const handleTabChange = (tabId: TabId) => {
    setActiveTab(tabId)
    if (tabId === 'tracks') {
      setSearchParams({})
    } else {
      setSearchParams({ tab: tabId })
    }
  }

  // Load data from storage
  useEffect(() => {
    if (storage.isLoaded()) {
      setDemoTracks(storage.getAllTracks())
      setAlbums(storage.getAllAlbums())
    }
  }, [storage])

  // Convert DemoTrack to Track format for shared component
  // NOTE: Demo tracks have string IDs, but shared Track interface requires number IDs
  const tracks: Track[] = demoTracks.map((t) => ({
    id: Number(t.id), // Convert string ID to number for shared interface
    title: t.title,
    artist: t.artist,
    album: t.album || undefined,
    duration: t.duration,
    trackNumber: undefined,
  }))

  // Get unique artists from tracks
  const artists = [...new Set(demoTracks.map(t => t.artist).filter(Boolean))] as string[]

  // Build queue callback - platform-specific logic
  const buildQueue = useCallback((allTracks: Track[], clickedTrack: Track, clickedIndex: number): QueueTrack[] => {
    console.log('[LibraryPage] buildQueue called:', {
      totalTracks: allTracks.length,
      clickedTrack: clickedTrack.title,
      clickedIndex
    })

    // Find corresponding demo tracks for file paths
    const queue = [
      ...allTracks.slice(clickedIndex),
      ...allTracks.slice(0, clickedIndex),
    ].map((t) => {
      // Convert Track.id (number) to string for lookup in demo storage
      const demoTrack = demoTracks.find((dt) => dt.id === String(t.id))

      if (!demoTrack) {
        console.error('[LibraryPage] Demo track not found for ID:', t.id, 'title:', t.title)
      }

      return {
        trackId: String(t.id),
        title: t.title,
        artist: t.artist || 'Unknown Artist',
        album: t.album || null,
        filePath: demoTrack?.path || '',
        durationSeconds: t.duration || null,
        trackNumber: t.trackNumber || null,
        coverArtPath: demoTrack?.coverUrl || undefined,
      }
    })

    // Filter out tracks with empty filePath (failed lookups)
    const validQueue = queue.filter(t => t.filePath !== '')

    if (validQueue.length !== queue.length) {
      console.warn('[LibraryPage] Filtered out', queue.length - validQueue.length, 'tracks with missing paths')
    }

    // Remove consecutive duplicates (prevents same track playing twice in a row)
    return removeConsecutiveDuplicates(validQueue, 'trackId')
  }, [demoTracks])

  const handleAlbumClick = async (album: DemoAlbum) => {
    console.log('[LibraryPage] Playing album:', album)
    try {
      // Get all tracks for this album
      const albumTracks = tracks.filter(t => t.album === album.title)
      if (albumTracks.length === 0) {
        console.error('[LibraryPage] No tracks found for album')
        return
      }

      // Build queue from album tracks
      const queue: QueueTrack[] = albumTracks.map((t) => {
        // Convert Track.id (number) to string for lookup in demo storage
        const demoTrack = demoTracks.find((dt) => dt.id === String(t.id))

        if (!demoTrack) {
          console.error('[LibraryPage] Demo track not found for album track ID:', t.id)
        }

        return {
          trackId: String(t.id),
          title: t.title,
          artist: t.artist || 'Unknown Artist',
          album: t.album || null,
          filePath: demoTrack?.path || '',
          durationSeconds: t.duration || null,
          trackNumber: null,
          coverArtPath: demoTrack?.coverUrl || undefined,
        }
      })

      // Remove consecutive duplicates (prevents same track playing twice in a row)
      const deduplicatedQueue = removeConsecutiveDuplicates(queue, 'trackId')

      console.log('[LibraryPage] Playing album with', deduplicatedQueue.length, 'tracks')

      // Play the album from the first track
      await commands.playQueue(deduplicatedQueue, 0)
    } catch (error) {
      console.error('[LibraryPage] Failed to play album:', error)
    }
  }

  return (
    <div className="flex flex-col" style={{ height: '100%' }}>
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-3xl font-bold">Library</h1>
          <p className="text-muted-foreground mt-1">
            {tracks.length} tracks • {albums.length} albums • {artists.length} artists
          </p>
        </div>
      </div>

      {/* Tab Navigation */}
      <div className="flex items-center gap-1 bg-muted rounded-lg p-1 mb-6 w-fit">
        {TABS.map((tab) => (
          <button
            key={tab.id}
            onClick={() => handleTabChange(tab.id)}
            className={`px-4 py-2 rounded-md transition-colors flex items-center gap-2 ${
              activeTab === tab.id
                ? 'bg-background shadow-sm'
                : 'hover:bg-background/50'
            }`}
            aria-label={`View ${tab.label}`}
          >
            {tab.icon}
            <span className="text-sm font-medium">{tab.label}</span>
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto">
        {activeTab === 'tracks' && (
          <TrackList tracks={tracks} buildQueue={buildQueue} />
        )}

        {activeTab === 'albums' && (
          <div className="grid grid-cols-4 gap-4">
            {albums.map((album) => (
              <div
                key={album.id}
                className="group cursor-pointer border rounded-lg p-4 hover:bg-muted/50 transition-colors"
                onClick={() => handleAlbumClick(album)}
              >
                <div className="aspect-square bg-muted rounded-lg mb-3 flex items-center justify-center overflow-hidden">
                  {album.coverUrl ? (
                    <img
                      src={album.coverUrl}
                      alt={album.title}
                      className="w-full h-full object-cover"
                    />
                  ) : (
                    <Disc3 className="w-12 h-12 text-muted-foreground" />
                  )}
                </div>
                <h3 className="font-medium truncate">{album.title}</h3>
                <p className="text-sm text-muted-foreground truncate">{album.artist}</p>
                <p className="text-xs text-muted-foreground mt-1">
                  {album.year} • {album.trackIds.length} tracks
                </p>
              </div>
            ))}
          </div>
        )}

        {activeTab === 'playlists' && (
          <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
            <ListMusic className="w-12 h-12 mb-4 opacity-50" />
            <p className="font-medium">No playlists yet</p>
            <p className="text-sm mt-1">Create a playlist to organize your music</p>
            <button className="mt-4 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90">
              Create Playlist
            </button>
          </div>
        )}

        {activeTab === 'artists' && (
          artists.length > 0 ? (
            <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4">
              {artists.map((artist) => (
                <div
                  key={artist}
                  className="group cursor-pointer"
                >
                  <div className="relative aspect-square mb-3 bg-muted rounded-full overflow-hidden shadow-md hover:shadow-xl transition-shadow flex items-center justify-center">
                    <Users className="w-12 h-12 text-muted-foreground" />
                  </div>
                  <div className="text-center">
                    <h3 className="font-medium truncate" title={artist}>
                      {artist}
                    </h3>
                    <p className="text-sm text-muted-foreground">
                      {demoTracks.filter(t => t.artist === artist).length} tracks
                    </p>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
              <Users className="w-12 h-12 mb-4 opacity-50" />
              <p className="font-medium">No artists found</p>
              <p className="text-sm mt-1">Import music to see your artists</p>
            </div>
          )
        )}

        {activeTab === 'genres' && (
          <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
            <Guitar className="w-12 h-12 mb-4 opacity-50" />
            <p className="font-medium">No genres found</p>
            <p className="text-sm mt-1">Genre information is extracted from your music files</p>
          </div>
        )}
      </div>
    </div>
  )
}
