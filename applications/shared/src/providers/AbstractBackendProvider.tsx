/**
 * Abstract Backend Provider - Base class for implementing BackendInterface
 *
 * This class provides default implementations that throw "not implemented" errors.
 * Platform-specific providers can extend this and override only the methods they need.
 *
 * Usage:
 *
 * ```typescript
 * class MyBackendProvider extends AbstractBackendProvider {
 *   async getAllTracks() {
 *     // Your implementation
 *   }
 *
 *   async getAlbumTracks(albumId: number) {
 *     // Your implementation
 *   }
 *
 *   // ... override other methods as needed
 * }
 *
 * // Then use it:
 * const backend = new MyBackendProvider();
 * ```
 *
 * Note: This is primarily useful for documenting the interface structure.
 * In practice, most backends will implement all methods from scratch.
 */

import type {
  BackendInterface,
  BackendTrack,
  BackendAlbum,
  BackendArtist,
  BackendPlaylist,
  BackendGenre,
  DatabaseHealth,
  PlaybackContext,
  SetArtworkParams,
} from '../contexts/BackendContext';

export abstract class AbstractBackendProvider implements BackendInterface {
  // ============================================================================
  // Library Data - Core Collections
  // ============================================================================

  async getAllTracks(): Promise<BackendTrack[]> {
    throw new Error('getAllTracks() not implemented');
  }

  async getAllAlbums(): Promise<BackendAlbum[]> {
    throw new Error('getAllAlbums() not implemented');
  }

  async getAllArtists(): Promise<BackendArtist[]> {
    throw new Error('getAllArtists() not implemented');
  }

  async getAllPlaylists(): Promise<BackendPlaylist[]> {
    throw new Error('getAllPlaylists() not implemented');
  }

  async getAllGenres(): Promise<BackendGenre[]> {
    throw new Error('getAllGenres() not implemented');
  }

  async getRandomAlbums(limit: number): Promise<BackendAlbum[]> {
    throw new Error('getRandomAlbums() not implemented');
  }

  async getRecentlyAddedAlbums(limit: number): Promise<BackendAlbum[]> {
    throw new Error('getRecentlyAddedAlbums() not implemented');
  }

  async getRecentlyAddedAlbumsWithinDays(days: number, limit: number): Promise<BackendAlbum[]> {
    throw new Error('getRecentlyAddedAlbumsWithinDays() not implemented');
  }

  async getLeastPlayedAlbums(limit: number): Promise<BackendAlbum[]> {
    throw new Error('getLeastPlayedAlbums() not implemented');
  }

  async getTimeCapsuleAlbums(limit: number): Promise<BackendAlbum[]> {
    return []; // Optional feature, return empty by default
  }

  async getGenreAlbums(genreId: number, limit: number): Promise<BackendAlbum[]> {
    throw new Error('getGenreAlbums() not implemented');
  }

  // ============================================================================
  // Single Item Lookups
  // ============================================================================

  async getAlbumById(id: number): Promise<BackendAlbum | null> {
    throw new Error('getAlbumById() not implemented');
  }

  async getArtistById(id: number): Promise<BackendArtist | null> {
    throw new Error('getArtistById() not implemented');
  }

  async getPlaylistById(id: string): Promise<BackendPlaylist | null> {
    throw new Error('getPlaylistById() not implemented');
  }

  async getGenreById(id: number): Promise<BackendGenre | null> {
    throw new Error('getGenreById() not implemented');
  }

  // ============================================================================
  // Related Data
  // ============================================================================

  async getAlbumTracks(albumId: number): Promise<BackendTrack[]> {
    throw new Error('getAlbumTracks() not implemented');
  }

  async getArtistTracks(artistId: number): Promise<BackendTrack[]> {
    throw new Error('getArtistTracks() not implemented');
  }

  async getArtistAlbums(artistId: number): Promise<BackendAlbum[]> {
    throw new Error('getArtistAlbums() not implemented');
  }

  async getArtistTopTracks(artistId: number, limit?: number): Promise<BackendTrack[]> {
    throw new Error('getArtistTopTracks() not implemented');
  }

  async getPlaylistTracks(playlistId: string): Promise<BackendTrack[]> {
    throw new Error('getPlaylistTracks() not implemented');
  }

  async getGenreTracks(genreId: number): Promise<BackendTrack[]> {
    throw new Error('getGenreTracks() not implemented');
  }

  // ============================================================================
  // Health & Diagnostics
  // ============================================================================

  async checkDatabaseHealth(): Promise<DatabaseHealth> {
    // Default: healthy with no data
    return {
      total_tracks: 0,
      tracks_with_availability: 0,
      tracks_with_local_files: 0,
      issues: [],
    };
  }

  // ============================================================================
  // Playback Context (Jump Back In)
  // ============================================================================

  async getRecentContexts(limit: number): Promise<PlaybackContext[]> {
    // Default: no recent contexts
    return [];
  }

  async recordContext(context: Omit<PlaybackContext, 'id' | 'playedAt'>): Promise<void> {
    // Default: no-op
    console.log('[AbstractBackend] recordContext not implemented');
  }

  // ============================================================================
  // Playlist Operations
  // ============================================================================

  async createPlaylist(name: string, description?: string): Promise<BackendPlaylist> {
    throw new Error('createPlaylist() not supported');
  }

  async deletePlaylist(id: string): Promise<void> {
    throw new Error('deletePlaylist() not supported');
  }

  async getPlaylistsContainingTrack(trackId: number): Promise<string[]> {
    // Default: track not in any playlists
    return [];
  }

  async addTrackToPlaylist(playlistId: string, trackId: number): Promise<void> {
    throw new Error('addTrackToPlaylist() not supported');
  }

  async removeTrackFromPlaylist(playlistId: string, trackId: number): Promise<void> {
    throw new Error('removeTrackFromPlaylist() not supported');
  }

  // ============================================================================
  // Track Operations
  // ============================================================================

  async deleteTrack(id: number): Promise<void> {
    throw new Error('deleteTrack() not supported');
  }

  async showInFileExplorer(path: string): Promise<void> {
    console.log('[AbstractBackend] showInFileExplorer not supported on this platform');
  }

  // ============================================================================
  // Onboarding
  // ============================================================================

  async checkOnboardingNeeded(): Promise<boolean> {
    // Default: no onboarding needed
    return false;
  }

  async getUserSetting(key: string): Promise<any> {
    return null; // Default: no setting value
  }

  async setUserSetting(key: string, value: any): Promise<void> {
    // Default: no-op
    console.log('[AbstractBackend] setUserSetting not implemented');
  }

  // ============================================================================
  // Artwork Operations
  // ============================================================================

  async setArtwork(params: SetArtworkParams): Promise<void> {
    throw new Error('setArtwork() not supported');
  }

  async removeArtwork(entityType: 'album' | 'artist' | 'playlist', entityId: string): Promise<void> {
    throw new Error('removeArtwork() not supported');
  }

  async getArtistArtwork(artistId: number): Promise<string | null> {
    return null;
  }

  async getPlaylistArtwork(playlistId: string): Promise<string | null> {
    return null;
  }
}

/**
 * Helper: Create a BackendProvider component from a class instance
 *
 * Usage:
 * ```typescript
 * class MyBackend extends AbstractBackendProvider {
 *   // ... implementation
 * }
 *
 * function MyBackendProvider({ children }) {
 *   const backend = useMemo(() => new MyBackend(), []);
 *   return <BackendProvider value={backend}>{children}</BackendProvider>;
 * }
 * ```
 */
export function createBackendProvider(
  backendClass: new () => BackendInterface
): React.ComponentType<{ children: React.ReactNode }> {
  return function BackendProviderComponent({ children }) {
    const { useMemo } = require('react');
    const { BackendProvider } = require('../contexts/BackendContext');

    const backend = useMemo(() => new backendClass(), []);

    return BackendProvider({ value: backend, children });
  };
}
