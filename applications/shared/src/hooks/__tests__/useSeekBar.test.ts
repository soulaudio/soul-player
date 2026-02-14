/**
 * Simplified tests for useSeekBar hook
 * Tests click-only seek implementation
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useSeekBar } from '../useSeekBar';
import { usePlayerStore } from '../../stores/player';

// Mock PlayerCommands context
const mockSeek = vi.fn();
vi.mock('../../contexts/PlayerCommandsContext', () => ({
  usePlayerCommands: () => ({
    seek: mockSeek,
  }),
}));

describe('useSeekBar', () => {
  beforeEach(() => {
    vi.clearAllMocks();

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

  describe('initialization', () => {
    it('should initialize with handleSeek function', () => {
      const { result } = renderHook(() => useSeekBar());

      expect(result.current.handleSeek).toBeDefined();
      expect(typeof result.current.handleSeek).toBe('function');
    });
  });

  describe('basic seeking', () => {
    it('should seek to 25% (75 seconds)', async () => {
      const { result } = renderHook(() => useSeekBar());

      await act(async () => {
        result.current.handleSeek(75);
      });

      expect(mockSeek).toHaveBeenCalledWith(75);
      const { progress } = usePlayerStore.getState();
      expect(progress).toBe(25);
    });

    it('should seek to 50% (150 seconds)', async () => {
      const { result } = renderHook(() => useSeekBar());

      await act(async () => {
        result.current.handleSeek(150);
      });

      expect(mockSeek).toHaveBeenCalledWith(150);
      const { progress } = usePlayerStore.getState();
      expect(progress).toBe(50);
    });

    it('should seek to 75% (225 seconds)', async () => {
      const { result } = renderHook(() => useSeekBar());

      await act(async () => {
        result.current.handleSeek(225);
      });

      expect(mockSeek).toHaveBeenCalledWith(225);
      const { progress } = usePlayerStore.getState();
      expect(progress).toBe(75);
    });
  });

  describe('multiple rapid seeks', () => {
    it('should handle multiple rapid seeks', async () => {
      const { result } = renderHook(() => useSeekBar());

      await act(async () => {
        result.current.handleSeek(60);
        result.current.handleSeek(120);
        result.current.handleSeek(180);
      });

      // All seeks should be sent to backend
      expect(mockSeek).toHaveBeenCalledTimes(3);
      expect(mockSeek).toHaveBeenNthCalledWith(1, 60);
      expect(mockSeek).toHaveBeenNthCalledWith(2, 120);
      expect(mockSeek).toHaveBeenNthCalledWith(3, 180);
    });
  });

  describe('position clamping', () => {
    it('should clamp position to 0 minimum', async () => {
      const { result } = renderHook(() => useSeekBar());

      await act(async () => {
        result.current.handleSeek(-10);
      });

      expect(mockSeek).toHaveBeenCalledWith(0);
    });

    it('should clamp position to (duration - 0.1s) maximum', async () => {
      const { result } = renderHook(() => useSeekBar());

      await act(async () => {
        result.current.handleSeek(305); // More than duration
      });

      expect(mockSeek).toHaveBeenCalledWith(299.9); // 300 - 0.1
    });

    it('should clamp progress to 100% maximum', async () => {
      const { result } = renderHook(() => useSeekBar());

      await act(async () => {
        result.current.handleSeek(305);
      });

      const { progress } = usePlayerStore.getState();
      expect(progress).toBeLessThanOrEqual(100);
    });
  });

  describe('error handling', () => {
    it('should handle seek failure gracefully', async () => {
      mockSeek.mockRejectedValueOnce(new Error('Seek failed'));
      const { result } = renderHook(() => useSeekBar());

      await act(async () => {
        result.current.handleSeek(150);
      });

      expect(mockSeek).toHaveBeenCalledWith(150);
      // Should not throw, error is caught internally
    });
  });

  describe('edge cases', () => {
    it('should handle zero duration', async () => {
      usePlayerStore.setState({ duration: 0 });
      const { result } = renderHook(() => useSeekBar());

      await act(async () => {
        result.current.handleSeek(50);
      });

      // Should still call seek (backend will handle appropriately)
      expect(mockSeek).toHaveBeenCalledWith(0); // Clamped to 0 since duration - 0.1 < 0
      const { progress } = usePlayerStore.getState();
      expect(progress).toBe(0);
    });

    it('should handle seek to position 0', async () => {
      const { result } = renderHook(() => useSeekBar());

      await act(async () => {
        result.current.handleSeek(0);
      });

      expect(mockSeek).toHaveBeenCalledWith(0);
      const { progress } = usePlayerStore.getState();
      expect(progress).toBe(0);
    });

    it('should handle seek near end of track', async () => {
      const { result } = renderHook(() => useSeekBar());

      await act(async () => {
        result.current.handleSeek(299.5);
      });

      expect(mockSeek).toHaveBeenCalledWith(299.5);
      const { progress } = usePlayerStore.getState();
      expect(progress).toBeCloseTo(99.83, 1);
    });
  });
});
