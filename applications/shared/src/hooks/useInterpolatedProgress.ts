import { useState, useEffect, useRef } from 'react';
import { usePlayerStore } from '../stores/player';

/**
 * Hook that provides smoothly interpolated progress between backend updates.
 *
 * The backend emits position updates every 500ms, which causes the progress bar to jump.
 * This hook uses requestAnimationFrame to interpolate progress smoothly at 60fps between
 * those backend updates, advancing at the actual playback rate (1 second per second).
 *
 * Features:
 * - Smooth 60fps animation between backend updates
 * - Automatically pauses when playback is paused
 * - Resets on track changes
 * - Detects and resets on seeks (backward or forward jumps)
 * - Prevents overshooting track duration
 * - No memory leaks (proper cleanup on unmount)
 *
 * @returns Interpolated progress percentage (0-100) and duration in seconds
 */
export function useInterpolatedProgress() {
  const { progress, duration, isPlaying, currentTrack } = usePlayerStore(state => ({
    progress: state.progress,
    duration: state.duration,
    isPlaying: state.isPlaying,
    currentTrack: state.currentTrack,
  }));

  // Interpolated progress state (in percentage 0-100)
  const [interpolatedProgress, setInterpolatedProgress] = useState(progress);

  // Track last backend update to detect seeks
  const lastBackendProgress = useRef(progress);
  const lastBackendTimestamp = useRef(Date.now());

  // Track ID to detect track changes
  const lastTrackId = useRef(currentTrack?.id);

  // Animation frame ID for cleanup
  const animationFrameRef = useRef<number | null>(null);

  useEffect(() => {
    // Detect track changes - reset immediately
    if (currentTrack?.id !== lastTrackId.current) {
      lastTrackId.current = currentTrack?.id;
      setInterpolatedProgress(0);
      lastBackendProgress.current = 0;
      lastBackendTimestamp.current = Date.now();
      return;
    }

    // Calculate progress difference to detect seeks
    const progressDiff = Math.abs(progress - lastBackendProgress.current);

    // Threshold for seek detection: 0.5% = ~0.5 seconds on 100-second track
    // This is larger than normal interpolation drift but smaller than typical seeks
    const SEEK_THRESHOLD = 0.5;

    // Detect seeks: sudden jumps in progress (backward or forward)
    const isSeek = progressDiff > SEEK_THRESHOLD;

    if (isSeek) {
      // Reset to new position immediately on seek
      setInterpolatedProgress(progress);
      lastBackendProgress.current = progress;
      lastBackendTimestamp.current = Date.now();
      return;
    }

    // Update last backend values
    lastBackendProgress.current = progress;
    lastBackendTimestamp.current = Date.now();

    // Stop interpolation if paused or no duration
    if (!isPlaying || duration <= 0) {
      setInterpolatedProgress(progress);
      if (animationFrameRef.current !== null) {
        cancelAnimationFrame(animationFrameRef.current);
        animationFrameRef.current = null;
      }
      return;
    }

    // Start interpolation animation
    let lastFrameTime = Date.now();

    const animate = () => {
      const now = Date.now();
      const deltaMs = now - lastFrameTime;
      lastFrameTime = now;

      // Calculate how much progress should advance per millisecond
      // Progress is 0-100%, duration is in seconds
      // Advance rate: (100% / duration_in_seconds) / 1000ms = percent per millisecond
      const advanceRate = duration > 0 ? (100 / duration) / 1000 : 0;
      const progressDelta = advanceRate * deltaMs;

      setInterpolatedProgress(current => {
        const newProgress = current + progressDelta;

        // Clamp to prevent overshooting
        // Don't exceed 100% or the backend's last known position + reasonable drift
        const maxProgress = Math.min(100, lastBackendProgress.current + 2); // Allow 2% drift
        return Math.min(newProgress, maxProgress);
      });

      // Continue animation
      animationFrameRef.current = requestAnimationFrame(animate);
    };

    // Start animation
    animationFrameRef.current = requestAnimationFrame(animate);

    // Cleanup on unmount or dependency change
    return () => {
      if (animationFrameRef.current !== null) {
        cancelAnimationFrame(animationFrameRef.current);
        animationFrameRef.current = null;
      }
    };
  }, [progress, duration, isPlaying, currentTrack?.id]);

  return {
    progress: interpolatedProgress,
    duration,
  };
}
