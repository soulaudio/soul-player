import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useInterpolatedProgress } from '../useInterpolatedProgress';
import { usePlayerStore } from '../../stores/player';

// Mock requestAnimationFrame and cancelAnimationFrame
let rafCallbacks: FrameRequestCallback[] = [];
let rafId = 0;

beforeEach(() => {
  rafCallbacks = [];
  rafId = 0;

  global.requestAnimationFrame = (callback: FrameRequestCallback) => {
    const id = ++rafId;
    rafCallbacks.push(callback);
    return id;
  };

  global.cancelAnimationFrame = (id: number) => {
    rafCallbacks = rafCallbacks.filter((_, index) => index + 1 !== id);
  };

  // Mock Date.now for consistent timing
  vi.useFakeTimers();
});

afterEach(() => {
  rafCallbacks = [];
  vi.useRealTimers();
});

const flushAnimationFrames = (count = 1) => {
  for (let i = 0; i < count; i++) {
    const callbacks = [...rafCallbacks];
    rafCallbacks = [];
    callbacks.forEach(callback => callback(performance.now()));
  }
};

describe('useInterpolatedProgress', () => {
  beforeEach(() => {
    // Reset store to initial state
    usePlayerStore.setState({
      progress: 0,
      duration: 100,
      isPlaying: false,
      currentTrack: null,
    });
  });

  it('should return initial progress from store', () => {
    usePlayerStore.setState({ progress: 25 });

    const { result } = renderHook(() => useInterpolatedProgress());

    expect(result.current.progress).toBe(25);
    expect(result.current.duration).toBe(100);
  });

  it('should interpolate progress when playing', () => {
    usePlayerStore.setState({
      progress: 0,
      duration: 100, // 100 seconds
      isPlaying: true,
      currentTrack: { id: '1', title: 'Test Track' } as any,
    });

    const { result } = renderHook(() => useInterpolatedProgress());

    // Initial progress
    expect(result.current.progress).toBe(0);

    // Advance time by 1 second (1000ms)
    act(() => {
      vi.advanceTimersByTime(1000);
      flushAnimationFrames(60); // ~60 frames in 1 second
    });

    // Should have advanced by ~1% (1 second out of 100 seconds)
    expect(result.current.progress).toBeGreaterThan(0.9);
    expect(result.current.progress).toBeLessThan(1.1);
  });

  it('should stop interpolating when paused', () => {
    usePlayerStore.setState({
      progress: 10,
      duration: 100,
      isPlaying: true,
      currentTrack: { id: '1', title: 'Test Track' } as any,
    });

    const { result } = renderHook(() => useInterpolatedProgress());

    // Advance while playing
    act(() => {
      vi.advanceTimersByTime(500);
      flushAnimationFrames(30);
    });

    const progressWhilePlaying = result.current.progress;
    expect(progressWhilePlaying).toBeGreaterThan(10);

    // Pause playback
    act(() => {
      usePlayerStore.setState({ isPlaying: false });
    });

    // Clear any queued animation frames from before the pause
    rafCallbacks = [];

    // Advance significant time while paused
    act(() => {
      vi.advanceTimersByTime(5000); // 5 seconds
      // No new animation frames should be queued since we're paused
    });

    // Progress should reset to backend value (10) when paused
    expect(result.current.progress).toBe(10);
    // Verify no new animation frames were scheduled
    expect(rafCallbacks.length).toBe(0);
  });

  it('should reset progress on track change', () => {
    usePlayerStore.setState({
      progress: 50,
      duration: 100,
      isPlaying: true,
      currentTrack: { id: '1', title: 'Track 1' } as any,
    });

    const { result } = renderHook(() => useInterpolatedProgress());
    expect(result.current.progress).toBe(50);

    // Change track
    act(() => {
      usePlayerStore.setState({
        progress: 0,
        currentTrack: { id: '2', title: 'Track 2' } as any,
      });
    });

    // Progress should reset to 0
    expect(result.current.progress).toBe(0);
  });

  it('should detect and reset on backward seek', () => {
    usePlayerStore.setState({
      progress: 50,
      duration: 100,
      isPlaying: true,
      currentTrack: { id: '1', title: 'Test Track' } as any,
    });

    const { result } = renderHook(() => useInterpolatedProgress());

    // Interpolate a bit
    act(() => {
      vi.advanceTimersByTime(500);
      flushAnimationFrames(30);
    });

    expect(result.current.progress).toBeGreaterThan(50);

    // Seek backward (backend updates progress)
    act(() => {
      usePlayerStore.setState({ progress: 20 });
    });

    // Should reset to new position
    expect(result.current.progress).toBe(20);
  });

  it('should detect and reset on forward seek', () => {
    usePlayerStore.setState({
      progress: 20,
      duration: 100,
      isPlaying: true,
      currentTrack: { id: '1', title: 'Test Track' } as any,
    });

    const { result } = renderHook(() => useInterpolatedProgress());
    expect(result.current.progress).toBe(20);

    // Seek forward (large jump)
    act(() => {
      usePlayerStore.setState({ progress: 80 });
    });

    // Should reset to new position
    expect(result.current.progress).toBe(80);
  });

  it('should not overshoot track duration', () => {
    usePlayerStore.setState({
      progress: 99,
      duration: 100,
      isPlaying: true,
      currentTrack: { id: '1', title: 'Test Track' } as any,
    });

    const { result } = renderHook(() => useInterpolatedProgress());

    // Advance time significantly
    act(() => {
      vi.advanceTimersByTime(5000); // 5 seconds
      flushAnimationFrames(300); // Many frames
    });

    // Should not exceed 100%
    expect(result.current.progress).toBeLessThanOrEqual(100);
  });

  it('should not overshoot backend position by more than 2%', () => {
    usePlayerStore.setState({
      progress: 50,
      duration: 100,
      isPlaying: true,
      currentTrack: { id: '1', title: 'Test Track' } as any,
    });

    const { result } = renderHook(() => useInterpolatedProgress());

    // Advance time by 10 seconds (should add 10% progress)
    act(() => {
      vi.advanceTimersByTime(10000);
      flushAnimationFrames(600);
    });

    // Should not exceed backend position (50%) + 2% drift allowance
    expect(result.current.progress).toBeLessThanOrEqual(52);
  });

  it('should handle zero duration gracefully', () => {
    usePlayerStore.setState({
      progress: 0,
      duration: 0,
      isPlaying: true,
      currentTrack: { id: '1', title: 'Test Track' } as any,
    });

    const { result } = renderHook(() => useInterpolatedProgress());

    act(() => {
      vi.advanceTimersByTime(1000);
      flushAnimationFrames(60);
    });

    // Should stay at 0 when duration is 0
    expect(result.current.progress).toBe(0);
  });

  it('should clean up animation frame on unmount', () => {
    usePlayerStore.setState({
      progress: 0,
      duration: 100,
      isPlaying: true,
      currentTrack: { id: '1', title: 'Test Track' } as any,
    });

    const cancelSpy = vi.spyOn(global, 'cancelAnimationFrame');
    const { unmount } = renderHook(() => useInterpolatedProgress());

    // Start animation
    act(() => {
      vi.advanceTimersByTime(100);
      flushAnimationFrames(6);
    });

    // Unmount should cancel animation
    unmount();

    expect(cancelSpy).toHaveBeenCalled();
    cancelSpy.mockRestore();
  });

  it('should sync with backend updates during playback', () => {
    usePlayerStore.setState({
      progress: 10,
      duration: 100,
      isPlaying: true,
      currentTrack: { id: '1', title: 'Test Track' } as any,
    });

    const { result } = renderHook(() => useInterpolatedProgress());

    // Interpolate for a bit
    act(() => {
      vi.advanceTimersByTime(200);
      flushAnimationFrames(12);
    });

    const interpolated = result.current.progress;
    expect(interpolated).toBeGreaterThan(10);

    // Backend sends normal update (not a seek, just regular 500ms update)
    act(() => {
      usePlayerStore.setState({ progress: 10.5 }); // Normal progression
    });

    // Should accept the update and continue from there
    // (Small difference < 0.5% threshold, so it's treated as a normal update)
    expect(result.current.progress).toBeCloseTo(10.5, 0);
  });
});
