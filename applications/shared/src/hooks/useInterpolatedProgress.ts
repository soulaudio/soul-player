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
  const { progress, duration, isPlaying, currentTrack, seekVersion, seekTarget } = usePlayerStore(state => ({
    progress: state.progress,
    duration: state.duration,
    isPlaying: state.isPlaying,
    currentTrack: state.currentTrack,
    seekVersion: state.seekVersion,
    seekTarget: state.seekTarget,
  }));

  // Interpolated progress state (in percentage 0-100)
  const [interpolatedProgress, setInterpolatedProgress] = useState(progress);

  // Track last backend update for drift detection during normal playback
  const lastBackendProgress = useRef(progress);
  const lastBackendTimestamp = useRef(Date.now());

  // Track ID to detect track changes
  const lastTrackId = useRef(currentTrack?.id);

  // Animation frame ID for cleanup
  const animationFrameRef = useRef<number | null>(null);

  // User seek protection: tracks the last seekVersion we processed and a window
  // during which stale backend position events (emitted before the seek completed)
  // should be ignored so they don't snap the bar back to the pre-seek position.
  const lastSeekVersionRef = useRef(seekVersion);
  const postSeekUntilRef = useRef<number>(0);
  // The progress value we snapped to on the user seek — used to identify stale events
  const postSeekTargetRef = useRef<number>(progress);

  useEffect(() => {
    // Detect track changes — reset immediately
    if (currentTrack?.id !== lastTrackId.current) {
      lastTrackId.current = currentTrack?.id;
      setInterpolatedProgress(0);
      lastBackendProgress.current = 0;
      lastBackendTimestamp.current = Date.now();
      return;
    }

    // --- User-initiated seek (seekVersion bumped by useSeekBar) ---
    if (seekVersion !== lastSeekVersionRef.current) {
      lastSeekVersionRef.current = seekVersion;
      const now = Date.now();
      // Use seekTarget (not progress): backend events may have already overwritten
      // progress with a stale value by the time this effect runs.
      setInterpolatedProgress(seekTarget);
      lastBackendProgress.current = seekTarget;
      lastBackendTimestamp.current = now;
      // Open a 400ms window to ignore stale backend events from before the seek
      postSeekUntilRef.current = now + 400;
      postSeekTargetRef.current = seekTarget;
      return;
    }

    // --- Backend position update ---

    // If we're inside the post-seek protection window, drop any backend update that
    // looks like a stale pre-seek position (i.e. far from where we just seeked to).
    if (Date.now() < postSeekUntilRef.current) {
      const distanceFromTarget = Math.abs(progress - postSeekTargetRef.current);
      if (distanceFromTarget > 0.5) {
        // Stale event — discard, keep interpolated at the seek target.
        // Don't update lastBackendProgress; leave it at the seek target so the
        // first real post-seek backend event has a small diff and enters normal flow.
        return;
      }
    }

    // Threshold for seek detection: 0.5% ≈ 0.5s on a 100s track
    const SEEK_THRESHOLD = 0.5;
    const progressDiff = Math.abs(progress - lastBackendProgress.current);
    const isBackendSeek = progressDiff > SEEK_THRESHOLD;

    if (isBackendSeek) {
      // Genuine backend-reported seek (e.g. loop, skip)
      setInterpolatedProgress(progress);
      lastBackendProgress.current = progress;
      lastBackendTimestamp.current = Date.now();
      return;
    }

    // Normal backend update — advance reference
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

    // Start RAF interpolation animation
    let lastFrameTime = Date.now();

    const animate = () => {
      const now = Date.now();
      const deltaMs = now - lastFrameTime;
      lastFrameTime = now;

      const advanceRate = duration > 0 ? (100 / duration) / 1000 : 0;
      const progressDelta = advanceRate * deltaMs;

      setInterpolatedProgress(current => {
        const newProgress = current + progressDelta;
        const maxProgress = Math.min(100, lastBackendProgress.current + 2);
        return Math.min(newProgress, maxProgress);
      });

      animationFrameRef.current = requestAnimationFrame(animate);
    };

    animationFrameRef.current = requestAnimationFrame(animate);

    return () => {
      if (animationFrameRef.current !== null) {
        cancelAnimationFrame(animationFrameRef.current);
        animationFrameRef.current = null;
      }
    };
  }, [progress, duration, isPlaying, currentTrack?.id, seekVersion, seekTarget]);

  // Eagerly use seekTarget on the render where a user seek is detected.
  // useEffect runs *after* paint, so interpolatedProgress is still the old value for
  // one frame. We detect the seek at render time (lastSeekVersionRef not yet updated)
  // and return seekTarget — which backend events cannot overwrite — instead of progress,
  // which may already have been clobbered by a stale backend position event.
  const isUserSeekThisRender = seekVersion !== lastSeekVersionRef.current;

  return {
    progress: isUserSeekThisRender ? seekTarget : interpolatedProgress,
    duration,
  };
}
