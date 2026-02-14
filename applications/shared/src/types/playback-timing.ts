/**
 * Playback timing configuration
 *
 * This interface mirrors the backend PlaybackTimingConfig struct.
 * Values are synchronized between frontend and backend to prevent timing-related bugs.
 */
export interface PlaybackTimingConfig {
  /**
   * Position update interval in milliseconds
   *
   * How frequently the backend emits position-updated events during playback.
   * Default: 500ms (2 updates per second)
   */
  positionUpdateIntervalMs: number;

  /**
   * Ignore window duration in milliseconds
   *
   * After a seek operation, the frontend ignores position updates from the backend
   * for this duration to prevent race conditions where stale position updates
   * cause the progress bar to jump back before settling at the new position.
   *
   * Calculated as: positionUpdateIntervalMs * 1.2
   * Default: 600ms (when position interval is 500ms)
   */
  ignoreWindowMs: number;

  /**
   * Device event deduplication window in milliseconds
   *
   * Platform APIs can emit duplicate device events. Events of the same type
   * for the same device within this window are ignored.
   * Default: 500ms
   */
  deviceEventDedupWindowMs: number;
}

/**
 * Default timing configuration (fallback if backend fetch fails)
 */
export const DEFAULT_TIMING_CONFIG: PlaybackTimingConfig = {
  positionUpdateIntervalMs: 500,
  ignoreWindowMs: 600, // 500 * 1.2
  deviceEventDedupWindowMs: 500,
};
