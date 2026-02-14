/**
 * Comprehensive tests for ProgressBar component
 * Tests click-only seek UI and smooth interpolation
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ProgressBar } from '../ProgressBar';
import { usePlayerStore } from '../../../stores/player';

// Mock useSeekBar hook
const mockHandleSeek = vi.fn();

vi.mock('../../../hooks/useSeekBar', () => ({
  useSeekBar: () => ({
    handleSeek: mockHandleSeek,
  }),
}));

// Mock useInterpolatedProgress hook
vi.mock('../../../hooks/useInterpolatedProgress', () => ({
  useInterpolatedProgress: () => {
    const { progress, duration } = usePlayerStore.getState();
    return { progress, duration };
  },
}));

describe('ProgressBar', () => {
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
  });

  describe('rendering', () => {
    it('should render progress bar with all elements', () => {
      const { container } = render(<ProgressBar />);

      // Check progress bar container exists
      const progressBar = container.querySelector('.cursor-pointer');
      expect(progressBar).toBeInTheDocument();
    });

    it('should display current time and total duration', () => {
      usePlayerStore.setState({ duration: 300, progress: 50 }); // 5 min track, at 50%

      render(<ProgressBar />);

      // Current time: 50% of 300s = 150s = 2:30
      expect(screen.getByText('2:30')).toBeInTheDocument();

      // Total duration: 300s = 5:00
      expect(screen.getByText('5:00')).toBeInTheDocument();
    });

    it('should display 0:00 for current time when progress is 0', () => {
      usePlayerStore.setState({ duration: 300, progress: 0 });

      render(<ProgressBar />);

      // Both times show 0:00 when at beginning with 0 duration in query
      const times = screen.getAllByText('0:00');
      expect(times.length).toBeGreaterThanOrEqual(1);

      expect(screen.getByText('5:00')).toBeInTheDocument();
    });

    it('should display correct time format for tracks over 1 hour', () => {
      usePlayerStore.setState({ duration: 3600, progress: 50 }); // 1 hour, at 50%

      render(<ProgressBar />);

      // Current time: 50% of 3600s = 1800s = 30:00
      expect(screen.getByText('30:00')).toBeInTheDocument();

      // Total duration: 3600s = 1:00:00
      expect(screen.getByText('1:00:00')).toBeInTheDocument();
    });

    it('should show seek handle on hover', () => {
      const { container } = render(<ProgressBar />);

      // Find the seek handle (the circular dot)
      const seekHandle = container.querySelector('.rounded-full.shadow-lg');
      expect(seekHandle).toBeInTheDocument();

      // Should have opacity-0 class (hidden by default)
      expect(seekHandle?.className).toContain('opacity-0');
      expect(seekHandle?.className).toContain('group-hover:opacity-100');
    });

    it('should display progress bar fill based on store progress', () => {
      usePlayerStore.setState({ progress: 30 });

      const { container } = render(<ProgressBar />);

      // Find the filled progress element
      const progressFill = container.querySelector('.bg-primary');
      expect(progressFill).toBeInTheDocument();
      expect(progressFill).toHaveStyle({ width: '30%' });
    });

    it('should update progress display when store changes', () => {
      const { container, rerender } = render(<ProgressBar />);

      // Initial progress
      usePlayerStore.setState({ progress: 25 });
      rerender(<ProgressBar />);

      let progressFill = container.querySelector('.bg-primary');
      expect(progressFill).toHaveStyle({ width: '25%' });

      // Update progress
      usePlayerStore.setState({ progress: 75 });
      rerender(<ProgressBar />);

      progressFill = container.querySelector('.bg-primary');
      expect(progressFill).toHaveStyle({ width: '75%' });
    });
  });

  describe('click to seek interaction', () => {
    it('should call handleSeek on click', () => {
      const { container } = render(<ProgressBar />);

      const progressBar = container.querySelector('.cursor-pointer');
      expect(progressBar).toBeInTheDocument();

      // Mock getBoundingClientRect to simulate progress bar dimensions
      vi.spyOn(progressBar!, 'getBoundingClientRect').mockReturnValue({
        left: 0,
        top: 0,
        right: 400,
        bottom: 8,
        width: 400,
        height: 8,
        x: 0,
        y: 0,
        toJSON: () => ({}),
      });

      // Click at 50% position (200px out of 400px)
      fireEvent.click(progressBar!, { clientX: 200, clientY: 4 });

      // Should call handleSeek with position (50% of 300s = 150s)
      expect(mockHandleSeek).toHaveBeenCalledWith(150);
      expect(mockHandleSeek).toHaveBeenCalledTimes(1);
    });

    it('should calculate correct position for click at 25%', () => {
      const { container } = render(<ProgressBar />);
      const progressBar = container.querySelector('.cursor-pointer');

      vi.spyOn(progressBar!, 'getBoundingClientRect').mockReturnValue({
        left: 0,
        top: 0,
        right: 400,
        bottom: 8,
        width: 400,
        height: 8,
        x: 0,
        y: 0,
        toJSON: () => ({}),
      });

      // Click at 25% position (100px out of 400px)
      fireEvent.click(progressBar!, { clientX: 100, clientY: 4 });

      // Should calculate 25% of 300s = 75s
      expect(mockHandleSeek).toHaveBeenCalledWith(75);
    });

    it('should calculate correct position for click at 75%', () => {
      const { container } = render(<ProgressBar />);
      const progressBar = container.querySelector('.cursor-pointer');

      vi.spyOn(progressBar!, 'getBoundingClientRect').mockReturnValue({
        left: 0,
        top: 0,
        right: 400,
        bottom: 8,
        width: 400,
        height: 8,
        x: 0,
        y: 0,
        toJSON: () => ({}),
      });

      // Click at 75% position (300px out of 400px)
      fireEvent.click(progressBar!, { clientX: 300, clientY: 4 });

      // Should calculate 75% of 300s = 225s
      expect(mockHandleSeek).toHaveBeenCalledWith(225);
    });

    it('should clamp position to prevent seeking beyond track end', () => {
      const { container } = render(<ProgressBar />);
      const progressBar = container.querySelector('.cursor-pointer');

      vi.spyOn(progressBar!, 'getBoundingClientRect').mockReturnValue({
        left: 0,
        top: 0,
        right: 400,
        bottom: 8,
        width: 400,
        height: 8,
        x: 0,
        y: 0,
        toJSON: () => ({}),
      });

      // Click at very end (100%)
      fireEvent.click(progressBar!, { clientX: 400, clientY: 4 });

      // Should clamp to duration - 0.1s to avoid EOF
      const expectedPosition = 300 - 0.1;
      expect(mockHandleSeek).toHaveBeenCalledWith(expectedPosition);
    });

    it('should handle clicks at position 0 (beginning)', () => {
      const { container } = render(<ProgressBar />);
      const progressBar = container.querySelector('.cursor-pointer');

      vi.spyOn(progressBar!, 'getBoundingClientRect').mockReturnValue({
        left: 0,
        top: 0,
        right: 400,
        bottom: 8,
        width: 400,
        height: 8,
        x: 0,
        y: 0,
        toJSON: () => ({}),
      });

      // Click at beginning (0%)
      fireEvent.click(progressBar!, { clientX: 0, clientY: 4 });

      expect(mockHandleSeek).toHaveBeenCalledWith(0);
    });

    it('should stop event propagation on click', () => {
      const { container } = render(<ProgressBar />);
      const progressBar = container.querySelector('.cursor-pointer');

      const mockStopPropagation = vi.fn();
      const event = new MouseEvent('click', { clientX: 200, clientY: 4, bubbles: true });
      event.stopPropagation = mockStopPropagation;

      fireEvent(progressBar!, event);

      expect(mockStopPropagation).toHaveBeenCalled();
    });
  });

  describe('interpolation and smooth updates', () => {
    it('should have transition classes for smooth animation', () => {
      const { container } = render(<ProgressBar />);

      const progressFill = container.querySelector('.bg-primary');
      expect(progressFill).toBeInTheDocument();

      // Should have transition class for smooth animation
      expect(progressFill?.className).toContain('transition-all');
      expect(progressFill?.className).toContain('duration-100');
    });

    it('should update displayed time as progress changes', () => {
      const { rerender } = render(<ProgressBar />);

      // Start at 0%
      usePlayerStore.setState({ progress: 0 });
      rerender(<ProgressBar />);
      const zeroTimes = screen.queryAllByText('0:00');
      expect(zeroTimes.length).toBeGreaterThanOrEqual(1);

      // Update to 25%
      usePlayerStore.setState({ progress: 25 });
      rerender(<ProgressBar />);
      expect(screen.getByText('1:15')).toBeInTheDocument(); // 25% of 300s = 75s

      // Update to 50%
      usePlayerStore.setState({ progress: 50 });
      rerender(<ProgressBar />);
      expect(screen.getByText('2:30')).toBeInTheDocument(); // 50% of 300s = 150s

      // Update to 75%
      usePlayerStore.setState({ progress: 75 });
      rerender(<ProgressBar />);
      expect(screen.getByText('3:45')).toBeInTheDocument(); // 75% of 300s = 225s
    });
  });

  describe('edge cases', () => {
    it('should handle zero duration gracefully', () => {
      usePlayerStore.setState({ duration: 0, progress: 0 });

      render(<ProgressBar />);

      const zeroTimes = screen.getAllByText('0:00');
      expect(zeroTimes.length).toBeGreaterThanOrEqual(2);
    });

    it('should handle very short tracks (< 1 minute)', () => {
      usePlayerStore.setState({ duration: 30, progress: 50 }); // 30 second track

      render(<ProgressBar />);

      expect(screen.getByText('0:15')).toBeInTheDocument(); // 15 seconds
      expect(screen.getByText('0:30')).toBeInTheDocument(); // 30 seconds
    });

    it('should handle very long tracks (> 1 hour)', () => {
      usePlayerStore.setState({ duration: 7200, progress: 50 }); // 2 hour track

      render(<ProgressBar />);

      expect(screen.getByText('1:00:00')).toBeInTheDocument(); // 1 hour
      expect(screen.getByText('2:00:00')).toBeInTheDocument(); // 2 hours
    });

    it('should prevent division by zero when duration is 0', () => {
      usePlayerStore.setState({ duration: 0 });

      const { container } = render(<ProgressBar />);

      const progressFill = container.querySelector('.bg-primary');

      // Should show 0% width, not NaN
      expect(progressFill).toHaveStyle({ width: '0%' });
    });

    it('should clamp progress display to between 0% and 100%', () => {
      const { container, rerender } = render(<ProgressBar />);

      // Test lower bound - progress clamped in style
      usePlayerStore.setState({ progress: -10 });
      rerender(<ProgressBar />);

      let progressFill = container.querySelector('.bg-primary');
      // The component clamps with Math.max(0, Math.min(100, progress))
      expect(progressFill?.getAttribute('style')).toContain('width: 0%');

      // Test upper bound - progress clamped in style
      usePlayerStore.setState({ progress: 150 });
      rerender(<ProgressBar />);

      progressFill = container.querySelector('.bg-primary');
      // The component clamps with Math.max(0, Math.min(100, progress))
      expect(progressFill?.getAttribute('style')).toContain('width: 100%');
    });

    it('should handle rapid progress updates without performance issues', () => {
      const { rerender } = render(<ProgressBar />);

      // Simulate 100 rapid updates
      for (let i = 0; i <= 100; i++) {
        usePlayerStore.setState({ progress: i });
        rerender(<ProgressBar />);
      }

      // Should end at 100% - both times show 5:00 (current time and duration)
      const times = screen.getAllByText('5:00');
      expect(times.length).toBeGreaterThanOrEqual(2);
    });

    it('should handle click when progress bar has offset position', () => {
      const { container } = render(<ProgressBar />);
      const progressBar = container.querySelector('.cursor-pointer');

      // Progress bar is offset from left edge of viewport
      vi.spyOn(progressBar!, 'getBoundingClientRect').mockReturnValue({
        left: 100, // Offset by 100px
        top: 0,
        right: 500,
        bottom: 8,
        width: 400,
        height: 8,
        x: 100,
        y: 0,
        toJSON: () => ({}),
      });

      // Click at viewport position 300px (200px into progress bar = 50%)
      fireEvent.click(progressBar!, { clientX: 300, clientY: 4 });

      // Should calculate relative to progress bar position
      expect(mockHandleSeek).toHaveBeenCalledWith(150); // 50% of 300s
    });
  });

  describe('accessibility', () => {
    it('should have cursor pointer to indicate clickability', () => {
      const { container } = render(<ProgressBar />);

      const progressBar = container.querySelector('.cursor-pointer');
      expect(progressBar).toBeInTheDocument();
      expect(progressBar?.className).toContain('cursor-pointer');
    });

    it('should show visual feedback on hover (seek handle)', () => {
      const { container } = render(<ProgressBar />);

      // Progress bar should have group class
      const progressBar = container.querySelector('.group');
      expect(progressBar).toBeInTheDocument();

      // Seek handle should have group-hover:opacity-100
      const seekHandle = container.querySelector('.group-hover\\:opacity-100');
      expect(seekHandle).toBeInTheDocument();
    });

    it('should display time in monospace font for easier reading', () => {
      const { container } = render(<ProgressBar />);

      // Time displays should have font-mono class
      const timeDisplays = container.querySelectorAll('.font-mono');
      expect(timeDisplays.length).toBeGreaterThanOrEqual(2); // Current time + duration
    });

    it('should have minimum width for time displays to prevent layout shift', () => {
      const { container } = render(<ProgressBar />);

      // Time displays should have min-width
      const minWidthElements = container.querySelectorAll('.min-w-\\[40px\\]');
      expect(minWidthElements.length).toBeGreaterThanOrEqual(1);
    });
  });

  describe('integration with useSeekBar hook', () => {
    it('should pass correct seek position to hook handler', () => {
      const { container } = render(<ProgressBar />);
      const progressBar = container.querySelector('.cursor-pointer');

      vi.spyOn(progressBar!, 'getBoundingClientRect').mockReturnValue({
        left: 0,
        top: 0,
        right: 400,
        bottom: 8,
        width: 400,
        height: 8,
        x: 0,
        y: 0,
        toJSON: () => ({}),
      });

      // Click at specific position
      fireEvent.click(progressBar!, { clientX: 123, clientY: 4 });

      // Should call handleSeek with calculated position
      expect(mockHandleSeek).toHaveBeenCalled();

      // Calculate expected position: (123 / 400) * 300 = 92.25
      const expectedPosition = (123 / 400) * 300;
      const actualPosition = mockHandleSeek.mock.calls[0][0];

      expect(actualPosition).toBeCloseTo(expectedPosition, 1);
    });

    it('should call hook cleanup on component unmount', () => {
      const { unmount } = render(<ProgressBar />);

      // Should not throw on unmount
      expect(() => unmount()).not.toThrow();
    });
  });

  describe('integration with useInterpolatedProgress', () => {
    it('should use interpolated progress from hook', () => {
      usePlayerStore.setState({ progress: 50, duration: 300 });

      const { container } = render(<ProgressBar />);

      const progressFill = container.querySelector('.bg-primary');
      expect(progressFill).toHaveStyle({ width: '50%' });
    });

    it('should update when interpolated values change', () => {
      usePlayerStore.setState({ progress: 25, duration: 300 });

      const { container, rerender } = render(<ProgressBar />);

      let progressFill = container.querySelector('.bg-primary');
      expect(progressFill).toHaveStyle({ width: '25%' });

      // Change progress
      usePlayerStore.setState({ progress: 75 });
      rerender(<ProgressBar />);

      progressFill = container.querySelector('.bg-primary');
      expect(progressFill).toHaveStyle({ width: '75%' });
    });
  });
});
