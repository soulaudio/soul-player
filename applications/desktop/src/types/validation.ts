/**
 * Zod schemas for runtime validation of Tauri invoke responses
 * Provides type safety by validating Rust responses match TypeScript types
 *
 * Usage:
 * - Use `invokeValidated()` for single object responses
 * - Use `invokeValidatedArray()` for array responses
 * - Validation errors throw ZodError with detailed mismatch information
 *
 * Example:
 * ```typescript
 * // Single object
 * const session = await invokeValidated(
 *   'restore_playback_session',
 *   PlaybackSessionSchema.nullable()
 * );
 *
 * // Array of objects
 * const tracks = await invokeValidatedArray(
 *   'get_tracks_by_ids',
 *   BackendTrackSchema.nullable(),
 *   { trackIds: [1, 2, 3] }
 * );
 * ```
 */

import { z } from 'zod';
import { invoke } from '@tauri-apps/api/core';

// =============================================================================
// Core Backend Types
// =============================================================================

export const BackendTrackSchema = z.object({
  id: z.number(),
  title: z.string(),
  artist_name: z.string().optional(),
  album_title: z.string().optional(),
  album_id: z.number().optional(),
  artist_id: z.number().optional(),
  duration_seconds: z.number().optional(),
  file_path: z.string().optional(),
  track_number: z.number().optional(),
  year: z.number().optional(),
  file_format: z.string().optional(),
  bit_rate: z.number().optional(),
  sample_rate: z.number().optional(),
  channels: z.number().optional(),
  source_type: z.string().optional(),
  source_name: z.string().optional(),
  source_online: z.boolean().optional(),
  cover_art_path: z.string().optional(),
  is_in_managed_library: z.boolean().optional(),
});

export const BackendAlbumSchema = z.object({
  id: z.number(),
  title: z.string(),
  artist_id: z.number().optional(),
  artist_name: z.string().optional(),
  year: z.number().optional(),
  track_count: z.number().optional(),
  cover_art_path: z.string().optional(),
});

export const BackendArtistSchema = z.object({
  id: z.number(),
  name: z.string(),
  sort_name: z.string().optional(),
  track_count: z.number(),
  album_count: z.number(),
  cover_art_path: z.string().optional(),
});

export const BackendPlaylistSchema = z.object({
  id: z.string(),
  name: z.string(),
  description: z.string().optional(),
  owner_id: z.number(),
  is_public: z.boolean(),
  is_favorite: z.boolean(),
  track_count: z.number(),
  created_at: z.string(),
  updated_at: z.string(),
  cover_art_path: z.string().optional(),
});

export const BackendGenreSchema = z.object({
  id: z.number(),
  name: z.string(),
  track_count: z.number(),
});

// =============================================================================
// Playback Session Types
// =============================================================================

export const PlaybackSessionSchema = z.object({
  current_track_id: z.number().nullable(),
  queue_track_ids: z.array(z.number()),
  queue_index: z.number(),
  position_seconds: z.number(),
  volume: z.number(),
  repeat_mode: z.string(),
  shuffle_mode: z.string(),
  context_type: z.string().nullable(),
  context_id: z.string().nullable(),
  was_playing: z.boolean(),
});

export const QueueTrackSchema = z.object({
  trackId: z.string(),
  title: z.string(),
  artist: z.string(),
  album: z.string().nullable(),
  filePath: z.string(),
  durationSeconds: z.number().nullable(),
  trackNumber: z.number().nullable(),
});

export const PlaybackContextSchema = z.object({
  id: z.number(),
  context_type: z.string(),
  context_id: z.string().nullable(),
  context_name: z.string().nullable(),
  context_artwork_path: z.string().nullable(),
  last_played_at: z.number(),
});

// =============================================================================
// Audio Settings Types
// =============================================================================

export const EffectSlotSchema = z.object({
  index: z.number(),
  effect: z.any().nullable(), // Complex discriminated union - skip for now
  enabled: z.boolean(),
});

export const HeadroomSettingsSchema = z.object({
  mode: z.string(),
  manual_db: z.number().optional(),
  enabled: z.boolean(),
});

export const LatencyInfoSchema = z.object({
  buffer_size: z.number(),
  sample_rate: z.number(),
  latency_ms: z.number(),
});

export const ExclusiveConfigSchema = z.object({
  buffer_size: z.number(),
});

export const AnalysisQueueStatsSchema = z.object({
  pending: z.number(),
  in_progress: z.number(),
  completed: z.number(),
  failed: z.number(),
});

export const AnalysisWorkerStatusSchema = z.object({
  is_running: z.boolean(),
  current_track_id: z.number().nullable(),
  current_track_title: z.string().nullable(),
});

export const DatabaseHealthSchema = z.object({
  total_tracks: z.number(),
  tracks_with_availability: z.number(),
  tracks_with_local_files: z.number(),
  issues: z.array(z.string()),
});

// =============================================================================
// Validation Helper
// =============================================================================

/**
 * Type-safe Tauri invoke with Zod validation
 * Throws ZodError if response doesn't match schema
 *
 * @example
 * const session = await invokeValidated(
 *   'restore_playback_session',
 *   PlaybackSessionSchema.nullable()
 * );
 */
export async function invokeValidated<T>(
  command: string,
  schema: z.ZodSchema<T>,
  args?: Record<string, unknown>
): Promise<T> {
  const result = await invoke(command, args);
  return schema.parse(result);
}

/**
 * Type-safe Tauri invoke with Zod validation (array responses)
 *
 * @example
 * const tracks = await invokeValidatedArray(
 *   'get_all_tracks',
 *   BackendTrackSchema
 * );
 */
export async function invokeValidatedArray<T>(
  command: string,
  itemSchema: z.ZodSchema<T>,
  args?: Record<string, unknown>
): Promise<T[]> {
  const result = await invoke(command, args);
  return z.array(itemSchema).parse(result);
}
