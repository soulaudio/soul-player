/**
 * Type definitions for demo playback
 * Mirrors the Rust types from soul-playback
 */

export interface QueueTrack {
  id: string
  path: string // URL to MP3/OGG (not PathBuf)
  title: string
  artist: string
  album?: string
  duration: number // seconds (not Duration)
  trackNumber?: number
  source: TrackSource
  coverUrl?: string
}

export type TrackSource =
  | { type: 'playlist'; id: string; name: string }
  | { type: 'album'; id: string; name: string }
  | { type: 'artist'; id: string; name: string }
  | { type: 'single' }

export enum PlaybackState {
  Stopped = 'stopped',
  Playing = 'playing',
  Paused = 'paused',
  Loading = 'loading'
}

export enum RepeatMode {
  Off = 'off',
  All = 'all',
  One = 'one'
}

export enum ShuffleMode {
  Off = 'off',
  Random = 'random',
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

// Demo data structure
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

export interface DemoData {
  tracks: DemoTrack[]
  albums: DemoAlbum[]
}
