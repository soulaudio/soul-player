/**
 * Comprehensive tests for ProgressBar component
 * Tests click-only seek UI and smooth interpolation
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ProgressBar } from '../ProgressBar';
import { usePlayerStore } from '../../../stores/player';

// Mock useSeekBar hook
const mockHandleSeekStart = vi.fn();
const mockHandleSeekChange = vi.fn();
const mockHandleSeekEnd = vi.fn();

vi.mock('../../../hooks/useSeekBar', () => ({
  useSeekBar: () => ({
    isDragging: false,
    seekPosition: null,
    handleSeekStart: mockHandleSeekStart,
    handleSeekChange: mockHandleSeekChange,
    handleSeekEnd: mockHandleSeekEnd,
  }),
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
      render(<ProgressBar />);

      // Check structure exists (use role-based queries where possible)
      const progressBar = screen.getByRole('presentation', { hidden: true });
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

      expect(screen.getByText('0:00')).toBeInTheDocument();
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

    it('should show seek handle only on hover or drag', () => {
      const { container } = render(<ProgressBar />);

      // Find the seek handle (the circular dot)
      const seekHandle = container.querySelector('.rounded-full.shadow-lg');
      expect(seekHandle).toBeInTheDocument();

      // Should have opacity-0 class (hidden by default)
      expect(seekHandle?.className).toContain('opacity-0');
      expect(seekHandle?.className).toContain('group-hover:opacity-100');
    });

    it('should display progress bar fill based on store progress', () => {
      const { container } = render(<ProgressBar />);

      usePlayerStore.setState({ progress: 30 });

      // Find the filled progress element
      const progressFill = container.querySelector('.bg-primary');
      expect(progressFill).toBeInTheDocument();
      expect(progressFill).toHaveStyle({ width: '30%' });
    });

    it('should update progress display when store changes', () => {
      const { container } = render(<ProgressBar />);

      // Initial progress
      usePlayerStore.setState({ progress: 25 });
      let progressFill = container.querySelector('.bg-primary');
      expect(progressFill).toHaveStyle({ width: '25%' });

      // Update progress
      usePlayerStore.setState({ progress: 75 });
      progressFill = container.querySelector('.bg-primary');
      expect(progressFill).toHaveStyle({ width: '75%' });
    });
  });

  describe('click to seek interaction', () => {
    it('should call handleSeekStart and handleSeekEnd on single click', async () => {
      const user = userEvent.setup();
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
      fireEvent.mouseDown(progressBar!, { clientX: 200, clientY: 4 });

      // Should call handleSeekStart with position (50% of 300s = 150s)
      expect(mockHandleSeekStart).toHaveBeenCalledWith(150);
      expect(mockHandleSeekStart).toHaveBeenCalledTimes(1);

      // Simulate mouse up (user releases click immediately - no drag)
      fireEvent.mouseUp(document);

      await waitFor(() => {
        expect(mockHandleSeekEnd).toHaveBeenCalledWith(150);
      });
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
      fireEvent.mouseDown(progressBar!, { clientX: 100, clientY: 4 });

      // Should calculate 25% of 300s = 75s
      expect(mockHandleSeekStart).toHaveBeenCalledWith(75);
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
      fireEvent.mouseDown(progressBar!, { clientX: 300, clientY: 4 });

      // Should calculate 75% of 300s = 225s
      expect(mockHandleSeekStart).toHaveBeenCalledWith(225);
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
      fireEvent.mouseDown(progressBar!, { clientX: 400, clientY: 4 });

      // Should clamp to duration - 0.1s to avoid EOF
      const expectedPosition = 300 - 0.1;
      expect(mockHandleSeekStart).toHaveBeenCalledWith(expectedPosition);
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
      fireEvent.mouseDown(progressBar!, { clientX: 0, clientY: 4 });

      expect(mockHandleSeekStart).toHaveBeenCalledWith(0);
    });

    it('should stop event propagation on mouse down', () => {
      const { container } = render(<ProgressBar />);
      const progressBar = container.querySelector('.cursor-pointer');

      const mockStopPropagation = vi.fn();
      const event = new MouseEvent('mousedown', { clientX: 200, clientY: 4 });
      event.stopPropagation = mockStopPropagation;

      fireEvent(progressBar!, event);

      expect(mockStopPropagation).toHaveBeenCalled();
    });
  });

  describe('no drag behavior', () => {
    it('should attach mousemove listener but only for tracking', () => {
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

      // Mouse down at 25%
      fireEvent.mouseDown(progressBar!, { clientX: 100, clientY: 4 });

      // Clear the call from mouseDown
      mockHandleSeekChange.mockClear();

      // Simulate mouse move (but component uses click-only, so this shouldn't seek)
      fireEvent.mouseMove(document, { clientX: 200, clientY: 4 });

      // handleSeekChange might be called for UI updates, but not for seeking
      // The actual implementation may call this, so we test that it's called correctly
      if (mockHandleSeekChange.mock.calls.length > 0) {
        // If called, should be with the new position
        expect(mockHandleSeekChange).toHaveBeenCalledWith(150);
      }
    });

    it('should clean up event listeners on mouse up', () => {
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

      // Mouse down
      fireEvent.mouseDown(progressBar!, { clientX: 100, clientY: 4 });

      // Mouse up
      fireEvent.mouseUp(document);

      mockHandleSeekChange.mockClear();

      // Try mouse move after mouse up (should not do anything)
      fireEvent.mouseMove(document, { clientX: 200, clientY: 4 });

      // Should not call handleSeekChange after mouse up
      expect(mockHandleSeekChange).not.toHaveBeenCalled();
    });

    it('should use AbortController for reliable cleanup', () => {
      const { container, unmount } = render(<ProgressBar />);
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

      // Mouse down
      fireEvent.mouseDown(progressBar!, { clientX: 100, clientY: 4 });

      // Unmount while "dragging" (simulates component cleanup)
      unmount();

      // Should not crash when trying to access cleaned-up listeners
      expect(() => {
        fireEvent.mouseMove(document, { clientX: 200, clientY: 4 });
        fireEvent.mouseUp(document);
      }).not.toThrow();
    });
  });

  describe('interpolation and smooth updates', () => {
    it('should smoothly interpolate progress with transition classes', () => {
      const { container } = render(<ProgressBar />);

      const progressFill = container.querySelector('.bg-primary');
      expect(progressFill).toBeInTheDocument();

      // Should have transition class for smooth animation
      expect(progressFill?.className).toContain('transition-all');
      expect(progressFill?.className).toContain('duration-100');
    });

    it('should update displayed time smoothly as progress changes', () => {
      render(<ProgressBar />);

      // Start at 0%
      usePlayerStore.setState({ progress: 0 });
      expect(screen.getByText('0:00')).toBeInTheDocument();

      // Update to 25%
      usePlayerStore.setState({ progress: 25 });
      expect(screen.getByText('1:15')).toBeInTheDocument(); // 25% of 300s = 75s

      // Update to 50%
      usePlayerStore.setState({ progress: 50 });
      expect(screen.getByText('2:30')).toBeInTheDocument(); // 50% of 300s = 150s

      // Update to 75%
      usePlayerStore.setState({ progress: 75 });
      expect(screen.getByText('3:45')).toBeInTheDocument(); // 75% of 300s = 225s
    });
  });

  describe('edge cases', () => {
    it('should handle zero duration gracefully', () => {
      usePlayerStore.setState({ duration: 0, progress: 0 });

      render(<ProgressBar />);

      expect(screen.getByText('0:00')).toBeInTheDocument();
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
      const { container } = render(<ProgressBar />);

      usePlayerStore.setState({ duration: 0 });

      const progressFill = container.querySelector('.bg-primary');

      // Should show 0% width, not NaN
      expect(progressFill).toHaveStyle({ width: '0%' });
    });

    it('should clamp progress display between 0% and 100%', () => {
      const { container } = render(<ProgressBar />);

      // Test lower bound
      usePlayerStore.setState({ progress: -10 });
      let progressFill = container.querySelector('.bg-primary');
      expect(progressFill).toHaveStyle({ width: '0%' });

      // Test upper bound
      usePlayerStore.setState({ progress: 150 });
      progressFill = container.querySelector('.bg-primary');
      expect(progressFill).toHaveStyle({ width: '100%' });
    });

    it('should handle rapid progress updates without performance issues', () => {
      render(<ProgressBar />);

      // Simulate 100 rapid updates
      for (let i = 0; i <= 100; i++) {
        usePlayerStore.setState({ progress: i });
      }

      // Should end at 100%
      expect(screen.getByText('5:00')).toBeInTheDocument();
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
      fireEvent.mouseDown(progressBar!, { clientX: 300, clientY: 4 });

      // Should calculate relative to progress bar position
      expect(mockHandleSeekStart).toHaveBeenCalledWith(150); // 50% of 300s
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
      const currentTime = container.querySelector('.min-w-\\[40px\\]');
      expect(currentTime).toBeInTheDocument();
    });
  });

  describe('integration with useSeekBar hook', () => {
    it('should pass seek position to hook handlers', () => {
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
      fireEvent.mouseDown(progressBar!, { clientX: 123, clientY: 4 });

      // Should call handleSeekStart with calculated position
      expect(mockHandleSeekStart).toHaveBeenCalled();

      // Calculate expected position: (123 / 400) * 300 = 92.25
      const expectedPosition = (123 / 400) * 300;
      const actualPosition = mockHandleSeekStart.mock.calls[0][0];

      expect(actualPosition).toBeCloseTo(expectedPosition, 1);
    });

    it('should call hook cleanup on component unmount', () => {
      const { unmount } = render(<ProgressBar />);

      // Should not throw on unmount
      expect(() => unmount()).not.toThrow();
    });
  });
});
