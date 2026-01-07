'use client'

import { useState, useEffect, useCallback } from 'react'
import { TrackList, usePlayerCommands, type Track, type QueueTrack } from '@soul-player/shared'
import { getDemoStorage } from '@/lib/demo/storage'
import { DemoTrack, DemoAlbum } from '@/lib/demo/types'

type ViewMode = 'tracks' | 'albums'

export function LibraryPage() {
  const [viewMode, setViewMode] = useState<ViewMode>('tracks')
  const [demoTracks, setDemoTracks] = useState<DemoTrack[]>([])
  const [albums, setAlbums] = useState<DemoAlbum[]>([])

  const commands = usePlayerCommands()
  const storage = getDemoStorage()

  // Load data from storage
  useEffect(() => {
    if (storage.isLoaded()) {
      setDemoTracks(storage.getAllTracks())
      setAlbums(storage.getAllAlbums())
    }
  }, [storage])

  // Convert DemoTrack to Track format for shared component
  const tracks: Track[] = demoTracks.map((t) => ({
    id: t.id,
    title: t.title,
    artist: t.artist,
    album: t.album || undefined,
    duration: t.duration,
    trackNumber: undefined,
  }))

  // Build queue callback - platform-specific logic
  const buildQueue = useCallback((allTracks: Track[], clickedTrack: Track, clickedIndex: number): QueueTrack[] => {
    // Find corresponding demo tracks for file paths
    const queue = [
      ...allTracks.slice(clickedIndex),
      ...allTracks.slice(0, clickedIndex),
    ].map((t) => {
      const demoTrack = demoTracks.find((dt) => dt.id === String(t.id))
      return {
        trackId: String(t.id),
        title: t.title,
        artist: t.artist || 'Unknown Artist',
        album: t.album || null,
        filePath: demoTrack?.path || '',
        durationSeconds: t.duration || null,
        trackNumber: t.trackNumber || null,
      }
    })

    return queue
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
        const demoTrack = demoTracks.find((dt) => dt.id === String(t.id))
        return {
          trackId: String(t.id),
          title: t.title,
          artist: t.artist || 'Unknown Artist',
          album: t.album || null,
          filePath: demoTrack?.path || '',
          durationSeconds: t.duration || null,
          trackNumber: null,
        }
      })

      console.log('[LibraryPage] Playing album with', queue.length, 'tracks')

      // Play the album from the first track
      await commands.playQueue(queue, 0)
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
            {tracks.length} tracks • {albums.length} albums
          </p>
        </div>

        {/* View mode toggle */}
        <div className="flex items-center gap-2 bg-muted rounded-lg p-1">
          <button
            onClick={() => setViewMode('tracks')}
            className={`px-4 py-2 rounded-md transition-colors flex items-center gap-2 ${
              viewMode === 'tracks' ? 'bg-background shadow-sm' : 'hover:bg-background/50'
            }`}
            aria-label="View tracks"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zM9 10l12-3" />
            </svg>
            <span className="text-sm font-medium">Tracks</span>
          </button>
          <button
            onClick={() => setViewMode('albums')}
            className={`px-4 py-2 rounded-md transition-colors flex items-center gap-2 ${
              viewMode === 'albums' ? 'bg-background shadow-sm' : 'hover:bg-background/50'
            }`}
            aria-label="View albums"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zM9 10l12-3" />
            </svg>
            <span className="text-sm font-medium">Albums</span>
          </button>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto">
        {viewMode === 'tracks' ? (
          <TrackList tracks={tracks} buildQueue={buildQueue} />
        ) : (
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
                    <svg
                      className="w-12 h-12 text-muted-foreground"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zM9 10l12-3"
                      />
                    </svg>
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
      </div>
    </div>
  )
}
