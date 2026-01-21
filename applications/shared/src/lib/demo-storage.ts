/**
 * Demo storage - loads tracks and albums from JSON
 * Reusable across marketing demo and web demo mode
 */

export interface DemoTrack {
  id: string
  title: string
  artist: string
  album?: string
  duration: number
  trackNumber?: number
  path: string
  coverUrl?: string
}

export interface DemoAlbum {
  id: string
  title: string
  artist: string
  year: number
  trackIds: string[]
  coverUrl?: string
}

export interface DemoPlaylist {
  id: string
  name: string
  description?: string
  trackIds: string[]
  created_at: string
  updated_at: string
}

export interface DemoData {
  tracks: DemoTrack[]
  albums: DemoAlbum[]
  playlists?: DemoPlaylist[]
}

export class DemoStorage {
  private tracks: Map<string, DemoTrack> = new Map()
  private albums: Map<string, DemoAlbum> = new Map()
  private playlists: Map<string, DemoPlaylist> = new Map()
  private loaded: boolean = false
  private nextPlaylistId: number = 1

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

      // Index playlists (optional in demo data)
      this.playlists.clear()
      if (data.playlists) {
        data.playlists.forEach(playlist => {
          this.playlists.set(playlist.id, playlist)
          // Track highest ID for generating new ones
          const idNum = parseInt(playlist.id, 10)
          if (!isNaN(idNum) && idNum >= this.nextPlaylistId) {
            this.nextPlaylistId = idNum + 1
          }
        })
      }

      this.loaded = true
    } catch (error) {
      debug.error('Failed to load demo data:', error)
      throw error
    }
  }

  /**
   * Load demo data from object (for testing or pre-loaded data)
   */
  loadFromData(data: DemoData): void {
    this.tracks.clear()
    data.tracks.forEach(track => {
      this.tracks.set(track.id, track)
    })

    this.albums.clear()
    data.albums.forEach(album => {
      this.albums.set(album.id, album)
    })

    this.playlists.clear()
    if (data.playlists) {
      data.playlists.forEach(playlist => {
        this.playlists.set(playlist.id, playlist)
        const idNum = parseInt(playlist.id, 10)
        if (!isNaN(idNum) && idNum >= this.nextPlaylistId) {
          this.nextPlaylistId = idNum + 1
        }
      })
    }

    this.loaded = true
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
   * Get all playlists
   */
  getAllPlaylists(): DemoPlaylist[] {
    return Array.from(this.playlists.values())
  }

  /**
   * Get playlist by ID
   */
  getPlaylistById(id: string): DemoPlaylist | null {
    return this.playlists.get(id) || null
  }

  /**
   * Get tracks for a playlist
   */
  getPlaylistTracks(playlistId: string): DemoTrack[] {
    const playlist = this.playlists.get(playlistId)
    if (!playlist) return []

    return playlist.trackIds
      .map(id => this.tracks.get(id))
      .filter((track): track is DemoTrack => track !== undefined)
  }

  /**
   * Create a new playlist (in-memory only)
   */
  createPlaylist(name: string, description?: string): DemoPlaylist {
    const id = String(this.nextPlaylistId++)
    const now = new Date().toISOString()

    const playlist: DemoPlaylist = {
      id,
      name,
      description,
      trackIds: [],
      created_at: now,
      updated_at: now,
    }

    this.playlists.set(id, playlist)
    return playlist
  }

  /**
   * Update playlist metadata (in-memory only)
   */
  updatePlaylist(id: string, updates: { name?: string; description?: string }): DemoPlaylist | null {
    const playlist = this.playlists.get(id)
    if (!playlist) return null

    const updated: DemoPlaylist = {
      ...playlist,
      ...updates,
      updated_at: new Date().toISOString(),
    }

    this.playlists.set(id, updated)
    return updated
  }

  /**
   * Delete a playlist (in-memory only)
   */
  deletePlaylist(id: string): boolean {
    return this.playlists.delete(id)
  }

  /**
   * Add track to playlist (in-memory only)
   */
  addTrackToPlaylist(playlistId: string, trackId: string): boolean {
    const playlist = this.playlists.get(playlistId)
    if (!playlist) return false

    // Check if track exists
    if (!this.tracks.has(trackId)) return false

    // Don't add duplicates
    if (playlist.trackIds.includes(trackId)) return true

    playlist.trackIds.push(trackId)
    playlist.updated_at = new Date().toISOString()
    return true
  }

  /**
   * Remove track from playlist (in-memory only)
   */
  removeTrackFromPlaylist(playlistId: string, trackId: string): boolean {
    const playlist = this.playlists.get(playlistId)
    if (!playlist) return false

    const index = playlist.trackIds.indexOf(trackId)
    if (index === -1) return false

    playlist.trackIds.splice(index, 1)
    playlist.updated_at = new Date().toISOString()
    return true
  }

  /**
   * Get playlists containing a track
   */
  getPlaylistsContainingTrack(trackId: string): DemoPlaylist[] {
    return Array.from(this.playlists.values()).filter(playlist =>
      playlist.trackIds.includes(trackId)
    )
  }

  /**
   * Check if data is loaded
   */
  isLoaded(): boolean {
    return this.loaded
  }

  /**
   * Clear all data
   */
  clear(): void {
    this.tracks.clear()
    this.albums.clear()
    this.playlists.clear()
    this.loaded = false
    this.nextPlaylistId = 1
  }
}
