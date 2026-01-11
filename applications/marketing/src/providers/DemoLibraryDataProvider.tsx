'use client'

/**
 * Demo LibraryData provider - wraps DemoStorage for marketing demo
 * Implements LibraryDataInterface from shared package
 */

import { useState, useEffect, useCallback, ReactNode, useMemo } from 'react'
import {
  LibraryDataProvider,
  type LibraryDataInterface,
  type Album,
  type Artist,
  type Playlist,
  type Genre,
  type LibraryTrack,
  type QueueTrack,
  type Track,
  removeConsecutiveDuplicates,
} from '@soul-player/shared'
import { getDemoStorage, initializeDemoStorage } from '@/lib/demo/storage'
import type { DemoTrack, DemoAlbum } from '@/lib/demo/types'

interface DemoLibraryDataProviderProps {
  children: ReactNode
  jsonUrl?: string
}

// Convert DemoTrack to LibraryTrack
function toLibraryTrack(dt: DemoTrack): LibraryTrack {
  return {
    id: Number(dt.id),
    title: dt.title,
    artist: dt.artist,
    artist_name: dt.artist,
    album: dt.album,
    album_title: dt.album,
    duration: dt.duration,
    duration_seconds: dt.duration,
    trackNumber: dt.trackNumber,
    file_path: dt.path,
    path: dt.path,
    coverUrl: dt.coverUrl,
    isAvailable: true,
  }
}

// Convert DemoAlbum to Album
function toAlbum(da: DemoAlbum): Album {
  return {
    id: da.id,
    title: da.title,
    artist: da.artist,
    artist_name: da.artist,
    year: da.year,
    trackCount: da.trackIds.length,
    coverUrl: da.coverUrl,
    trackIds: da.trackIds,
  }
}

// Extract artists from tracks
function extractArtists(tracks: DemoTrack[]): Artist[] {
  const artistMap = new Map<string, { trackCount: number; albumTitles: Set<string> }>()

  tracks.forEach(track => {
    const existing = artistMap.get(track.artist)
    if (existing) {
      existing.trackCount++
      if (track.album) existing.albumTitles.add(track.album)
    } else {
      artistMap.set(track.artist, {
        trackCount: 1,
        albumTitles: track.album ? new Set([track.album]) : new Set(),
      })
    }
  })

  return Array.from(artistMap.entries()).map(([name, data], index) => ({
    id: index + 1,
    name,
    track_count: data.trackCount,
    album_count: data.albumTitles.size,
  }))
}

// Extract genres from tracks (demo has none, but interface requires it)
function extractGenres(_tracks: DemoTrack[]): Genre[] {
  // Demo data doesn't have genres
  return []
}

export function DemoLibraryDataProvider({ children, jsonUrl = '/demo-data.json' }: DemoLibraryDataProviderProps) {
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [tracks, setTracks] = useState<LibraryTrack[]>([])
  const [albums, setAlbums] = useState<Album[]>([])
  const [artists, setArtists] = useState<Artist[]>([])
  const [rawTracks, setRawTracks] = useState<DemoTrack[]>([])
  const [rawAlbums, setRawAlbums] = useState<DemoAlbum[]>([])

  // Load data on mount
  useEffect(() => {
    loadLibrary()
  }, [jsonUrl])

  const loadLibrary = useCallback(async () => {
    setIsLoading(true)
    setError(null)
    try {
      const storage = await initializeDemoStorage(jsonUrl)
      const demoTracks = storage.getAllTracks()
      const demoAlbums = storage.getAllAlbums()

      setRawTracks(demoTracks)
      setRawAlbums(demoAlbums)
      setTracks(demoTracks.map(toLibraryTrack))
      setAlbums(demoAlbums.map(toAlbum))
      setArtists(extractArtists(demoTracks))
    } catch (err) {
      console.error('Failed to load demo library:', err)
      setError(err instanceof Error ? err.message : 'Failed to load library')
    } finally {
      setIsLoading(false)
    }
  }, [jsonUrl])

  // Lookup helpers
  const getTrackById = useCallback((id: string | number): LibraryTrack | undefined => {
    const stringId = String(id)
    const raw = rawTracks.find(t => t.id === stringId)
    return raw ? toLibraryTrack(raw) : undefined
  }, [rawTracks])

  const getAlbumById = useCallback((id: string | number): Album | undefined => {
    const stringId = String(id)
    const raw = rawAlbums.find(a => a.id === stringId)
    return raw ? toAlbum(raw) : undefined
  }, [rawAlbums])

  const getArtistById = useCallback((id: string | number): Artist | undefined => {
    const numId = typeof id === 'string' ? parseInt(id, 10) : id
    return artists.find(a => a.id === numId)
  }, [artists])

  const getPlaylistById = useCallback((_id: string): Playlist | undefined => {
    // Demo doesn't have playlists
    return undefined
  }, [])

  // Data fetching
  const getAlbumTracks = useCallback(async (albumId: string | number): Promise<LibraryTrack[]> => {
    const storage = getDemoStorage()
    const albumTracks = storage.getAlbumTracks(String(albumId))
    return albumTracks.map(toLibraryTrack)
  }, [])

  const getArtistTracks = useCallback(async (artistId: string | number): Promise<LibraryTrack[]> => {
    const artist = artists.find(a => a.id === artistId)
    if (!artist) return []

    return rawTracks
      .filter(t => t.artist === artist.name)
      .map(toLibraryTrack)
  }, [artists, rawTracks])

  const getArtistAlbums = useCallback(async (artistId: string | number): Promise<Album[]> => {
    const artist = artists.find(a => a.id === artistId)
    if (!artist) return []

    return rawAlbums
      .filter(a => a.artist === artist.name)
      .map(toAlbum)
  }, [artists, rawAlbums])

  const getPlaylistTracks = useCallback(async (_playlistId: string): Promise<LibraryTrack[]> => {
    // Demo doesn't have playlists
    return []
  }, [])

  // Build queue from tracks
  const buildQueueFromTracks = useCallback((
    libraryTracks: LibraryTrack[],
    clickedTrack: Track,
    clickedIndex: number
  ): QueueTrack[] => {
    // Find the clicked track in the library tracks
    const validClickedIndex = libraryTracks.findIndex(t => t.id === clickedTrack.id)
    const actualIndex = validClickedIndex !== -1 ? validClickedIndex : clickedIndex

    // Build queue starting from clicked track, then wrap around
    const queue = [
      ...libraryTracks.slice(actualIndex),
      ...libraryTracks.slice(0, actualIndex),
    ].map((t): QueueTrack => ({
      trackId: String(t.id),
      title: t.title || 'Unknown',
      artist: t.artist_name || t.artist || 'Unknown Artist',
      album: t.album_title || t.album || null,
      filePath: t.file_path || t.path || '',
      durationSeconds: t.duration_seconds || t.duration || null,
      trackNumber: t.trackNumber || null,
      coverArtPath: t.coverUrl,
    }))

    // Filter out tracks without file paths and remove duplicates
    return removeConsecutiveDuplicates(
      queue.filter(t => t.filePath !== ''),
      'trackId'
    )
  }, [])

  // Empty playlists and genres for demo
  const playlists = useMemo(() => [] as Playlist[], [])
  const genres = useMemo(() => extractGenres(rawTracks), [rawTracks])

  const value: LibraryDataInterface = {
    isLoading,
    error,
    healthWarning: null, // Demo doesn't have health checks

    tracks,
    albums,
    artists,
    playlists,
    genres,

    loadLibrary,

    getTrackById,
    getAlbumById,
    getArtistById,
    getPlaylistById,

    getAlbumTracks,
    getArtistTracks,
    getArtistAlbums,
    getPlaylistTracks,

    buildQueueFromTracks,
  }

  return (
    <LibraryDataProvider value={value}>
      {children}
    </LibraryDataProvider>
  )
}
