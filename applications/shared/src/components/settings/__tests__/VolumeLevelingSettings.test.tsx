/**
 * Tests for VolumeLevelingSettings component
 * Focus: User interactions and meaningful behavior verification
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, act, cleanup } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { VolumeLevelingSettings } from '../audio/VolumeLevelingSettings';

// Get the mocked invoke function
const mockInvoke = vi.mocked(invoke);

// Default queue stats and worker status
const defaultQueueStats = { total: 0, pending: 0, processing: 0, completed: 0, failed: 0 };
const defaultWorkerStatus = { isRunning: false, tracksAnalyzed: 0 };

// Default props for testing
const createDefaultProps = () => ({
  mode: 'replaygain_track' as const,
  preampDb: 0,
  preventClipping: true,
  onModeChange: vi.fn(),
  onPreampChange: vi.fn(),
  onPreventClippingChange: vi.fn(),
});

// Helper to set up default mocks for async functions
const setupDefaultMocks = () => {
  mockInvoke.mockImplementation((cmd) => {
    if (cmd === 'get_analysis_queue_stats') return Promise.resolve(defaultQueueStats);
    if (cmd === 'get_analysis_worker_status') return Promise.resolve(defaultWorkerStatus);
    return Promise.resolve(undefined);
  });
};

describe('VolumeLevelingSettings Component', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    setupDefaultMocks();
  });

  afterEach(() => {
    cleanup();
  });

  describe('mode selection', () => {
    it('should call onModeChange with correct mode when user clicks a mode option', async () => {
      const props = createDefaultProps();
      await act(async () => {
        render(<VolumeLevelingSettings {...props} />);
      });

      // Test clicking each mode
      const modes = [
        { text: 'Disabled', value: 'disabled' },
        { text: 'ReplayGain (Track)', value: 'replaygain_track' },
        { text: 'ReplayGain (Album)', value: 'replaygain_album' },
        { text: 'EBU R128', value: 'ebu_r128' },
      ];

      for (const mode of modes) {
        props.onModeChange.mockClear();
        await act(async () => {
          fireEvent.click(screen.getByText(mode.text).closest('button')!);
        });
        expect(props.onModeChange).toHaveBeenCalledWith(mode.value);
      }
    });

    it('should hide pre-amp slider and prevent clipping checkbox when mode is disabled', async () => {
      const props = createDefaultProps();
      props.mode = 'disabled';
      await act(async () => {
        render(<VolumeLevelingSettings {...props} />);
      });

      expect(screen.queryByText('Pre-amp Adjustment')).not.toBeInTheDocument();
      expect(screen.queryByRole('slider')).not.toBeInTheDocument();
      expect(screen.queryByText('Prevent Clipping')).not.toBeInTheDocument();
      expect(screen.queryByRole('checkbox')).not.toBeInTheDocument();
      expect(screen.queryByText('Library Analysis')).not.toBeInTheDocument();
    });

    it('should show pre-amp slider, prevent clipping checkbox, and library analysis when mode is enabled', async () => {
      const props = createDefaultProps();
      props.mode = 'ebu_r128';
      await act(async () => {
        render(<VolumeLevelingSettings {...props} />);
      });

      expect(screen.getByText('Pre-amp Adjustment')).toBeInTheDocument();
      expect(screen.getByRole('slider')).toBeInTheDocument();
      expect(screen.getByText('Prevent Clipping')).toBeInTheDocument();
      expect(screen.getByRole('checkbox')).toBeInTheDocument();
      expect(screen.getByText('Library Analysis')).toBeInTheDocument();
    });
  });

  describe('pre-amp slider', () => {
    it('should call onPreampChange with new value when slider is changed', async () => {
      const props = createDefaultProps();
      await act(async () => {
        render(<VolumeLevelingSettings {...props} />);
      });

      const slider = screen.getByRole('slider');
      await act(async () => {
        fireEvent.change(slider, { target: { value: '6' } });
      });

      expect(props.onPreampChange).toHaveBeenCalledWith(6);
    });

    it('should call Tauri invoke directly when onPreampChange callback is not provided', async () => {
      const props = createDefaultProps();
      props.onPreampChange = undefined;
      await act(async () => {
        render(<VolumeLevelingSettings {...props} />);
      });

      const slider = screen.getByRole('slider');
      await act(async () => {
        fireEvent.change(slider, { target: { value: '-3' } });
      });

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('set_volume_leveling_preamp', { preampDb: -3 });
      });
    });

    it('should display the current preamp value formatted correctly', async () => {
      const props = createDefaultProps();
      let rerender: (ui: React.ReactElement) => void;
      await act(async () => {
        const result = render(<VolumeLevelingSettings {...props} preampDb={0} />);
        rerender = result.rerender;
      });

      expect(screen.getByText('+0.0 dB')).toBeInTheDocument();

      await act(async () => {
        rerender!(<VolumeLevelingSettings {...props} preampDb={3} />);
      });
      expect(screen.getByText('+3.0 dB')).toBeInTheDocument();

      await act(async () => {
        rerender!(<VolumeLevelingSettings {...props} preampDb={-6} />);
      });
      expect(screen.getByText('-6.0 dB')).toBeInTheDocument();
    });
  });

  describe('prevent clipping checkbox', () => {
    it('should call onPreventClippingChange when checkbox is toggled', async () => {
      const props = createDefaultProps();
      props.preventClipping = true;
      await act(async () => {
        render(<VolumeLevelingSettings {...props} />);
      });

      const checkbox = screen.getByRole('checkbox');
      expect(checkbox).toBeChecked();

      await act(async () => {
        fireEvent.click(checkbox);
      });

      expect(props.onPreventClippingChange).toHaveBeenCalledWith(false);
    });

    it('should call Tauri invoke directly when onPreventClippingChange callback is not provided', async () => {
      const props = createDefaultProps();
      props.preventClipping = true;
      props.onPreventClippingChange = undefined;
      await act(async () => {
        render(<VolumeLevelingSettings {...props} />);
      });

      const checkbox = screen.getByRole('checkbox');
      await act(async () => {
        fireEvent.click(checkbox);
      });

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('set_volume_leveling_prevent_clipping', { prevent: false });
      });
    });

    it('should reflect preventClipping prop state changes', async () => {
      const props = createDefaultProps();
      let rerender: (ui: React.ReactElement) => void;
      await act(async () => {
        const result = render(<VolumeLevelingSettings {...props} preventClipping={true} />);
        rerender = result.rerender;
      });

      expect(screen.getByRole('checkbox')).toBeChecked();

      await act(async () => {
        rerender!(<VolumeLevelingSettings {...props} preventClipping={false} />);
      });

      expect(screen.getByRole('checkbox')).not.toBeChecked();
    });
  });

  describe('library analysis', () => {
    it('should queue and start analysis when Analyze All Tracks is clicked', async () => {
      const props = createDefaultProps();
      mockInvoke.mockImplementation((cmd) => {
        if (cmd === 'queue_all_unanalyzed') return Promise.resolve(5);
        if (cmd === 'get_analysis_queue_stats') return Promise.resolve({ total: 5, pending: 5, processing: 0, completed: 0, failed: 0 });
        if (cmd === 'get_analysis_worker_status') return Promise.resolve({ isRunning: false, tracksAnalyzed: 0 });
        if (cmd === 'start_analysis_worker') return Promise.resolve(undefined);
        return Promise.resolve(undefined);
      });
      await act(async () => {
        render(<VolumeLevelingSettings {...props} />);
      });

      await act(async () => {
        fireEvent.click(screen.getByText('Analyze All Tracks'));
      });

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('queue_all_unanalyzed');
        expect(mockInvoke).toHaveBeenCalledWith('start_analysis_worker');
      });
    });

    it('should not start worker when no tracks are queued', async () => {
      const props = createDefaultProps();
      mockInvoke.mockImplementation((cmd) => {
        if (cmd === 'queue_all_unanalyzed') return Promise.resolve(0);
        if (cmd === 'get_analysis_queue_stats') return Promise.resolve({ total: 0, pending: 0, processing: 0, completed: 0, failed: 0 });
        if (cmd === 'get_analysis_worker_status') return Promise.resolve({ isRunning: false, tracksAnalyzed: 0 });
        return Promise.resolve(undefined);
      });
      await act(async () => {
        render(<VolumeLevelingSettings {...props} />);
      });

      await act(async () => {
        fireEvent.click(screen.getByText('Analyze All Tracks'));
      });

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('queue_all_unanalyzed');
      });

      expect(mockInvoke).not.toHaveBeenCalledWith('start_analysis_worker');
    });

    it('should stop analysis when Stop Analysis button is clicked', async () => {
      const props = createDefaultProps();
      mockInvoke.mockImplementation((cmd) => {
        if (cmd === 'get_analysis_queue_stats') return Promise.resolve({ total: 10, pending: 5, processing: 1, completed: 4, failed: 0 });
        if (cmd === 'get_analysis_worker_status') return Promise.resolve({ isRunning: true, tracksAnalyzed: 4 });
        return Promise.resolve(undefined);
      });
      await act(async () => {
        render(<VolumeLevelingSettings {...props} />);
      });

      await waitFor(() => {
        expect(screen.getByText('Stop Analysis')).toBeInTheDocument();
      });

      await act(async () => {
        fireEvent.click(screen.getByText('Stop Analysis'));
      });

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('stop_analysis_worker');
      });
    });

    it('should clear completed items when Clear Completed is clicked', async () => {
      const props = createDefaultProps();
      mockInvoke.mockImplementation((cmd) => {
        if (cmd === 'get_analysis_queue_stats') return Promise.resolve({ total: 10, pending: 0, processing: 0, completed: 10, failed: 0 });
        if (cmd === 'get_analysis_worker_status') return Promise.resolve({ isRunning: false, tracksAnalyzed: 10 });
        return Promise.resolve(undefined);
      });
      await act(async () => {
        render(<VolumeLevelingSettings {...props} />);
      });

      await waitFor(() => {
        expect(screen.getByText('Clear Completed')).toBeInTheDocument();
      });

      await act(async () => {
        fireEvent.click(screen.getByText('Clear Completed'));
      });

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('clear_completed_analysis');
      });
    });

    it('should resume analysis when there are pending items and Resume is clicked', async () => {
      const props = createDefaultProps();
      mockInvoke.mockImplementation((cmd) => {
        if (cmd === 'get_analysis_queue_stats') return Promise.resolve({ total: 10, pending: 5, processing: 0, completed: 5, failed: 0 });
        if (cmd === 'get_analysis_worker_status') return Promise.resolve({ isRunning: false, tracksAnalyzed: 5 });
        return Promise.resolve(undefined);
      });
      await act(async () => {
        render(<VolumeLevelingSettings {...props} />);
      });

      await waitFor(() => {
        expect(screen.getByText(/Resume/)).toBeInTheDocument();
        expect(screen.getByText(/5 pending/)).toBeInTheDocument();
      });

      await act(async () => {
        fireEvent.click(screen.getByText(/Resume/));
      });

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('start_analysis_worker');
      });
    });

    it('should refresh queue stats when refresh button is clicked', async () => {
      const props = createDefaultProps();
      await act(async () => {
        render(<VolumeLevelingSettings {...props} />);
      });

      // Wait for initial load
      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('get_analysis_queue_stats');
      });
      mockInvoke.mockClear();
      setupDefaultMocks();

      const refreshButton = screen.getByTitle('Refresh stats');
      await act(async () => {
        fireEvent.click(refreshButton);
      });

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('get_analysis_queue_stats');
      });
    });

    it('should display queue statistics from backend', async () => {
      const props = createDefaultProps();
      mockInvoke.mockImplementation((cmd) => {
        if (cmd === 'get_analysis_queue_stats') return Promise.resolve({ total: 100, pending: 50, processing: 1, completed: 45, failed: 4 });
        if (cmd === 'get_analysis_worker_status') return Promise.resolve({ isRunning: false, tracksAnalyzed: 0 });
        return Promise.resolve(undefined);
      });
      await act(async () => {
        render(<VolumeLevelingSettings {...props} />);
      });

      await waitFor(() => {
        expect(screen.getByText('50')).toBeInTheDocument(); // pending
        expect(screen.getByText('1')).toBeInTheDocument(); // processing
        expect(screen.getByText('45')).toBeInTheDocument(); // completed
        expect(screen.getByText('4')).toBeInTheDocument(); // failed
      });
    });

    it('should show analyzing status and tracks analyzed count when worker is running', async () => {
      const props = createDefaultProps();
      mockInvoke.mockImplementation((cmd) => {
        if (cmd === 'get_analysis_queue_stats') return Promise.resolve({ total: 10, pending: 5, processing: 1, completed: 4, failed: 0 });
        if (cmd === 'get_analysis_worker_status') return Promise.resolve({ isRunning: true, tracksAnalyzed: 42 });
        return Promise.resolve(undefined);
      });
      await act(async () => {
        render(<VolumeLevelingSettings {...props} />);
      });

      await waitFor(() => {
        expect(screen.getByText('Analyzing library...')).toBeInTheDocument();
        expect(screen.getByText('42 tracks analyzed this session')).toBeInTheDocument();
      });
    });
  });

  describe('error handling', () => {
    it('should handle Tauri invoke errors gracefully without crashing', async () => {
      const props = createDefaultProps();
      props.onPreampChange = undefined;
      props.onPreventClippingChange = undefined;

      mockInvoke.mockImplementation((cmd) => {
        if (cmd === 'get_analysis_queue_stats') return Promise.resolve(defaultQueueStats);
        if (cmd === 'get_analysis_worker_status') return Promise.resolve(defaultWorkerStatus);
        if (cmd === 'set_volume_leveling_preamp') return Promise.reject(new Error('Network error'));
        if (cmd === 'set_volume_leveling_prevent_clipping') return Promise.reject(new Error('Network error'));
        return Promise.resolve(undefined);
      });

      await act(async () => {
        render(<VolumeLevelingSettings {...props} />);
      });

      // Should not crash when preamp fails
      const slider = screen.getByRole('slider');
      await act(async () => {
        fireEvent.change(slider, { target: { value: '6' } });
      });

      // Should not crash when prevent clipping fails
      const checkbox = screen.getByRole('checkbox');
      await act(async () => {
        fireEvent.click(checkbox);
      });

      // Component should still be rendered
      expect(screen.getByText('Pre-amp Adjustment')).toBeInTheDocument();
    });

    it('should handle queue stats load failure gracefully', async () => {
      const props = createDefaultProps();
      mockInvoke.mockRejectedValue(new Error('Failed to load'));

      await act(async () => {
        render(<VolumeLevelingSettings {...props} />);
      });

      // Component should still render despite error
      expect(screen.getByText('Library Analysis')).toBeInTheDocument();
    });
  });
});
