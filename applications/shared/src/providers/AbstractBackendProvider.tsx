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

import React, { useMemo } from 'react';
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
import { BackendProvider } from '../contexts/BackendContext';
import { debug } from '../utils/debug';

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

  async getRandomAlbums(_limit: number): Promise<BackendAlbum[]> {
    throw new Error('getRandomAlbums() not implemented');
  }

  async getRecentlyAddedAlbums(_limit: number): Promise<BackendAlbum[]> {
    throw new Error('getRecentlyAddedAlbums() not implemented');
  }

  async getRecentlyAddedAlbumsWithinDays(_days: number, _limit: number): Promise<BackendAlbum[]> {
    throw new Error('getRecentlyAddedAlbumsWithinDays() not implemented');
  }

  async getLeastPlayedAlbums(_limit: number): Promise<BackendAlbum[]> {
    throw new Error('getLeastPlayedAlbums() not implemented');
  }

  async getTimeCapsuleAlbums(_limit: number): Promise<BackendAlbum[]> {
    return []; // Optional feature, return empty by default
  }

  async getGenreAlbums(_genreId: number, _limit: number): Promise<BackendAlbum[]> {
    throw new Error('getGenreAlbums() not implemented');
  }

  // ============================================================================
  // Single Item Lookups
  // ============================================================================

  async getAlbumById(_id: number): Promise<BackendAlbum | null> {
    throw new Error('getAlbumById() not implemented');
  }

  async getArtistById(_id: number): Promise<BackendArtist | null> {
    throw new Error('getArtistById() not implemented');
  }

  async getPlaylistById(_id: string): Promise<BackendPlaylist | null> {
    throw new Error('getPlaylistById() not implemented');
  }

  async getGenreById(_id: number): Promise<BackendGenre | null> {
    throw new Error('getGenreById() not implemented');
  }

  // ============================================================================
  // Related Data
  // ============================================================================

  async getAlbumTracks(_albumId: number): Promise<BackendTrack[]> {
    throw new Error('getAlbumTracks() not implemented');
  }

  async getArtistTracks(_artistId: number): Promise<BackendTrack[]> {
    throw new Error('getArtistTracks() not implemented');
  }

  async getArtistAlbums(_artistId: number): Promise<BackendAlbum[]> {
    throw new Error('getArtistAlbums() not implemented');
  }

  async getArtistTopTracks(_artistId: number, _limit?: number): Promise<BackendTrack[]> {
    throw new Error('getArtistTopTracks() not implemented');
  }

  async getPlaylistTracks(_playlistId: string): Promise<BackendTrack[]> {
    throw new Error('getPlaylistTracks() not implemented');
  }

  async getGenreTracks(_genreId: number): Promise<BackendTrack[]> {
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

  async getRecentContexts(_limit: number): Promise<PlaybackContext[]> {
    // Default: no recent contexts
    return [];
  }

  async recordContext(_context: Omit<PlaybackContext, 'id' | 'playedAt'>): Promise<void> {
    // Default: no-op
    debug.log('[AbstractBackend] recordContext not implemented');
  }

  // ============================================================================
  // Playlist Operations
  // ============================================================================

  async createPlaylist(_name: string, _description?: string): Promise<BackendPlaylist> {
    throw new Error('createPlaylist() not supported');
  }

  async deletePlaylist(_id: string): Promise<void> {
    throw new Error('deletePlaylist() not supported');
  }

  async getPlaylistsContainingTrack(_trackId: number): Promise<string[]> {
    // Default: track not in any playlists
    return [];
  }

  async addTrackToPlaylist(_playlistId: string, _trackId: number): Promise<void> {
    throw new Error('addTrackToPlaylist() not supported');
  }

  async removeTrackFromPlaylist(_playlistId: string, _trackId: number): Promise<void> {
    throw new Error('removeTrackFromPlaylist() not supported');
  }

  // ============================================================================
  // Track Operations
  // ============================================================================

  async deleteTrack(_id: number): Promise<void> {
    throw new Error('deleteTrack() not supported');
  }

  async showInFileExplorer(_path: string): Promise<void> {
    debug.log('[AbstractBackend] showInFileExplorer not supported on this platform');
  }

  // ============================================================================
  // Onboarding
  // ============================================================================

  async checkOnboardingNeeded(): Promise<boolean> {
    // Default: no onboarding needed
    return false;
  }

  async getUserSetting(_key: string): Promise<any> {
    return null; // Default: no setting value
  }

  async setUserSetting(_key: string, _value: any): Promise<void> {
    // Default: no-op
    debug.log('[AbstractBackend] setUserSetting not implemented');
  }

  // ============================================================================
  // Artwork Operations
  // ============================================================================

  async setArtwork(_params: SetArtworkParams): Promise<void> {
    throw new Error('setArtwork() not supported');
  }

  async removeArtwork(_entityType: 'album' | 'artist' | 'playlist', _entityId: string): Promise<void> {
    throw new Error('removeArtwork() not supported');
  }

  async getArtistArtwork(_artistId: number): Promise<string | null> {
    return null;
  }

  async getPlaylistArtwork(_playlistId: string): Promise<string | null> {
    return null;
  }

  // ============================================================================
  // App Metadata
  // ============================================================================

  async getVersion(): Promise<string> {
    throw new Error('getVersion() not implemented');
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
    const backend = useMemo(() => new backendClass(), []);

    return BackendProvider({ value: backend, children });
  };
}
