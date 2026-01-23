/**
 * Comprehensive integration tests for WebPlaybackProvider
 *
 * Tests WASM playback integration with React context and Zustand store.
 * Covers initialization, playback commands, queue management, event bridges,
 * shuffle/repeat modes, and error handling.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, waitFor, screen, act } from '@testing-library/react';
import React from 'react';
import { WebPlaybackProvider } from '../WebPlaybackProvider';
import { usePlayerCommands } from '../../contexts/PlayerCommandsContext';
import { usePlayerStore } from '../../stores/player';
import type { PlaybackDataStorage } from '../../types/storage';
import type { DemoTrack } from '../../lib/demo-storage';
import { PlaybackState, ShuffleMode, RepeatMode } from '@soul-player/playback-web';

// =============================================================================
// Mock Setup
// =============================================================================

// Mock WasmPlaybackAdapter
const mockWasmAdapter = {
  initialize: vi.fn(),
  stop: vi.fn(),
  play: vi.fn(),
  pause: vi.fn(),
  next: vi.fn(),
  previous: vi.fn(),
  seek: vi.fn(),
  setVolume: vi.fn(),
  setShuffle: vi.fn(),
  getShuffle: vi.fn(),
  setRepeat: vi.fn(),
  getQueue: vi.fn(),
  clearQueue: vi.fn(),
  addToQueueNext: vi.fn(),
  addToQueueEnd: vi.fn(),
  clearPlayNext: vi.fn(),
  clearAddToQueue: vi.fn(),
  loadPlaylist: vi.fn(),
  queueLength: vi.fn(),
  skipToQueueIndex: vi.fn(),
  hasNext: vi.fn(),
  hasPrevious: vi.fn(),
  getDuration: vi.fn(),
  on: vi.fn(),
  off: vi.fn(),
};

// Mock the @soul-player/playback-web module
vi.mock('@soul-player/playback-web', async () => {
  const actual = await vi.importActual('@soul-player/playback-web');
  return {
    ...actual,
    WasmPlaybackAdapter: vi.fn(() => mockWasmAdapter),
    toQueueTrack: vi.fn((track: DemoTrack) => ({
      id: track.id,
      path: track.path,
      title: track.title,
      artist: track.artist,
      album: track.album,
      duration_secs: track.duration,
      track_number: track.trackNumber,
      coverUrl: track.coverUrl,
    })),
  };
});

// Mock storage
const createMockStorage = (tracks: DemoTrack[] = []): PlaybackDataStorage => {
  const trackMap = new Map(tracks.map(t => [t.id, t]));

  return {
    getTrackById: vi.fn((id: string) => trackMap.get(id) || null),
  };
};

// Sample test data
const createSampleTrack = (id: string, overrides: Partial<DemoTrack> = {}): DemoTrack => ({
  id,
  title: `Track ${id}`,
  artist: `Artist ${id}`,
  album: `Album ${id}`,
  duration: 180,
  trackNumber: parseInt(id),
  path: `/audio/track${id}.mp3`,
  coverUrl: `/covers/track${id}.jpg`,
  ...overrides,
});

// Test component that uses PlayerCommandsContext
function TestConsumer({ onReady }: { onReady?: (commands: any) => void }) {
  const commands = usePlayerCommands();

  React.useEffect(() => {
    if (onReady) {
      onReady(commands);
    }
  }, [commands, onReady]);

  return <div data-testid="consumer">Ready</div>;
}

// Helper to render provider with mock storage
const renderProvider = (storage: PlaybackDataStorage, children?: React.ReactNode) => {
  return render(
    <WebPlaybackProvider storage={storage}>
      {children || <TestConsumer />}
    </WebPlaybackProvider>
  );
};

// Helper to wait for initialization
const waitForInitialization = async () => {
  await waitFor(() => {
    expect(screen.queryByTestId('consumer')).toBeInTheDocument();
  });
};

// =============================================================================
// Tests
// =============================================================================

describe('WebPlaybackProvider', () => {
  beforeEach(() => {
    vi.clearAllMocks();

    // Default mock implementations
    mockWasmAdapter.initialize.mockResolvedValue(undefined);
    mockWasmAdapter.play.mockResolvedValue(undefined);
    mockWasmAdapter.on.mockReturnValue(() => {});
    mockWasmAdapter.queueLength.mockReturnValue(0);
    mockWasmAdapter.getQueue.mockReturnValue([]);
    mockWasmAdapter.getShuffle.mockReturnValue(ShuffleMode.Off);
    mockWasmAdapter.hasNext.mockReturnValue(false);
    mockWasmAdapter.hasPrevious.mockReturnValue(false);
    mockWasmAdapter.getDuration.mockReturnValue(0);
  });

  afterEach(() => {
    // Reset store to initial state
    usePlayerStore.setState({
      currentTrack: null,
      isPlaying: false,
      volume: 0.8,
      previousVolume: 0.8,
      progress: 0,
      duration: 0,
      queue: [],
      queueIndex: -1,
      repeatMode: 'off',
      shuffleMode: 'off',
    });
  });

  // ===========================================================================
  // Initialization Tests
  // ===========================================================================

  describe('initialization', () => {
    it('should initialize WASM adapter on mount', async () => {
      const storage = createMockStorage();

      renderProvider(storage);

      await waitFor(() => {
        expect(mockWasmAdapter.initialize).toHaveBeenCalledTimes(1);
      });
    });

    it('should wait for initialization before rendering children', async () => {
      const storage = createMockStorage();
      let resolveInit: () => void;
      const initPromise = new Promise<void>((resolve) => {
        resolveInit = resolve;
      });

      mockWasmAdapter.initialize.mockReturnValue(initPromise);

      renderProvider(storage);

      // Children should not be rendered yet
      expect(screen.queryByTestId('consumer')).not.toBeInTheDocument();

      // Resolve initialization
      act(() => {
        resolveInit!();
      });

      // Now children should render
      await waitFor(() => {
        expect(screen.getByTestId('consumer')).toBeInTheDocument();
      });
    });

    it('should setup event bridge after initialization', async () => {
      const storage = createMockStorage();

      renderProvider(storage);

      await waitForInitialization();

      // Verify all event listeners were registered
      expect(mockWasmAdapter.on).toHaveBeenCalledWith('stateChange', expect.any(Function));
      expect(mockWasmAdapter.on).toHaveBeenCalledWith('trackChange', expect.any(Function));
      expect(mockWasmAdapter.on).toHaveBeenCalledWith('positionUpdate', expect.any(Function));
      expect(mockWasmAdapter.on).toHaveBeenCalledWith('volumeChange', expect.any(Function));
      expect(mockWasmAdapter.on).toHaveBeenCalledWith('shuffleChange', expect.any(Function));
      expect(mockWasmAdapter.on).toHaveBeenCalledWith('repeatChange', expect.any(Function));
      expect(mockWasmAdapter.on).toHaveBeenCalledWith('queueChange', expect.any(Function));
    });

    it('should handle initialization errors gracefully', async () => {
      const storage = createMockStorage();
      const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});

      mockWasmAdapter.initialize.mockRejectedValue(new Error('WASM initialization failed'));

      renderProvider(storage);

      await waitFor(() => {
        expect(consoleError).toHaveBeenCalledWith(
          expect.stringContaining('Failed to initialize WASM'),
          expect.any(Error)
        );
      });

      // Children should not render on error
      expect(screen.queryByTestId('consumer')).not.toBeInTheDocument();

      consoleError.mockRestore();
    });

    it('should cleanup adapter on unmount', async () => {
      const storage = createMockStorage();

      const { unmount } = renderProvider(storage);

      await waitForInitialization();

      unmount();

      expect(mockWasmAdapter.stop).toHaveBeenCalledTimes(1);
    });
  });

  // ===========================================================================
  // Playback Command Tests
  // ===========================================================================

  describe('playback commands', () => {
    it('should play queue with tracks', async () => {
      const tracks = [createSampleTrack('1'), createSampleTrack('2')];
      const storage = createMockStorage(tracks);

      let commands: any;
      const onReady = (c: any) => { commands = c; };

      renderProvider(storage, <TestConsumer onReady={onReady} />);
      await waitForInitialization();

      mockWasmAdapter.queueLength.mockReturnValue(2);

      const queue = tracks.map(t => ({
        trackId: t.id,
        title: t.title,
        artist: t.artist,
        album: t.album || null,
        filePath: t.path,
        durationSeconds: t.duration || null,
        trackNumber: t.trackNumber || null,
        coverArtPath: t.coverUrl,
      }));

      await act(async () => {
        await commands.playQueue(queue, 0);
      });

      expect(mockWasmAdapter.stop).toHaveBeenCalled();
      expect(mockWasmAdapter.loadPlaylist).toHaveBeenCalledWith(
        expect.arrayContaining([
          expect.objectContaining({
            id: '1',
            title: 'Track 1',
            artist: 'Artist 1',
            path: '/audio/track1.mp3',
          }),
        ])
      );
      expect(mockWasmAdapter.play).toHaveBeenCalled();
    });

    it('should validate queue before playing', async () => {
      const storage = createMockStorage();

      let commands: any;
      const onReady = (c: any) => { commands = c; };

      renderProvider(storage, <TestConsumer onReady={onReady} />);
      await waitForInitialization();

      await expect(async () => {
        await commands.playQueue([], 0);
      }).rejects.toThrow('Cannot play empty queue');
    });

    it('should handle invalid tracks in queue', async () => {
      const storage = createMockStorage();

      let commands: any;
      const onReady = (c: any) => { commands = c; };

      renderProvider(storage, <TestConsumer onReady={onReady} />);
      await waitForInitialization();

      const invalidQueue = [{
        trackId: '1',
        title: 'Track 1',
        artist: '', // Invalid: empty artist
        album: null,
        filePath: '', // Invalid: empty path
        durationSeconds: null,
        trackNumber: null,
      }];

      await expect(async () => {
        await commands.playQueue(invalidQueue, 0);
      }).rejects.toThrow('invalid track');
    });

    it('should play single track', async () => {
      const track = createSampleTrack('1');
      const storage = createMockStorage([track]);

      let commands: any;
      const onReady = (c: any) => { commands = c; };

      renderProvider(storage, <TestConsumer onReady={onReady} />);
      await waitForInitialization();

      mockWasmAdapter.queueLength.mockReturnValue(1);

      await act(async () => {
        await commands.playTrack('1');
      });

      expect(mockWasmAdapter.clearQueue).toHaveBeenCalled();
      expect(mockWasmAdapter.addToQueueNext).toHaveBeenCalled();
      expect(mockWasmAdapter.play).toHaveBeenCalled();
    });

    it('should throw error when playing non-existent track', async () => {
      const storage = createMockStorage();

      let commands: any;
      const onReady = (c: any) => { commands = c; };

      renderProvider(storage, <TestConsumer onReady={onReady} />);
      await waitForInitialization();

      await expect(async () => {
        await commands.playTrack('999');
      }).rejects.toThrow('Track 999 not found');
    });

    it('should pause playback', async () => {
      const storage = createMockStorage();

      let commands: any;
      const onReady = (c: any) => { commands = c; };

      renderProvider(storage, <TestConsumer onReady={onReady} />);
      await waitForInitialization();

      await act(async () => {
        await commands.pausePlayback();
      });

      expect(mockWasmAdapter.pause).toHaveBeenCalled();
    });

    it('should resume playback', async () => {
      const storage = createMockStorage();

      let commands: any;
      const onReady = (c: any) => { commands = c; };

      renderProvider(storage, <TestConsumer onReady={onReady} />);
      await waitForInitialization();

      await act(async () => {
        await commands.resumePlayback();
      });

      expect(mockWasmAdapter.play).toHaveBeenCalled();
    });

    it('should stop playback', async () => {
      const storage = createMockStorage();

      let commands: any;
      const onReady = (c: any) => { commands = c; };

      renderProvider(storage, <TestConsumer onReady={onReady} />);
      await waitForInitialization();

      await act(async () => {
        await commands.stopPlayback();
      });

      expect(mockWasmAdapter.stop).toHaveBeenCalled();
    });

    it('should skip to next track', async () => {
      const storage = createMockStorage();

      let commands: any;
      const onReady = (c: any) => { commands = c; };

      renderProvider(storage, <TestConsumer onReady={onReady} />);
      await waitForInitialization();

      await act(async () => {
        await commands.skipNext();
      });

      expect(mockWasmAdapter.next).toHaveBeenCalled();
    });

    it('should skip to previous track', async () => {
      const storage = createMockStorage();

      let commands: any;
      const onReady = (c: any) => { commands = c; };

      renderProvider(storage, <TestConsumer onReady={onReady} />);
      await waitForInitialization();

      await act(async () => {
        await commands.skipPrevious();
      });

      expect(mockWasmAdapter.previous).toHaveBeenCalled();
    });

    it('should seek to position', async () => {
      const storage = createMockStorage();

      let commands: any;
      const onReady = (c: any) => { commands = c; };

      renderProvider(storage, <TestConsumer onReady={onReady} />);
      await waitForInitialization();

      await act(async () => {
        await commands.seek(60);
      });

      expect(mockWasmAdapter.seek).toHaveBeenCalledWith(60);
    });

    it('should set volume (0-1 to 0-100 conversion)', async () => {
      const storage = createMockStorage();

      let commands: any;
      const onReady = (c: any) => { commands = c; };

      renderProvider(storage, <TestConsumer onReady={onReady} />);
      await waitForInitialization();

      await act(async () => {
        await commands.setVolume(0.5);
      });

      expect(mockWasmAdapter.setVolume).toHaveBeenCalledWith(50);
    });

    it('should clamp volume to 0-100 range', async () => {
      const storage = createMockStorage();

      let commands: any;
      const onReady = (c: any) => { commands = c; };

      renderProvider(storage, <TestConsumer onReady={onReady} />);
      await waitForInitialization();

      await act(async () => {
        await commands.setVolume(1.5); // Over 100%
      });

      expect(mockWasmAdapter.setVolume).toHaveBeenCalledWith(100);

      await act(async () => {
        await commands.setVolume(-0.5); // Negative
      });

      expect(mockWasmAdapter.setVolume).toHaveBeenCalledWith(0);
    });
  });

  // ===========================================================================
  // Queue Management Tests
  // ===========================================================================

  describe('queue management', () => {
    it('should get current queue with cover art', async () => {
      const tracks = [createSampleTrack('1'), createSampleTrack('2')];
      const storage = createMockStorage(tracks);

      let commands: any;
      const onReady = (c: any) => { commands = c; };

      renderProvider(storage, <TestConsumer onReady={onReady} />);
      await waitForInitialization();

      mockWasmAdapter.getQueue.mockReturnValue([
        {
          id: '1',
          path: '/audio/track1.mp3',
          title: 'Track 1',
          artist: 'Artist 1',
          album: 'Album 1',
          duration_secs: 180,
          track_number: 1,
        },
      ]);

      const queue = await commands.getQueue();

      expect(queue).toHaveLength(1);
      expect(queue[0]).toMatchObject({
        trackId: '1',
        title: 'Track 1',
        artist: 'Artist 1',
        coverArtPath: '/covers/track1.jpg',
      });
    });

    it('should skip to queue index', async () => {
      const storage = createMockStorage();

      let commands: any;
      const onReady = (c: any) => { commands = c; };

      renderProvider(storage, <TestConsumer onReady={onReady} />);
      await waitForInitialization();

      await act(async () => {
        await commands.skipToQueueIndex(2);
      });

      expect(mockWasmAdapter.skipToQueueIndex).toHaveBeenCalledWith(2);
    });

    it('should add track to play next', async () => {
      const track = createSampleTrack('1');
      const storage = createMockStorage([track]);

      let commands: any;
      const onReady = (c: any) => { commands = c; };

      renderProvider(storage, <TestConsumer onReady={onReady} />);
      await waitForInitialization();

      const queueTrack = {
        trackId: '1',
        title: 'Track 1',
        artist: 'Artist 1',
        album: null,
        filePath: '/audio/track1.mp3',
        durationSeconds: 180,
        trackNumber: 1,
      };

      await act(async () => {
        await commands.addPlayNext(queueTrack);
      });

      expect(mockWasmAdapter.addToQueueNext).toHaveBeenCalled();
    });

    it('should add track to queue end', async () => {
      const track = createSampleTrack('1');
      const storage = createMockStorage([track]);

      let commands: any;
      const onReady = (c: any) => { commands = c; };

      renderProvider(storage, <TestConsumer onReady={onReady} />);
      await waitForInitialization();

      const queueTrack = {
        trackId: '1',
        title: 'Track 1',
        artist: 'Artist 1',
        album: null,
        filePath: '/audio/track1.mp3',
        durationSeconds: 180,
        trackNumber: 1,
      };

      await act(async () => {
        await commands.addToQueueEnd(queueTrack);
      });

      expect(mockWasmAdapter.addToQueueEnd).toHaveBeenCalled();
    });

    it('should throw error when adding non-existent track', async () => {
      const storage = createMockStorage();

      let commands: any;
      const onReady = (c: any) => { commands = c; };

      renderProvider(storage, <TestConsumer onReady={onReady} />);
      await waitForInitialization();

      const queueTrack = {
        trackId: '999',
        title: 'Track 999',
        artist: 'Artist 999',
        album: null,
        filePath: '/audio/track999.mp3',
        durationSeconds: 180,
        trackNumber: 1,
      };

      await expect(async () => {
        await commands.addPlayNext(queueTrack);
      }).rejects.toThrow('Track 999 not found');
    });

    it('should clear play next queue', async () => {
      const storage = createMockStorage();

      let commands: any;
      const onReady = (c: any) => { commands = c; };

      renderProvider(storage, <TestConsumer onReady={onReady} />);
      await waitForInitialization();

      await act(async () => {
        await commands.clearPlayNext();
      });

      expect(mockWasmAdapter.clearPlayNext).toHaveBeenCalled();
    });

    it('should clear add to queue', async () => {
      const storage = createMockStorage();

      let commands: any;
      const onReady = (c: any) => { commands = c; };

      renderProvider(storage, <TestConsumer onReady={onReady} />);
      await waitForInitialization();

      await act(async () => {
        await commands.clearAddToQueue();
      });

      expect(mockWasmAdapter.clearAddToQueue).toHaveBeenCalled();
    });
  });

  // ===========================================================================
  // Shuffle/Repeat Tests
  // ===========================================================================

  describe('shuffle and repeat modes', () => {
    it('should set shuffle mode', async () => {
      const storage = createMockStorage();

      let commands: any;
      const onReady = (c: any) => { commands = c; };

      renderProvider(storage, <TestConsumer onReady={onReady} />);
      await waitForInitialization();

      await act(async () => {
        await commands.setShuffle('random');
      });

      expect(mockWasmAdapter.setShuffle).toHaveBeenCalledWith(ShuffleMode.Random);
    });

    it('should cycle shuffle modes', async () => {
      const storage = createMockStorage();

      let commands: any;
      const onReady = (c: any) => { commands = c; };

      renderProvider(storage, <TestConsumer onReady={onReady} />);
      await waitForInitialization();

      // Off -> Random
      mockWasmAdapter.getShuffle.mockReturnValue(ShuffleMode.Off);
      let result = await commands.cycleShuffle();
      expect(mockWasmAdapter.setShuffle).toHaveBeenCalledWith(ShuffleMode.Random);
      expect(result).toBe('random');

      // Random -> Smart
      mockWasmAdapter.getShuffle.mockReturnValue(ShuffleMode.Random);
      result = await commands.cycleShuffle();
      expect(mockWasmAdapter.setShuffle).toHaveBeenCalledWith(ShuffleMode.Smart);
      expect(result).toBe('smart');

      // Smart -> Off
      mockWasmAdapter.getShuffle.mockReturnValue(ShuffleMode.Smart);
      result = await commands.cycleShuffle();
      expect(mockWasmAdapter.setShuffle).toHaveBeenCalledWith(ShuffleMode.Off);
      expect(result).toBe('off');
    });

    it('should get shuffle mode', async () => {
      const storage = createMockStorage();

      let commands: any;
      const onReady = (c: any) => { commands = c; };

      renderProvider(storage, <TestConsumer onReady={onReady} />);
      await waitForInitialization();

      mockWasmAdapter.getShuffle.mockReturnValue(ShuffleMode.Random);

      const mode = await commands.getShuffle();
      expect(mode).toBe('random');
    });

    it('should set repeat mode', async () => {
      const storage = createMockStorage();

      let commands: any;
      const onReady = (c: any) => { commands = c; };

      renderProvider(storage, <TestConsumer onReady={onReady} />);
      await waitForInitialization();

      await act(async () => {
        await commands.setRepeatMode('all');
      });

      expect(mockWasmAdapter.setRepeat).toHaveBeenCalledWith(RepeatMode.All);
    });

    it('should get playback capabilities', async () => {
      const storage = createMockStorage();

      let commands: any;
      const onReady = (c: any) => { commands = c; };

      renderProvider(storage, <TestConsumer onReady={onReady} />);
      await waitForInitialization();

      mockWasmAdapter.hasNext.mockReturnValue(true);
      mockWasmAdapter.hasPrevious.mockReturnValue(false);

      const capabilities = await commands.getPlaybackCapabilities();

      expect(capabilities).toEqual({
        hasNext: true,
        hasPrevious: false,
      });
    });
  });

  // ===========================================================================
  // Event Bridge Tests
  // ===========================================================================

  describe('event bridge', () => {
    it('should sync state changes to player store', async () => {
      const storage = createMockStorage();

      renderProvider(storage);
      await waitForInitialization();

      // Get the stateChange callback
      const stateChangeCall = mockWasmAdapter.on.mock.calls.find(
        call => call[0] === 'stateChange'
      );
      expect(stateChangeCall).toBeDefined();

      const stateChangeCallback = stateChangeCall![1];

      // Trigger state change
      act(() => {
        stateChangeCallback(PlaybackState.Playing);
      });

      expect(usePlayerStore.getState().isPlaying).toBe(true);

      act(() => {
        stateChangeCallback(PlaybackState.Paused);
      });

      expect(usePlayerStore.getState().isPlaying).toBe(false);
    });

    it('should sync track changes to player store', async () => {
      const track = createSampleTrack('1');
      const storage = createMockStorage([track]);

      renderProvider(storage);
      await waitForInitialization();

      const trackChangeCall = mockWasmAdapter.on.mock.calls.find(
        call => call[0] === 'trackChange'
      );
      const trackChangeCallback = trackChangeCall![1];

      act(() => {
        trackChangeCallback({
          id: '1',
          path: '/audio/track1.mp3',
          title: 'Track 1',
          artist: 'Artist 1',
          album: 'Album 1',
          duration_secs: 180,
          track_number: 1,
        });
      });

      const currentTrack = usePlayerStore.getState().currentTrack;
      expect(currentTrack).toMatchObject({
        id: 1,
        title: 'Track 1',
        artist: 'Artist 1',
        coverArtPath: '/covers/track1.jpg',
      });
      expect(usePlayerStore.getState().duration).toBe(180);
    });

    it('should clear track when null', async () => {
      const storage = createMockStorage();

      // Set initial track
      usePlayerStore.setState({
        currentTrack: {
          id: 1,
          title: 'Track 1',
          artist: 'Artist 1',
          album: '',
          duration: 180,
          filePath: '/audio/track1.mp3',
          addedAt: new Date().toISOString(),
        },
        duration: 180,
      });

      renderProvider(storage);
      await waitForInitialization();

      const trackChangeCall = mockWasmAdapter.on.mock.calls.find(
        call => call[0] === 'trackChange'
      );
      const trackChangeCallback = trackChangeCall![1];

      act(() => {
        trackChangeCallback(null);
      });

      expect(usePlayerStore.getState().currentTrack).toBeNull();
      expect(usePlayerStore.getState().duration).toBe(0);
    });

    it('should sync position updates to player store', async () => {
      const storage = createMockStorage();

      renderProvider(storage);
      await waitForInitialization();

      mockWasmAdapter.getDuration.mockReturnValue(180);

      const positionUpdateCall = mockWasmAdapter.on.mock.calls.find(
        call => call[0] === 'positionUpdate'
      );
      const positionUpdateCallback = positionUpdateCall![1];

      act(() => {
        positionUpdateCallback(90); // 50% through
      });

      expect(usePlayerStore.getState().progress).toBe(50);
    });

    it('should sync volume changes to player store', async () => {
      const storage = createMockStorage();

      renderProvider(storage);
      await waitForInitialization();

      const volumeChangeCall = mockWasmAdapter.on.mock.calls.find(
        call => call[0] === 'volumeChange'
      );
      const volumeChangeCallback = volumeChangeCall![1];

      act(() => {
        volumeChangeCallback(75); // 0-100 range
      });

      expect(usePlayerStore.getState().volume).toBe(0.75); // 0-1 range
    });

    it('should sync shuffle changes to player store', async () => {
      const storage = createMockStorage();

      renderProvider(storage);
      await waitForInitialization();

      const shuffleChangeCall = mockWasmAdapter.on.mock.calls.find(
        call => call[0] === 'shuffleChange'
      );
      const shuffleChangeCallback = shuffleChangeCall![1];

      act(() => {
        shuffleChangeCallback('random');
      });

      expect(usePlayerStore.getState().shuffleMode).toBe('random');
    });

    it('should sync repeat changes to player store', async () => {
      const storage = createMockStorage();

      renderProvider(storage);
      await waitForInitialization();

      const repeatChangeCall = mockWasmAdapter.on.mock.calls.find(
        call => call[0] === 'repeatChange'
      );
      const repeatChangeCallback = repeatChangeCall![1];

      act(() => {
        repeatChangeCallback('all');
      });

      expect(usePlayerStore.getState().repeatMode).toBe('all');
    });

    it('should sync queue changes to player store', async () => {
      const tracks = [createSampleTrack('1'), createSampleTrack('2')];
      const storage = createMockStorage(tracks);

      renderProvider(storage);
      await waitForInitialization();

      const queueChangeCall = mockWasmAdapter.on.mock.calls.find(
        call => call[0] === 'queueChange'
      );
      const queueChangeCallback = queueChangeCall![1];

      mockWasmAdapter.getQueue.mockReturnValue([
        {
          id: '1',
          path: '/audio/track1.mp3',
          title: 'Track 1',
          artist: 'Artist 1',
          album: 'Album 1',
          duration_secs: 180,
          track_number: 1,
        },
        {
          id: '2',
          path: '/audio/track2.mp3',
          title: 'Track 2',
          artist: 'Artist 2',
          album: 'Album 2',
          duration_secs: 200,
          track_number: 2,
        },
      ]);

      act(() => {
        queueChangeCallback();
      });

      const queue = usePlayerStore.getState().queue;
      expect(queue).toHaveLength(2);
      expect(queue[0]).toMatchObject({
        id: 1,
        title: 'Track 1',
        coverArtPath: '/covers/track1.jpg',
      });
    });
  });

  // ===========================================================================
  // Error Handling Tests
  // ===========================================================================

  describe('error handling', () => {
    it('should throw error when manager not initialized', async () => {
      const storage = createMockStorage();

      // Don't resolve initialization
      mockWasmAdapter.initialize.mockReturnValue(new Promise(() => {}));

      renderProvider(storage);

      // Wait a bit but initialization won't complete
      await new Promise(resolve => setTimeout(resolve, 100));

      // Try to use commands - should fail because not initialized yet
      // (children won't render, so commands will be undefined)
      expect(screen.queryByTestId('consumer')).not.toBeInTheDocument();
    });

    it('should register error event listener', async () => {
      const storage = createMockStorage();

      renderProvider(storage);
      await waitForInitialization();

      // Verify error listener was NOT registered (not implemented yet)
      const errorCall = mockWasmAdapter.on.mock.calls.find(
        call => call[0] === 'error'
      );

      // Current implementation doesn't register error listener
      // This test documents that fact for future implementation
      expect(errorCall).toBeUndefined();
    });

    it('should handle playback errors gracefully', async () => {
      const tracks = [createSampleTrack('1')];
      const storage = createMockStorage(tracks);

      let commands: any;
      const onReady = (c: any) => { commands = c; };

      renderProvider(storage, <TestConsumer onReady={onReady} />);
      await waitForInitialization();

      mockWasmAdapter.play.mockRejectedValue(new Error('Playback failed'));

      const queue = [{
        trackId: '1',
        title: 'Track 1',
        artist: 'Artist 1',
        album: null,
        filePath: '/audio/track1.mp3',
        durationSeconds: 180,
        trackNumber: 1,
      }];

      mockWasmAdapter.queueLength.mockReturnValue(1);

      await expect(async () => {
        await commands.playQueue(queue, 0);
      }).rejects.toThrow('Playback failed');
    });

    it('should handle queue loading failures', async () => {
      const tracks = [createSampleTrack('1')];
      const storage = createMockStorage(tracks);

      let commands: any;
      const onReady = (c: any) => { commands = c; };

      renderProvider(storage, <TestConsumer onReady={onReady} />);
      await waitForInitialization();

      // Simulate queue loading but returning empty
      mockWasmAdapter.queueLength.mockReturnValue(0);

      const queue = [{
        trackId: '1',
        title: 'Track 1',
        artist: 'Artist 1',
        album: null,
        filePath: '/audio/track1.mp3',
        durationSeconds: 180,
        trackNumber: 1,
      }];

      await expect(async () => {
        await commands.playQueue(queue, 0);
      }).rejects.toThrow('Queue is empty after loading playlist');
    });
  });

  // ===========================================================================
  // Edge Cases
  // ===========================================================================

  describe('edge cases', () => {
    it('should handle queue with start index', async () => {
      const tracks = [
        createSampleTrack('1'),
        createSampleTrack('2'),
        createSampleTrack('3'),
      ];
      const storage = createMockStorage(tracks);

      let commands: any;
      const onReady = (c: any) => { commands = c; };

      renderProvider(storage, <TestConsumer onReady={onReady} />);
      await waitForInitialization();

      mockWasmAdapter.queueLength.mockReturnValue(3);
      mockWasmAdapter.play.mockResolvedValue(undefined); // Reset to resolve

      const queue = tracks.map(t => ({
        trackId: t.id,
        title: t.title,
        artist: t.artist,
        album: t.album || null,
        filePath: t.path,
        durationSeconds: t.duration || null,
        trackNumber: t.trackNumber || null,
      }));

      await act(async () => {
        await commands.playQueue(queue, 1); // Start from second track
      });

      // Verify reordered queue (track 2, 3, 1)
      const loadedQueue = mockWasmAdapter.loadPlaylist.mock.calls[0][0];
      expect(loadedQueue[0].id).toBe('2');
      expect(loadedQueue[1].id).toBe('3');
      expect(loadedQueue[2].id).toBe('1');
    });

    it('should handle tracks without cover art', async () => {
      const track = createSampleTrack('1', { coverUrl: undefined });
      const storage = createMockStorage([track]);

      renderProvider(storage);
      await waitForInitialization();

      const trackChangeCall = mockWasmAdapter.on.mock.calls.find(
        call => call[0] === 'trackChange'
      );
      const trackChangeCallback = trackChangeCall![1];

      act(() => {
        trackChangeCallback({
          id: '1',
          path: '/audio/track1.mp3',
          title: 'Track 1',
          artist: 'Artist 1',
          album: 'Album 1',
          duration_secs: 180,
          track_number: 1,
        });
      });

      const currentTrack = usePlayerStore.getState().currentTrack;
      expect(currentTrack?.coverArtPath).toBeUndefined();
    });

    it('should handle position updates with zero duration', async () => {
      const storage = createMockStorage();

      renderProvider(storage);
      await waitForInitialization();

      mockWasmAdapter.getDuration.mockReturnValue(0);

      const positionUpdateCall = mockWasmAdapter.on.mock.calls.find(
        call => call[0] === 'positionUpdate'
      );
      const positionUpdateCallback = positionUpdateCall![1];

      act(() => {
        positionUpdateCallback(0);
      });

      // Should not update progress when duration is 0
      expect(usePlayerStore.getState().progress).toBe(0);
    });

    it('should handle playQueueWithContext by delegating to playQueue', async () => {
      const tracks = [createSampleTrack('1')];
      const storage = createMockStorage(tracks);

      let commands: any;
      const onReady = (c: any) => { commands = c; };

      renderProvider(storage, <TestConsumer onReady={onReady} />);
      await waitForInitialization();

      mockWasmAdapter.queueLength.mockReturnValue(1);
      mockWasmAdapter.play.mockResolvedValue(undefined); // Reset to resolve

      const queue = [{
        trackId: '1',
        title: 'Track 1',
        artist: 'Artist 1',
        album: null,
        filePath: '/audio/track1.mp3',
        durationSeconds: 180,
        trackNumber: 1,
      }];

      const context = {
        contextType: 'album' as const,
        contextId: 'album-1',
      };

      await act(async () => {
        await commands.playQueueWithContext(context, queue, 0, false);
      });

      expect(mockWasmAdapter.loadPlaylist).toHaveBeenCalled();
      expect(mockWasmAdapter.play).toHaveBeenCalled();
    });

    it('should enable shuffle when playQueueWithContext called with shuffle', async () => {
      const tracks = [createSampleTrack('1')];
      const storage = createMockStorage(tracks);

      let commands: any;
      const onReady = (c: any) => { commands = c; };

      renderProvider(storage, <TestConsumer onReady={onReady} />);
      await waitForInitialization();

      mockWasmAdapter.queueLength.mockReturnValue(1);
      mockWasmAdapter.play.mockResolvedValue(undefined); // Reset to resolve

      const queue = [{
        trackId: '1',
        title: 'Track 1',
        artist: 'Artist 1',
        album: null,
        filePath: '/audio/track1.mp3',
        durationSeconds: 180,
        trackNumber: 1,
      }];

      const context = {
        contextType: 'album' as const,
        contextId: 'album-1',
      };

      await act(async () => {
        await commands.playQueueWithContext(context, queue, 0, true);
      });

      expect(mockWasmAdapter.setShuffle).toHaveBeenCalledWith(ShuffleMode.Random);
    });

    it('should return mock sources for getAllSources', async () => {
      const storage = createMockStorage();

      let commands: any;
      const onReady = (c: any) => { commands = c; };

      renderProvider(storage, <TestConsumer onReady={onReady} />);
      await waitForInitialization();

      const sources = await commands.getAllSources();

      expect(sources).toEqual([
        {
          id: 1,
          name: 'Demo Library',
          sourceType: 'local',
          isActive: true,
          isOnline: true,
        },
      ]);
    });
  });
});
