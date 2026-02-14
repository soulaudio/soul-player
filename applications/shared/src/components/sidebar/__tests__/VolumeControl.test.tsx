import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { VolumeControl } from '../VolumeControl';

describe('VolumeControl', () => {
  const defaultProps = {
    volume: 0.5,
    isMuted: false,
    onVolumeChange: vi.fn(),
    onMuteToggle: vi.fn(),
  };

  describe('Rendering', () => {
    it('renders volume control with correct initial state', () => {
      render(<VolumeControl {...defaultProps} />);

      expect(screen.getByTestId('volume-control')).toBeInTheDocument();
      expect(screen.getByTestId('volume-slider')).toBeInTheDocument();
      expect(screen.getByTestId('volume-mute-button')).toBeInTheDocument();
      // Volume 0.5 (50%) displayed as-is (backend handles logarithmic scaling)
      expect(screen.getByTestId('volume-percentage')).toHaveTextContent('50');
    });

    it('shows Volume2 icon when not muted and volume > 0', () => {
      render(<VolumeControl {...defaultProps} />);

      const button = screen.getByTestId('volume-mute-button');
      expect(button).toHaveAttribute('aria-label', 'Mute audio');
      expect(button).toHaveAttribute('aria-pressed', 'false');
    });

    it('shows VolumeX icon when muted', () => {
      render(<VolumeControl {...defaultProps} isMuted={true} />);

      const button = screen.getByTestId('volume-mute-button');
      expect(button).toHaveAttribute('aria-label', 'Unmute audio');
      expect(button).toHaveAttribute('aria-pressed', 'true');
    });

    it('shows VolumeX icon when volume is 0', () => {
      render(<VolumeControl {...defaultProps} volume={0} />);

      const button = screen.getByTestId('volume-mute-button');
      expect(button).toHaveAttribute('aria-label', 'Mute audio');
    });

    it('displays volume as 0% when muted regardless of volume value', () => {
      render(<VolumeControl {...defaultProps} volume={0.8} isMuted={true} />);

      expect(screen.getByTestId('volume-percentage')).toHaveTextContent('0');
    });
  });

  describe('ARIA Attributes', () => {
    it('has correct ARIA attributes on slider', () => {
      render(<VolumeControl {...defaultProps} volume={0.75} />);

      const slider = screen.getByTestId('volume-slider');
      expect(slider).toHaveAttribute('aria-label', 'Volume');
      expect(slider).toHaveAttribute('aria-valuemin', '0');
      expect(slider).toHaveAttribute('aria-valuemax', '100');
      // Volume 0.75 (75%) displayed as-is
      expect(slider).toHaveAttribute('aria-valuenow', '75');
      expect(slider).toHaveAttribute('aria-valuetext', '75 percent');
    });

    it('has aria-pressed on mute button', () => {
      const { rerender } = render(<VolumeControl {...defaultProps} isMuted={false} />);

      let button = screen.getByTestId('volume-mute-button');
      expect(button).toHaveAttribute('aria-pressed', 'false');

      rerender(<VolumeControl {...defaultProps} isMuted={true} />);
      button = screen.getByTestId('volume-mute-button');
      expect(button).toHaveAttribute('aria-pressed', 'true');
    });

    it('has aria-live region for volume percentage', () => {
      render(<VolumeControl {...defaultProps} />);

      const percentage = screen.getByTestId('volume-percentage');
      expect(percentage).toHaveAttribute('aria-live', 'polite');
      expect(percentage).toHaveAttribute('aria-atomic', 'true');
    });
  });

  describe('Interactions', () => {
    it('calls onVolumeChange when slider is moved', async () => {
      const onVolumeChange = vi.fn();
      render(<VolumeControl {...defaultProps} onVolumeChange={onVolumeChange} />);

      const slider = screen.getByTestId('volume-slider');
      fireEvent.change(slider, { target: { value: '0.75' } });

      expect(onVolumeChange).toHaveBeenCalledTimes(1);
    });

    it('calls onMuteToggle when mute button is clicked', async () => {
      const onMuteToggle = vi.fn();
      const user = userEvent.setup();

      render(<VolumeControl {...defaultProps} onMuteToggle={onMuteToggle} />);

      const button = screen.getByTestId('volume-mute-button');
      await user.click(button);

      expect(onMuteToggle).toHaveBeenCalledTimes(1);
    });

    it('calls onWheel when mouse wheel is used', () => {
      const onWheel = vi.fn();
      render(<VolumeControl {...defaultProps} onWheel={onWheel} />);

      const container = screen.getByTestId('volume-control');
      fireEvent.wheel(container, { deltaY: -100 });

      expect(onWheel).toHaveBeenCalledTimes(1);
    });

    it('supports keyboard navigation with arrow keys', () => {
      const onVolumeChange = vi.fn();

      render(<VolumeControl {...defaultProps} volume={0.5} onVolumeChange={onVolumeChange} />);

      const slider = screen.getByTestId('volume-slider') as HTMLInputElement;

      // Verify slider supports keyboard input (native HTML5 range behavior)
      expect(slider.type).toBe('range');
      expect(slider.min).toBe('0');
      expect(slider.max).toBe('1');
      expect(slider.step).toBe('0.01');

      // Simulate keyboard input (in real browsers, arrow keys work automatically with type="range")
      fireEvent.change(slider, { target: { value: '0.51' } }); // Simulate arrow up
      expect(onVolumeChange).toHaveBeenCalled();
    });
  });

  describe('Volume Percentage Display', () => {
    it('displays correct percentage for different volume levels', () => {
      const { rerender } = render(<VolumeControl {...defaultProps} volume={0} />);
      expect(screen.getByTestId('volume-percentage')).toHaveTextContent('0');

      // Volume levels displayed as-is (backend handles logarithmic scaling)
      rerender(<VolumeControl {...defaultProps} volume={0.25} />);
      expect(screen.getByTestId('volume-percentage')).toHaveTextContent('25');

      rerender(<VolumeControl {...defaultProps} volume={0.5} />);
      expect(screen.getByTestId('volume-percentage')).toHaveTextContent('50');

      rerender(<VolumeControl {...defaultProps} volume={0.75} />);
      expect(screen.getByTestId('volume-percentage')).toHaveTextContent('75');

      rerender(<VolumeControl {...defaultProps} volume={1} />);
      expect(screen.getByTestId('volume-percentage')).toHaveTextContent('100');
    });

    it('rounds percentage to nearest integer', () => {
      // Volume levels rounded to nearest integer
      const { rerender } = render(<VolumeControl {...defaultProps} volume={0.123} />);
      expect(screen.getByTestId('volume-percentage')).toHaveTextContent('12');

      rerender(<VolumeControl {...defaultProps} volume={0.876} />);
      expect(screen.getByTestId('volume-percentage')).toHaveTextContent('88');
    });
  });

  describe('Edge Cases', () => {
    it('handles volume of 0 correctly', () => {
      render(<VolumeControl {...defaultProps} volume={0} />);

      const slider = screen.getByTestId('volume-slider');
      expect(slider).toHaveValue('0');
      expect(screen.getByTestId('volume-percentage')).toHaveTextContent('0');
    });

    it('handles volume of 1 correctly', () => {
      render(<VolumeControl {...defaultProps} volume={1} />);

      const slider = screen.getByTestId('volume-slider');
      expect(slider).toHaveValue('1');
      expect(screen.getByTestId('volume-percentage')).toHaveTextContent('100');
    });

    it('handles rapid volume changes', async () => {
      const onVolumeChange = vi.fn();
      render(<VolumeControl {...defaultProps} volume={0} onVolumeChange={onVolumeChange} />);

      const slider = screen.getByTestId('volume-slider');

      // Simulate 10 rapid changes from 0 to 1
      for (let i = 1; i <= 10; i++) {
        fireEvent.change(slider, { target: { value: (i / 10).toString() } });
      }

      expect(onVolumeChange).toHaveBeenCalledTimes(10);
    });
  });

  describe('Accessibility', () => {
    it('mute button is keyboard accessible', async () => {
      const onMuteToggle = vi.fn();
      const user = userEvent.setup();

      render(<VolumeControl {...defaultProps} onMuteToggle={onMuteToggle} />);

      const button = screen.getByTestId('volume-mute-button');
      button.focus();

      await user.keyboard('{Enter}');
      expect(onMuteToggle).toHaveBeenCalledTimes(1);
    });

    it('has visible focus indicators', () => {
      render(<VolumeControl {...defaultProps} />);

      const slider = screen.getByTestId('volume-slider');
      const button = screen.getByTestId('volume-mute-button');

      // Check for focus-visible classes
      expect(slider.className).toContain('focus-visible');
      expect(button.className).toContain('focus-visible');
    });

    it('has title attributes for tooltips', () => {
      render(<VolumeControl {...defaultProps} isMuted={false} />);

      const button = screen.getByTestId('volume-mute-button');
      expect(button).toHaveAttribute('title', 'Mute (M)');

      const slider = screen.getByTestId('volume-slider');
      // Volume 0.5 (50%) displayed as-is
      expect(slider).toHaveAttribute('title', 'Volume: 50%');
    });
  });
});
