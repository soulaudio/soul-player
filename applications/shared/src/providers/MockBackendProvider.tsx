/**
 * Mock Backend Provider - implements BackendInterface using DemoStorage
 * Reusable for marketing demo, web demo mode, and testing
 */

import { ReactNode, useMemo, useCallback, useRef, useEffect, useState } from 'react'
import { BackendProvider } from '../contexts/BackendContext'
import type {
  BackendInterface,
  BackendTrack,
  BackendAlbum,
  BackendArtist,
  BackendPlaylist,
  BackendGenre,
  DatabaseHealth,
  PlaybackContext,
} from '../contexts/BackendContext'
import { DemoStorage, DemoTrack, DemoAlbum } from '../lib/demo-storage'

// Seeded random for consistent demo experience per session
function seededRandom(seed: number) {
  const x = Math.sin(seed++) * 10000
  return x - Math.floor(x)
}

// Fisher-Yates shuffle with seed
function shuffleArray<T>(array: T[], seed: number): T[] {
  const result = [...array]
  for (let i = result.length - 1; i > 0; i--) {
    const j = Math.floor(seededRandom(seed + i) * (i + 1))
    ;[result[i], result[j]] = [result[j], result[i]]
  }
  return result
}

// Generate mock playlists from tracks
interface MockPlaylist {
  id: string
  name: string
  description: string
  trackIds: string[]
  coverUrl?: string
}

function generateMockPlaylists(tracks: DemoTrack[], _albums: DemoAlbum[]): MockPlaylist[] {
  if (tracks.length === 0) return []

  const seed = Date.now() % 1000 // Session-based seed
  const shuffledTracks = shuffleArray(tracks, seed)

  const playlists: MockPlaylist[] = []

  // Playlist 1: "Favorites" - random selection
  const favoriteTracks = shuffledTracks.slice(0, Math.min(8, tracks.length))
  if (favoriteTracks.length > 0) {
    playlists.push({
      id: 'favorites',
      name: 'Favorites',
      description: 'Your most played tracks',
      trackIds: favoriteTracks.map(t => t.id),
      coverUrl: favoriteTracks[0]?.coverUrl,
    })
  }

  // Playlist 2: "Recently Added" - last few tracks
  const recentTracks = tracks.slice(-Math.min(6, tracks.length))
  if (recentTracks.length > 0) {
    playlists.push({
      id: 'recent',
      name: 'Recently Added',
      description: 'Fresh additions to your library',
      trackIds: recentTracks.map(t => t.id),
      coverUrl: recentTracks[recentTracks.length - 1]?.coverUrl,
    })
  }

  // Playlist 3: "Chill Mix" - random subset
  const chillTracks = shuffleArray(tracks, seed + 100).slice(0, Math.min(10, tracks.length))
  if (chillTracks.length > 0) {
    playlists.push({
      id: 'chill',
      name: 'Chill Mix',
      description: 'Relaxing tunes for any mood',
      trackIds: chillTracks.map(t => t.id),
      coverUrl: chillTracks[0]?.coverUrl,
    })
  }

  // Playlist 4: "Discovery Mix" - another random subset
  const discoveryTracks = shuffleArray(tracks, seed + 200).slice(0, Math.min(12, tracks.length))
  if (discoveryTracks.length > 0) {
    playlists.push({
      id: 'discovery',
      name: 'Discovery Mix',
      description: 'Explore new sounds',
      trackIds: discoveryTracks.map(t => t.id),
      coverUrl: discoveryTracks[0]?.coverUrl,
    })
  }

  return playlists
}

// Convert DemoTrack to BackendTrack
function toBackendTrack(dt: DemoTrack, index?: number): BackendTrack {
  return {
    id: parseInt(dt.id, 10) || index || 0,
    title: dt.title,
    artist_name: dt.artist,
    album_title: dt.album,
    duration_seconds: dt.duration,
    file_path: dt.path,
    track_number: dt.trackNumber,
    cover_art_path: dt.coverUrl,
  }
}

// Convert DemoAlbum to BackendAlbum
function toBackendAlbum(da: DemoAlbum, index?: number, artistIdMap?: Map<string, number>): BackendAlbum {
  return {
    id: parseInt(da.id, 10) || index || 0,
    title: da.title,
    artist_id: artistIdMap?.get(da.artist),
    artist_name: da.artist,
    year: da.year,
    track_count: da.trackIds.length,
    cover_art_path: da.coverUrl,
  }
}

interface MockBackendProviderProps {
  storage: DemoStorage
  children: ReactNode
}

export function MockBackendProvider({ storage, children }: MockBackendProviderProps) {
  // Initialize default playlists if storage doesn't have any
  const playlistsInitialized = useRef(false)
  const [storageLoaded, setStorageLoaded] = useState(storage.isLoaded())

  // Track when storage finishes loading
  useEffect(() => {
    if (storage.isLoaded() && !storageLoaded) {
      setStorageLoaded(true)
    }
  }, [storage, storageLoaded])

  // Initialize playlists when storage is loaded
  useEffect(() => {
    if (!playlistsInitialized.current && storageLoaded && storage.getAllPlaylists().length === 0) {
      playlistsInitialized.current = true
      const mockPlaylists = generateMockPlaylists(storage.getAllTracks(), storage.getAllAlbums())

      // Transfer mock playlists to storage
      mockPlaylists.forEach(mp => {
        const playlist = storage.createPlaylist(mp.name, mp.description)
        mp.trackIds.forEach(trackId => {
          storage.addTrackToPlaylist(playlist.id, trackId)
        })
      })

      console.log('[MockBackendProvider] Initialized', mockPlaylists.length, 'default playlists')
    }
  }, [storage, storageLoaded])

  // Keep getMockPlaylists for backwards compatibility (now uses storage)
  const getMockPlaylists = useCallback((): MockPlaylist[] => {
    return storage.getAllPlaylists().map(p => ({
      id: p.id,
      name: p.name,
      description: p.description || '',
      trackIds: p.trackIds,
      coverUrl: storage.getTrackById(p.trackIds[0])?.coverUrl,
    }))
  }, [storage])

  // Extract unique artists from tracks (cached for consistent IDs)
  const artistsRef = useRef<{ artists: BackendArtist[]; idMap: Map<string, number> } | null>(null)
  const getArtistsData = useCallback(() => {
    if (!artistsRef.current) {
      const tracks = storage.getAllTracks()
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

      const artists = Array.from(artistMap.entries()).map(([name, data], index) => ({
        id: index + 1,
        name,
        track_count: data.trackCount,
        album_count: data.albumTitles.size,
      }))

      const idMap = new Map<string, number>()
      artists.forEach(a => idMap.set(a.name, a.id))

      artistsRef.current = { artists, idMap }
    }
    return artistsRef.current
  }, [storage])

  const getArtistsFromTracks = useCallback((): BackendArtist[] => {
    return getArtistsData().artists
  }, [getArtistsData])

  const getArtistIdMap = useCallback((): Map<string, number> => {
    return getArtistsData().idMap
  }, [getArtistsData])

  const backend = useMemo<BackendInterface>(() => ({
    // Library data
    async getAllTracks() {
      return storage.getAllTracks().map((t, i) => toBackendTrack(t, i))
    },

    async getAllAlbums() {
      const artistIdMap = getArtistIdMap()
      return storage.getAllAlbums().map((a, i) => toBackendAlbum(a, i, artistIdMap))
    },

    async getRandomAlbums(limit: number) {
      const artistIdMap = getArtistIdMap()
      const allAlbums = storage.getAllAlbums()
      const seed = Date.now() % 10000
      const shuffled = shuffleArray(allAlbums, seed)
      return shuffled.slice(0, limit).map((a, i) => toBackendAlbum(a, i, artistIdMap))
    },

    async getRecentlyAddedAlbums(limit: number) {
      const artistIdMap = getArtistIdMap()
      const allAlbums = storage.getAllAlbums()
      // In demo mode, just return last N albums (reversed to show newest first)
      return allAlbums.slice(-limit).reverse().map((a, i) => toBackendAlbum(a, i, artistIdMap))
    },

    async getRecentlyAddedAlbumsWithinDays(_days: number, limit: number) {
      const artistIdMap = getArtistIdMap()
      const allAlbums = storage.getAllAlbums()
      // In demo mode, simulate recent albums (always return some to show the section)
      const recentCount = Math.min(5, allAlbums.length)
      return allAlbums.slice(-recentCount).reverse().slice(0, limit).map((a, i) => toBackendAlbum(a, i, artistIdMap))
    },

    async getLeastPlayedAlbums(limit: number) {
      const artistIdMap = getArtistIdMap()
      const allAlbums = storage.getAllAlbums()
      // In demo mode, shuffle with a different seed and take first N
      const seed = 12345 // Fixed seed for consistency
      const shuffled = shuffleArray(allAlbums, seed)
      return shuffled.slice(0, limit).map((a, i) => toBackendAlbum(a, i, artistIdMap))
    },

    async getTimeCapsuleAlbums(limit: number) {
      const artistIdMap = getArtistIdMap()
      const allAlbums = storage.getAllAlbums()
      // Filter albums by year matching current month/day from previous years
      const now = new Date()
      const currentMonth = now.getMonth() + 1
      const currentDay = now.getDate()

      const matching = allAlbums.filter(a => {
        if (!a.year) return false
        // Simple heuristic: return albums from different years
        // In real app, this would check play history from this day in previous years
        return a.year < now.getFullYear() - 1
      })

      if (matching.length === 0) return []

      const seed = currentMonth * 100 + currentDay
      const shuffled = shuffleArray(matching, seed)
      return shuffled.slice(0, limit).map((a, i) => toBackendAlbum(a, i, artistIdMap))
    },

    async getGenreAlbums(genreId: number, limit: number) {
      const artistIdMap = getArtistIdMap()
      const allAlbums = storage.getAllAlbums()
      // In demo mode, just return random albums
      // In real app, this would filter by genre
      const seed = genreId * 1000
      const shuffled = shuffleArray(allAlbums, seed)
      return shuffled.slice(0, limit).map((a, i) => toBackendAlbum(a, i, artistIdMap))
    },

    async getAllArtists() {
      return getArtistsFromTracks()
    },

    async getAllPlaylists(): Promise<BackendPlaylist[]> {
      return getMockPlaylists().map((p) => ({
        id: p.id,
        name: p.name,
        description: p.description,
        track_count: p.trackIds.length,
        owner_id: 1,
        is_public: false,
        is_favorite: false,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      }))
    },

    async getAllGenres(): Promise<BackendGenre[]> {
      return []
    },

    // Single item lookups
    async getAlbumById(id: number) {
      const albums = storage.getAllAlbums()
      const album = albums.find(a => parseInt(a.id, 10) === id || a.id === String(id))
      return album ? toBackendAlbum(album, undefined, getArtistIdMap()) : null
    },

    async getArtistById(id: number) {
      const artists = getArtistsFromTracks()
      return artists.find(a => a.id === id) || null
    },

    async getPlaylistById(id: string): Promise<BackendPlaylist | null> {
      const playlist = getMockPlaylists().find(p => p.id === id)
      if (!playlist) return null
      return {
        id: playlist.id,
        name: playlist.name,
        description: playlist.description,
        track_count: playlist.trackIds.length,
        owner_id: 1,
        is_public: false,
        is_favorite: false,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      }
    },

    async getGenreById(_id: number): Promise<BackendGenre | null> {
      return null
    },

    // Related data
    async getAlbumTracks(albumId: number) {
      const albums = storage.getAllAlbums()
      const album = albums.find(a => parseInt(a.id, 10) === albumId || a.id === String(albumId))
      if (!album) return []

      return storage.getAlbumTracks(album.id).map((t, i) => toBackendTrack(t, i))
    },

    async getArtistTracks(artistId: number) {
      const artists = getArtistsFromTracks()
      const artist = artists.find(a => a.id === artistId)
      if (!artist) return []

      return storage.getTracksByArtist(artist.name).map((t, i) => toBackendTrack(t, i))
    },

    async getArtistAlbums(artistId: number) {
      const artists = getArtistsFromTracks()
      const artist = artists.find(a => a.id === artistId)
      if (!artist) return []

      const artistIdMap = getArtistIdMap()
      return storage.getAllAlbums()
        .filter(a => a.artist === artist.name)
        .map((a, i) => toBackendAlbum(a, i, artistIdMap))
    },

    async getArtistTopTracks(artistId: number, limit = 10) {
      const artists = getArtistsFromTracks()
      const artist = artists.find(a => a.id === artistId)
      if (!artist) return []

      // Demo: Return random selection of artist tracks (no real play count data)
      const artistTracks = storage.getTracksByArtist(artist.name).map((t, i) => toBackendTrack(t, i))

      // Shuffle and take first N tracks
      const seed = artistId * 100 // Use artistId as seed for consistency
      const shuffled = shuffleArray(artistTracks, seed)
      return shuffled.slice(0, Math.min(limit, shuffled.length))
    },

    async getPlaylistTracks(playlistId: string): Promise<BackendTrack[]> {
      const playlist = getMockPlaylists().find(p => p.id === playlistId)
      if (!playlist) return []

      return playlist.trackIds
        .map(id => storage.getTrackById(id))
        .filter((t): t is DemoTrack => t !== null)
        .map((t, i) => toBackendTrack(t, i))
    },

    async getGenreTracks(_genreId: number): Promise<BackendTrack[]> {
      return []
    },

    // Health check
    async checkDatabaseHealth(): Promise<DatabaseHealth> {
      const tracks = storage.getAllTracks()
      return {
        total_tracks: tracks.length,
        tracks_with_availability: tracks.length,
        tracks_with_local_files: tracks.filter(t => t.path).length,
        issues: [],
      }
    },

    // Playback context - return mock "Jump Back In" data
    async getRecentContexts(limit: number): Promise<PlaybackContext[]> {
      const contexts: PlaybackContext[] = []
      const albums = storage.getAllAlbums()
      const playlists = getMockPlaylists()

      // Use session-based seed for consistent ordering
      const seed = Date.now() % 1000
      const shuffledAlbums = shuffleArray(albums, seed)
      const shuffledPlaylists = shuffleArray(playlists, seed + 50)

      // Mix albums and playlists
      let albumIndex = 0
      let playlistIndex = 0

      while (contexts.length < limit && (albumIndex < shuffledAlbums.length || playlistIndex < shuffledPlaylists.length)) {
        // Alternate between albums and playlists
        if (albumIndex < shuffledAlbums.length && (contexts.length % 3 !== 2 || playlistIndex >= shuffledPlaylists.length)) {
          const album = shuffledAlbums[albumIndex++]
          contexts.push({
            id: contexts.length + 1,
            contextType: 'album',
            contextId: album.id,
            contextName: album.title,
            contextArtworkPath: album.coverUrl || null,
            playedAt: new Date(Date.now() - contexts.length * 3600000).toISOString(),
          })
        } else if (playlistIndex < shuffledPlaylists.length) {
          const playlist = shuffledPlaylists[playlistIndex++]
          contexts.push({
            id: contexts.length + 1,
            contextType: 'playlist',
            contextId: playlist.id,
            contextName: playlist.name,
            contextArtworkPath: playlist.coverUrl || null,
            playedAt: new Date(Date.now() - contexts.length * 3600000).toISOString(),
          })
        }
      }

      return contexts.slice(0, limit)
    },

    async recordContext(_context: Omit<PlaybackContext, 'id' | 'playedAt'>) {
      // No-op for mock
    },

    // Playlist operations - supported in-memory
    async createPlaylist(name: string, description?: string): Promise<BackendPlaylist> {
      const playlist = storage.createPlaylist(name, description)
      return {
        id: playlist.id,
        name: playlist.name,
        description: playlist.description,
        track_count: playlist.trackIds.length,
        owner_id: 1,
        is_public: false,
        is_favorite: false,
        created_at: playlist.created_at,
        updated_at: playlist.updated_at,
      }
    },

    async deletePlaylist(id: string) {
      const success = storage.deletePlaylist(id)
      if (!success) {
        throw new Error(`Playlist ${id} not found`)
      }
    },

    async getPlaylistsContainingTrack(trackId: number): Promise<string[]> {
      const playlists = storage.getPlaylistsContainingTrack(String(trackId))
      return playlists.map(p => p.id)
    },

    async addTrackToPlaylist(playlistId: string, trackId: number) {
      const success = storage.addTrackToPlaylist(playlistId, String(trackId))
      if (!success) {
        throw new Error(`Failed to add track ${trackId} to playlist ${playlistId}`)
      }
    },

    async removeTrackFromPlaylist(playlistId: string, trackId: number) {
      const success = storage.removeTrackFromPlaylist(playlistId, String(trackId))
      if (!success) {
        throw new Error(`Failed to remove track ${trackId} from playlist ${playlistId}`)
      }
    },

    // Track operations - not supported in mock
    async deleteTrack(_id: number) {
      throw new Error('Track deletion not supported in demo mode')
    },

    async showInFileExplorer(_path: string) {
      console.log('[MockBackend] showInFileExplorer not supported in demo mode')
    },

    // Onboarding - not needed for mock
    async checkOnboardingNeeded() {
      return false
    },

    async getUserSetting(key: string) {
      // Mock implementation - use sessionStorage for demo
      const stored = sessionStorage.getItem(`setting:${key}`)
      return stored ? JSON.parse(stored) : null
    },

    async setUserSetting(key: string, value: any) {
      // Mock implementation - use sessionStorage for demo
      sessionStorage.setItem(`setting:${key}`, JSON.stringify(value))
    },

    // Artwork editing - not supported in mock
    async setArtwork() {
      console.log('[MockBackend] Artwork editing not supported in demo mode')
    },

    async removeArtwork() {
      console.log('[MockBackend] Artwork removal not supported in demo mode')
    },

    async getArtistArtwork() {
      return null
    },

    async getPlaylistArtwork() {
      return null
    },

    // App metadata
    async getVersion() {
      return '0.1.1 (Demo)'
    },
  }), [storage, getArtistsFromTracks, getArtistIdMap, getMockPlaylists])

  return (
    <BackendProvider value={backend}>
      {children}
    </BackendProvider>
  )
}
