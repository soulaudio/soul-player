/**
 * Type definitions for WASM playback
 *
 * Defines types used for web playback with WASM queue logic.
 * Mirrors Rust types from soul-playback crate.
 *
 * CRITICAL NAMING CONVENTION: Field names must match Rust serde expectations.
 * Use snake_case (duration_secs, track_number) not camelCase (durationSecs, trackNumber).
 * Incorrect names will cause deserialization errors in WASM.
 *
 * @module @soul-player/playback-web
 */

// Re-export demo data types from shared for backward compatibility
export type { DemoTrack, DemoAlbum, DemoData } from '@soul-player/shared'

/**
 * Track in playback queue
 *
 * Represents a track that can be played by the WASM adapter.
 * All fields use snake_case naming to match Rust serde expectations.
 *
 * @interface QueueTrack
 */
export interface QueueTrack {
  /** Unique track identifier */
  id: string
  /** URL or file path to audio file (MP3, OGG, etc.) */
  path: string
  /** Track title */
  title: string
  /** Artist name (REQUIRED: never undefined, use 'Unknown Artist' as fallback) */
  artist: string
  /** Album name (optional) */
  album?: string
  /** Duration in seconds (MUST use snake_case: duration_secs) */
  duration_secs: number
  /** Track number on album (MUST use snake_case: track_number) */
  track_number?: number
  /** Source context (optional, used for "Jump Back In" feature) */
  source?: TrackSource
  /** Cover art URL (demo-specific, not stored in WASM) */
  coverUrl?: string
}

export type TrackSource =
  | { type: 'playlist'; id: string; name: string }
  | { type: 'album'; id: string; name: string }
  | { type: 'artist'; id: string; name: string }
  | { type: 'single' }

/**
 * Playback state
 *
 * Current state of the playback engine.
 *
 * @enum PlaybackState
 */
export enum PlaybackState {
  /** No track loaded or playback explicitly stopped */
  Stopped = 'stopped',
  /** Currently playing audio */
  Playing = 'playing',
  /** Playback paused (can be resumed) */
  Paused = 'paused',
  /** Loading track (transitional state) */
  Loading = 'loading'
}

/**
 * Repeat mode
 *
 * Controls queue repeat behavior.
 *
 * @enum RepeatMode
 */
export enum RepeatMode {
  /** No repeat (stop at end of queue) */
  Off = 'off',
  /** Repeat entire queue */
  All = 'all',
  /** Repeat current track only */
  One = 'one'
}

/**
 * Shuffle mode
 *
 * Controls queue randomization.
 *
 * @enum ShuffleMode
 */
export enum ShuffleMode {
  /** No shuffle (play in order) */
  Off = 'off',
  /** Random shuffle (Fisher-Yates algorithm) */
  Random = 'random',
  /** Smart shuffle (avoids artist repetition) - Future feature */
  Smart = 'smart'
}

export interface PlaybackConfig {
  historySize: number
  volume: number // 0-100
  shuffle: ShuffleMode
  repeat: RepeatMode
  gapless: boolean
}

export const defaultPlaybackConfig: PlaybackConfig = {
  historySize: 50,
  volume: 80,
  shuffle: ShuffleMode.Off,
  repeat: RepeatMode.Off,
  gapless: true
}
