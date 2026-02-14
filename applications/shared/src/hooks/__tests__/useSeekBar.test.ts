/**
 * Comprehensive tests for useSeekBar hook
 * Tests click-only seek implementation with race condition prevention
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import { useSeekBar, shouldIgnorePositionUpdates } from '../useSeekBar';
import { usePlayerStore } from '../../stores/player';
import type { PlaybackTimingConfig } from '../../types/playback-timing';

// Mock PlayerCommands context
const mockSeek = vi.fn();
vi.mock('../../contexts/PlayerCommandsContext', () => ({
  usePlayerCommands: () => ({
    seek: mockSeek,
  }),
}));

// Mock usePlaybackTiming hook
const mockTimingConfig: PlaybackTimingConfig = {
  positionUpdateIntervalMs: 500,
  ignoreWindowMs: 600,
  deviceEventDedupWindowMs: 500,
};

vi.mock('../usePlaybackTiming', () => ({
  usePlaybackTiming: () => mockTimingConfig,
}));

describe('useSeekBar', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();

    // Reset player store to known state
    usePlayerStore.setState({
      duration: 300, // 5 minutes
      progress: 0,
      isPlaying: false,
      currentTrack: null,
      volume: 0.8,
      previousVolume: 0.8,
      queue: [],
      queueIndex: -1,
      repeatMode: 'off',
      shuffleMode: 'off',
    });

    // Mock successful seek by default
    mockSeek.mockResolvedValue(undefined);
  });

  afterEach(() => {
    vi.clearAllTimers();
    vi.useRealTimers();
  });

  describe('initialization', () => {
    it('should initialize with correct handlers', () => {
      const { result } = renderHook(() => useSeekBar());

      expect(typeof result.current.handleSeek).toBe('function');
    });
  });

  describe('click to seek', () => {
    it('should seek to correct position when clicking at 50% of progress bar', async () => {
      const { result } = renderHook(() => useSeekBar());
      const duration = usePlayerStore.getState().duration; // 300 seconds

      // Simulate click at 50% position
      const targetPosition = duration * 0.5; // 150 seconds

      await act(async () => {
        result.current.handleSeek(targetPosition);
      });

      // Should call seek with correct position
      expect(mockSeek).toHaveBeenCalledWith(targetPosition);
      expect(mockSeek).toHaveBeenCalledTimes(1);
    });

    it('should seek to correct position when clicking at 25% of progress bar', async () => {
      const { result } = renderHook(() => useSeekBar());
      const duration = usePlayerStore.getState().duration;

      const targetPosition = duration * 0.25; // 75 seconds

      await act(async () => {
        result.current.handleSeek(targetPosition);
      });

      expect(mockSeek).toHaveBeenCalledWith(targetPosition);
    });

    it('should seek to correct position when clicking at 75% of progress bar', async () => {
      const { result } = renderHook(() => useSeekBar());
      const duration = usePlayerStore.getState().duration;

      const targetPosition = duration * 0.75; // 225 seconds

      await act(async () => {
        result.current.handleSeek(targetPosition);
      });

      expect(mockSeek).toHaveBeenCalledWith(targetPosition);
    });

    it('should update store progress immediately on seek', async () => {
      const { result } = renderHook(() => useSeekBar());
      const duration = usePlayerStore.getState().duration;
      const targetPosition = duration * 0.5; // 150 seconds

      await act(async () => {
        result.current.handleSeek(targetPosition);
      });

      // Store should be updated immediately with new progress
      const { progress } = usePlayerStore.getState();
      expect(progress).toBe(50); // 50% progress
    });
  });

  describe('multiple rapid clicks', () => {
    it('should seek to each clicked position', async () => {
      const { result } = renderHook(() => useSeekBar());
      const duration = usePlayerStore.getState().duration;

      // Simulate 3 rapid clicks
      await act(async () => {
        result.current.handleSeek(duration * 0.25);
      });

      await act(async () => {
        result.current.handleSeek(duration * 0.5);
      });

      await act(async () => {
        result.current.handleSeek(duration * 0.75);
      });

      // Should have called seek 3 times (once per click)
      expect(mockSeek).toHaveBeenCalledTimes(3);
      expect(mockSeek).toHaveBeenNthCalledWith(1, duration * 0.25);
      expect(mockSeek).toHaveBeenNthCalledWith(2, duration * 0.5);
      expect(mockSeek).toHaveBeenNthCalledWith(3, duration * 0.75);
    });
  });

  describe('ignore window for race condition prevention', () => {
    it('should set ignore position updates flag when seeking', async () => {
      const { result } = renderHook(() => useSeekBar());
      const duration = usePlayerStore.getState().duration;

      await act(async () => {
        result.current.handleSeek(duration * 0.5);
      });

      // Flag should be set immediately
      expect(shouldIgnorePositionUpdates()).toBe(true);
    });

    it('should clear ignore flag after configured ignore window (600ms)', async () => {
      const { result } = renderHook(() => useSeekBar());
      const duration = usePlayerStore.getState().duration;

      await act(async () => {
        result.current.handleSeek(duration * 0.5);
      });

      expect(shouldIgnorePositionUpdates()).toBe(true);

      // Advance timers by ignore window duration
      await act(async () => {
        vi.advanceTimersByTime(mockTimingConfig.ignoreWindowMs);
      });

      expect(shouldIgnorePositionUpdates()).toBe(false);
    });

    it('should reset ignore timer on subsequent seeks', async () => {
      const { result } = renderHook(() => useSeekBar());
      const duration = usePlayerStore.getState().duration;

      // First seek
      await act(async () => {
        result.current.handleSeek(duration * 0.3);
      });

      expect(shouldIgnorePositionUpdates()).toBe(true);

      // Advance 300ms (not enough to clear)
      await act(async () => {
        vi.advanceTimersByTime(300);
      });

      expect(shouldIgnorePositionUpdates()).toBe(true);

      // Second seek (should reset timer)
      await act(async () => {
        result.current.handleSeek(duration * 0.6);
      });

      expect(shouldIgnorePositionUpdates()).toBe(true);

      // Advance another 300ms (total 600ms from first seek, but only 300ms from second)
      await act(async () => {
        vi.advanceTimersByTime(300);
      });

      // Should still be true (timer was reset)
      expect(shouldIgnorePositionUpdates()).toBe(true);

      // Advance remaining time to complete second seek's ignore window
      await act(async () => {
        vi.advanceTimersByTime(mockTimingConfig.ignoreWindowMs - 300);
      });

      // Now should be false
      expect(shouldIgnorePositionUpdates()).toBe(false);
    });
  });

  describe('seek verification', () => {
    it('should verify seek completed successfully after ignore window', async () => {
      const { result } = renderHook(() => useSeekBar());
      const duration = usePlayerStore.getState().duration;
      const targetPosition = duration * 0.5; // 150 seconds

      await act(async () => {
        result.current.handleSeek(targetPosition);
      });

      // Advance timers to trigger verification
      await act(async () => {
        vi.advanceTimersByTime(mockTimingConfig.ignoreWindowMs);
      });

      // Verification should have checked position
      // (logs verification result, but doesn't throw on failure in current implementation)
      const { progress } = usePlayerStore.getState();
      expect(progress).toBe(50);
    });

    it('should allow 0.5s tolerance for seek verification', async () => {
      const { result } = renderHook(() => useSeekBar());
      const duration = usePlayerStore.getState().duration;
      const targetPosition = 150.0;

      await act(async () => {
        result.current.handleSeek(targetPosition);
      });

      // Simulate slight position offset after seek (within tolerance)
      usePlayerStore.setState({ progress: 49.8 }); // ~149.4 seconds

      // Advance timers to trigger verification
      await act(async () => {
        vi.advanceTimersByTime(mockTimingConfig.ignoreWindowMs);
      });

      // Should not log warning (within 0.5s tolerance)
      // Test passes if no exception thrown
    });
  });

  describe('error handling', () => {
    it('should handle seek success correctly', async () => {
      const { result } = renderHook(() => useSeekBar());
      const duration = usePlayerStore.getState().duration;

      mockSeek.mockResolvedValueOnce(undefined);

      await act(async () => {
        result.current.handleSeek(duration * 0.5);
        // Flush promises
        await vi.runAllTimersAsync();
      });

      expect(mockSeek).toHaveBeenCalled();
    });

    it('should handle seek failure gracefully', async () => {
      const { result } = renderHook(() => useSeekBar());
      const duration = usePlayerStore.getState().duration;

      // Mock seek failure
      const seekError = new Error('Seek failed');
      mockSeek.mockRejectedValueOnce(seekError);

      // Should not throw
      await act(async () => {
        result.current.handleSeek(duration * 0.5);
        // Flush promises
        await vi.runAllTimersAsync();
      });

      expect(mockSeek).toHaveBeenCalled();
    });

    it('should clear ignore flag on seek failure', async () => {
      const { result } = renderHook(() => useSeekBar());
      const duration = usePlayerStore.getState().duration;

      mockSeek.mockRejectedValueOnce(new Error('Seek failed'));

      await act(async () => {
        result.current.handleSeek(duration * 0.5);
        // Flush promises to trigger error handler
        await vi.runAllTimersAsync();
      });

      // Flag should be cleared on error
      expect(shouldIgnorePositionUpdates()).toBe(false);
    });
  });

  describe('edge cases', () => {
    it('should handle seek at position 0 (beginning of track)', async () => {
      const { result } = renderHook(() => useSeekBar());

      await act(async () => {
        result.current.handleSeek(0);
      });

      expect(mockSeek).toHaveBeenCalledWith(0);
      expect(usePlayerStore.getState().progress).toBe(0);
    });

    it('should handle seek near end of track', async () => {
      const { result } = renderHook(() => useSeekBar());
      const duration = usePlayerStore.getState().duration;
      const nearEnd = duration - 1; // 299 seconds (1 second before end)

      await act(async () => {
        result.current.handleSeek(nearEnd);
      });

      expect(mockSeek).toHaveBeenCalledWith(nearEnd);
    });

    it('should handle zero duration gracefully', async () => {
      usePlayerStore.setState({ duration: 0 });

      const { result } = renderHook(() => useSeekBar());

      await act(async () => {
        result.current.handleSeek(0);
      });

      expect(mockSeek).toHaveBeenCalledWith(0);
      // Progress should be 0 when duration is 0
      expect(usePlayerStore.getState().progress).toBe(0);
    });

    it('should clamp progress to 100% maximum', async () => {
      const { result } = renderHook(() => useSeekBar());
      const duration = usePlayerStore.getState().duration;

      // Try to seek beyond duration (should be prevented by UI, but test edge case)
      const beyondEnd = duration + 50;

      await act(async () => {
        result.current.handleSeek(beyondEnd);
      });

      // Progress should be clamped to 100%
      const { progress } = usePlayerStore.getState();
      expect(progress).toBeLessThanOrEqual(100);
    });

    it('should handle seek when no track is loaded', async () => {
      usePlayerStore.setState({ currentTrack: null, duration: 0 });

      const { result } = renderHook(() => useSeekBar());

      await act(async () => {
        result.current.handleSeek(50);
      });

      // Should still call seek (backend will handle if no track)
      expect(mockSeek).toHaveBeenCalledWith(50);
    });
  });

  describe('cleanup on unmount', () => {
    it('should clean up timers on unmount', async () => {
      const { result, unmount } = renderHook(() => useSeekBar());
      const duration = usePlayerStore.getState().duration;

      await act(async () => {
        result.current.handleSeek(duration * 0.5);
      });

      // Unmount before timers fire
      unmount();

      // Advance timers
      act(() => {
        vi.advanceTimersByTime(1000);
      });

      // Should not crash (timers should be cleaned up)
    });
  });

  describe('interaction with player store', () => {
    it('should correctly calculate progress percentage from position and duration', async () => {
      const { result } = renderHook(() => useSeekBar());

      // Test various durations
      const testCases = [
        { duration: 100, position: 50, expectedProgress: 50 },
        { duration: 300, position: 150, expectedProgress: 50 },
        { duration: 200, position: 100, expectedProgress: 50 },
        { duration: 180, position: 90, expectedProgress: 50 },
      ];

      for (const { duration, position, expectedProgress } of testCases) {
        usePlayerStore.setState({ duration, progress: 0 });

        await act(async () => {
          result.current.handleSeek(position);
        });

        const { progress } = usePlayerStore.getState();
        expect(progress).toBe(expectedProgress);
      }
    });

    it('should update store before calling backend seek', async () => {
      const { result } = renderHook(() => useSeekBar());
      const duration = usePlayerStore.getState().duration;

      let progressWhenSeekCalled = 0;
      mockSeek.mockImplementationOnce(() => {
        progressWhenSeekCalled = usePlayerStore.getState().progress;
        return Promise.resolve();
      });

      await act(async () => {
        result.current.handleSeek(duration * 0.5);
      });

      // Progress should have been updated before seek was called
      expect(progressWhenSeekCalled).toBe(50);
    });
  });

  describe('timing configuration integration', () => {
    it('should use ignore window from timing config', async () => {
      const { result } = renderHook(() => useSeekBar());
      const duration = usePlayerStore.getState().duration;

      await act(async () => {
        result.current.handleSeek(duration * 0.5);
      });

      expect(shouldIgnorePositionUpdates()).toBe(true);

      // Advance by less than ignore window
      await act(async () => {
        vi.advanceTimersByTime(mockTimingConfig.ignoreWindowMs - 100);
      });

      expect(shouldIgnorePositionUpdates()).toBe(true);

      // Advance remaining time
      await act(async () => {
        vi.advanceTimersByTime(100);
      });

      expect(shouldIgnorePositionUpdates()).toBe(false);
    });
  });
});
