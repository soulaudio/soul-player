/**
 * WASM-specific conversion helpers
 * Converts DemoTrack to QueueTrack format expected by WASM playback manager
 */

import type { DemoTrack } from '@soul-player/shared'
import type { QueueTrack, TrackSource } from './types'

/**
 * Convert DemoTrack to QueueTrack for WASM playback
 */
export function toQueueTrack(track: DemoTrack, source: TrackSource = { type: 'single' }): QueueTrack {
  return {
    id: track.id,
    path: track.path,
    title: track.title,
    artist: track.artist,
    album: track.album,
    duration_secs: track.duration,  // WASM expects duration_secs
    track_number: track.trackNumber,
    source,
    coverUrl: track.coverUrl,
  }
}

/**
 * Convert multiple DemoTracks to QueueTracks with a shared source
 */
export function tracksToQueue(tracks: DemoTrack[], source: TrackSource = { type: 'single' }): QueueTrack[] {
  return tracks.map(track => toQueueTrack(track, source))
}
