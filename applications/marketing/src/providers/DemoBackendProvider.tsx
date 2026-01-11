'use client'

/**
 * Demo Backend Provider - implements BackendInterface using DemoStorage
 * Used for marketing demo with WASM playback
 */

import { ReactNode, useMemo, useCallback, useRef } from 'react'
import {
  BackendProvider,
  type BackendInterface,
  type BackendTrack,
  type BackendAlbum,
  type BackendArtist,
  type BackendPlaylist,
  type BackendGenre,
  type DatabaseHealth,
  type BackendPlaybackContext,
  usePlayerCommands,
} from '@soul-player/shared'
import { getDemoStorage } from '@/lib/demo/storage'
import type { DemoTrack, DemoAlbum } from '@/lib/demo/types'

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

function generateMockPlaylists(tracks: DemoTrack[], albums: DemoAlbum[]): MockPlaylist[] {
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
function toBackendAlbum(da: DemoAlbum, index?: number): BackendAlbum {
  return {
    id: parseInt(da.id, 10) || index || 0,
    title: da.title,
    artist_name: da.artist,
    year: da.year,
    track_count: da.trackIds.length,
    cover_art_path: da.coverUrl,
  }
}

interface DemoBackendProviderProps {
  children: ReactNode
}

export function DemoBackendProvider({ children }: DemoBackendProviderProps) {
  const commands = usePlayerCommands()
  const storage = getDemoStorage()

  // Generate mock playlists once per session
  const mockPlaylistsRef = useRef<MockPlaylist[] | null>(null)
  const getMockPlaylists = useCallback((): MockPlaylist[] => {
    if (!mockPlaylistsRef.current) {
      mockPlaylistsRef.current = generateMockPlaylists(
        storage.getAllTracks(),
        storage.getAllAlbums()
      )
    }
    return mockPlaylistsRef.current
  }, [storage])

  // Extract unique artists from tracks
  const getArtistsFromTracks = useCallback((): BackendArtist[] => {
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

    return Array.from(artistMap.entries()).map(([name, data], index) => ({
      id: index + 1,
      name,
      track_count: data.trackCount,
      album_count: data.albumTitles.size,
    }))
  }, [storage])

  const backend = useMemo<BackendInterface>(() => ({
    // Library data
    async getAllTracks() {
      return storage.getAllTracks().map((t, i) => toBackendTrack(t, i))
    },

    async getAllAlbums() {
      return storage.getAllAlbums().map((a, i) => toBackendAlbum(a, i))
    },

    async getAllArtists() {
      return getArtistsFromTracks()
    },

    async getAllPlaylists(): Promise<BackendPlaylist[]> {
      // Return mock playlists generated from demo tracks
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
      // Demo doesn't have genres
      return []
    },

    // Single item lookups
    async getAlbumById(id: number) {
      const albums = storage.getAllAlbums()
      const album = albums.find(a => parseInt(a.id, 10) === id || a.id === String(id))
      return album ? toBackendAlbum(album) : null
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

      return storage.getAllAlbums()
        .filter(a => a.artist === artist.name)
        .map((a, i) => toBackendAlbum(a, i))
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
    async getRecentContexts(limit: number): Promise<BackendPlaybackContext[]> {
      const contexts: BackendPlaybackContext[] = []
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
            playedAt: new Date(Date.now() - contexts.length * 3600000).toISOString(), // Mock played times
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

    async recordContext(_context: Omit<BackendPlaybackContext, 'id' | 'playedAt'>) {
      // No-op for demo
    },

    // Playlist operations - not supported in demo
    async createPlaylist(_name: string, _description?: string): Promise<BackendPlaylist> {
      throw new Error('Playlist creation not supported in demo')
    },

    async deletePlaylist(_id: string) {
      throw new Error('Playlist deletion not supported in demo')
    },

    // Track operations - not supported in demo
    async deleteTrack(_id: number) {
      throw new Error('Track deletion not supported in demo')
    },

    // Queue/playback - delegate to PlayerCommands
    async playQueue(queue, startIndex) {
      await commands.playQueue(queue.map(t => ({
        trackId: t.trackId,
        title: t.title,
        artist: t.artist,
        album: t.album,
        filePath: t.filePath,
        durationSeconds: t.durationSeconds,
        trackNumber: t.trackNumber,
      })), startIndex)
    },

    // Onboarding - not needed for demo
    async checkOnboardingNeeded() {
      return false
    },
  }), [storage, getArtistsFromTracks, getMockPlaylists, commands])

  return (
    <BackendProvider value={backend}>
      {children}
    </BackendProvider>
  )
}
