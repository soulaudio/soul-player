/**
 * Demo storage - loads tracks and albums from JSON
 * Simplified version of StorageContext for demo purposes
 */

import {
  DemoData,
  DemoTrack,
  DemoAlbum,
  QueueTrack,
  TrackSource
} from './types'

export class DemoStorage {
  private tracks: Map<string, DemoTrack> = new Map()
  private albums: Map<string, DemoAlbum> = new Map()
  private loaded: boolean = false

  /**
   * Load demo data from JSON file
   */
  async loadFromJson(url: string): Promise<void> {
    try {
      const response = await fetch(url)
      if (!response.ok) {
        throw new Error(`Failed to load demo data: ${response.statusText}`)
      }

      const data: DemoData = await response.json()

      // Index tracks
      this.tracks.clear()
      data.tracks.forEach(track => {
        this.tracks.set(track.id, track)
      })

      // Index albums
      this.albums.clear()
      data.albums.forEach(album => {
        this.albums.set(album.id, album)
      })

      this.loaded = true
    } catch (error) {
      console.error('Failed to load demo data:', error)
      throw error
    }
  }

  /**
   * Get all tracks
   */
  getAllTracks(): DemoTrack[] {
    return Array.from(this.tracks.values())
  }

  /**
   * Get track by ID
   */
  getTrackById(id: string): DemoTrack | null {
    return this.tracks.get(id) || null
  }

  /**
   * Get all albums
   */
  getAllAlbums(): DemoAlbum[] {
    return Array.from(this.albums.values())
  }

  /**
   * Get album by ID
   */
  getAlbumById(id: string): DemoAlbum | null {
    return this.albums.get(id) || null
  }

  /**
   * Get tracks for an album
   */
  getAlbumTracks(albumId: string): DemoTrack[] {
    const album = this.albums.get(albumId)
    if (!album) return []

    return album.trackIds
      .map(id => this.tracks.get(id))
      .filter((track): track is DemoTrack => track !== undefined)
  }

  /**
   * Convert DemoTrack to QueueTrack for playback
   */
  toQueueTrack(track: DemoTrack, source: TrackSource = { type: 'single' }): QueueTrack {
    return {
      id: track.id,
      path: track.path,
      title: track.title,
      artist: track.artist,
      album: track.album,
      duration: track.duration,
      trackNumber: track.trackNumber,
      source,
      coverUrl: track.coverUrl
    }
  }

  /**
   * Convert album to queue tracks
   */
  albumToQueueTracks(albumId: string): QueueTrack[] {
    const album = this.albums.get(albumId)
    if (!album) return []

    const tracks = this.getAlbumTracks(albumId)
    const source: TrackSource = {
      type: 'album',
      id: album.id,
      name: album.title
    }

    return tracks.map(track => this.toQueueTrack(track, source))
  }

  /**
   * Search tracks by title, artist, or album
   */
  searchTracks(query: string): DemoTrack[] {
    const lowerQuery = query.toLowerCase()

    return Array.from(this.tracks.values()).filter(track => {
      return (
        track.title.toLowerCase().includes(lowerQuery) ||
        track.artist.toLowerCase().includes(lowerQuery) ||
        track.album?.toLowerCase().includes(lowerQuery)
      )
    })
  }

  /**
   * Get tracks by artist
   */
  getTracksByArtist(artist: string): DemoTrack[] {
    return Array.from(this.tracks.values()).filter(
      track => track.artist === artist
    )
  }

  /**
   * Get unique artists
   */
  getArtists(): string[] {
    const artists = new Set<string>()
    this.tracks.forEach(track => artists.add(track.artist))
    return Array.from(artists).sort()
  }

  /**
   * Check if data is loaded
   */
  isLoaded(): boolean {
    return this.loaded
  }
}

// Singleton instance for easy access
let demoStorageInstance: DemoStorage | null = null

export function getDemoStorage(): DemoStorage {
  if (!demoStorageInstance) {
    demoStorageInstance = new DemoStorage()
  }
  return demoStorageInstance
}

export async function initializeDemoStorage(jsonUrl: string = '/demo-data.json'): Promise<DemoStorage> {
  const storage = getDemoStorage()
  if (!storage.isLoaded()) {
    await storage.loadFromJson(jsonUrl)
  }
  return storage
}
