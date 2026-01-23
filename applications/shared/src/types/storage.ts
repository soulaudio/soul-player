/**
 * Storage interface for web playback providers
 *
 * Defines minimal data access requirements for playback functionality.
 * This interface decouples playback logic from data source, allowing
 * different storage implementations (JSON, API, IndexedDB, etc.)
 *
 * @module @soul-player/shared
 */

import type { DemoTrack } from '../lib/demo-storage'

/**
 * Data storage interface for web playback
 *
 * Implementations provide track lookup for metadata not stored in WASM
 * (e.g., cover art URLs, additional metadata fields).
 *
 * WASM only stores core playback fields (id, path, title, artist, duration).
 * This interface allows looking up additional UI-specific data from storage.
 *
 * @interface PlaybackDataStorage
 *
 * @example
 * ```typescript
 * // JSON-based storage
 * class DemoStorage implements PlaybackDataStorage {
 *   private tracks: DemoTrack[];
 *
 *   getTrackById(id: string) {
 *     return this.tracks.find(t => t.id === id) ?? null;
 *   }
 * }
 *
 * // API-based storage
 * class ApiStorage implements PlaybackDataStorage {
 *   async getTrackById(id: string) {
 *     const response = await fetch(`/api/tracks/${id}`);
 *     return response.json();
 *   }
 * }
 * ```
 */
export interface PlaybackDataStorage {
  /**
   * Get a track by its ID
   *
   * Used by playback provider to fetch track metadata not stored in WASM
   * (e.g., cover art URLs, genre, additional tags).
   *
   * @param id - Track ID (string format)
   * @returns Track object with full metadata, or null if not found
   *
   * @example
   * ```typescript
   * const track = storage.getTrackById('123');
   * if (track) {
   *   console.log(track.coverUrl); // Cover art URL from storage
   * }
   * ```
   */
  getTrackById(id: string): DemoTrack | null
}
