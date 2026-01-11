/**
 * Comprehensive tests for TrackList component
 * Tests source/format columns, play/pause, track availability, and rendering
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { TrackList, Track, SourceType } from '../TrackList';
import { PlayerCommandsProvider, PlayerContextValue, QueueTrack } from '../../contexts/PlayerCommandsContext';
import { usePlayerStore } from '../../stores/player';

// Mock player commands
const createMockCommands = () => ({
  playTrack: vi.fn().mockResolvedValue(undefined),
  pausePlayback: vi.fn().mockResolvedValue(undefined),
  resumePlayback: vi.fn().mockResolvedValue(undefined),
  stopPlayback: vi.fn().mockResolvedValue(undefined),
  skipNext: vi.fn().mockResolvedValue(undefined),
  skipPrevious: vi.fn().mockResolvedValue(undefined),
  seek: vi.fn().mockResolvedValue(undefined),
  setVolume: vi.fn().mockResolvedValue(undefined),
  setShuffle: vi.fn().mockResolvedValue(undefined),
  setRepeatMode: vi.fn().mockResolvedValue(undefined),
  getPlaybackCapabilities: vi.fn().mockResolvedValue({ hasNext: true, hasPrevious: true }),
  getQueue: vi.fn().mockResolvedValue([]),
  playQueue: vi.fn().mockResolvedValue(undefined),
  skipToQueueIndex: vi.fn().mockResolvedValue(undefined),
  getAllSources: vi.fn().mockResolvedValue([]),
});

const createMockEvents = () => ({
  onStateChange: vi.fn(() => () => {}),
  onTrackChange: vi.fn(() => () => {}),
  onPositionUpdate: vi.fn(() => () => {}),
  onVolumeChange: vi.fn(() => () => {}),
  onQueueUpdate: vi.fn(() => () => {}),
  onError: vi.fn(() => () => {}),
});

// Helper to create mock context value
const createMockContext = (): PlayerContextValue => ({
  commands: createMockCommands(),
  events: createMockEvents(),
});

// Helper to render with provider
const renderWithProvider = (
  ui: React.ReactElement,
  contextValue: PlayerContextValue = createMockContext()
) => {
  return render(
    <PlayerCommandsProvider value={contextValue}>
      {ui}
    </PlayerCommandsProvider>
  );
};

// Default buildQueue implementation
const defaultBuildQueue = (tracks: Track[], _clickedTrack: Track, clickedIndex: number): QueueTrack[] => {
  return tracks.map((track, idx) => ({
    trackId: String(track.id),
    title: track.title,
    artist: track.artist || 'Unknown Artist',
    album: track.album || null,
    filePath: `/path/to/track${track.id}.flac`,
    durationSeconds: track.duration || null,
    trackNumber: track.trackNumber || idx + 1,
  }));
};

// Sample tracks for testing
const createSampleTracks = (): Track[] => [
  {
    id: 1,
    title: 'Lossless Track',
    artist: 'Test Artist',
    album: 'Test Album',
    duration: 240,
    trackNumber: 1,
    format: 'flac',
    sampleRate: 44100,
    channels: 2,
    sourceType: 'local' as SourceType,
    sourceName: 'Local Library',
  },
  {
    id: 2,
    title: 'Hi-Res Track',
    artist: 'Hi-Res Artist',
    album: 'Hi-Res Album',
    duration: 180,
    trackNumber: 2,
    format: 'flac',
    sampleRate: 192000,
    channels: 2,
    sourceType: 'server' as SourceType,
    sourceName: 'Music Server',
    sourceOnline: true,
  },
  {
    id: 3,
    title: 'Lossy Track',
    artist: 'MP3 Artist',
    album: 'MP3 Album',
    duration: 200,
    trackNumber: 3,
    format: 'mp3',
    bitrate: 320,
    channels: 2,
    sourceType: 'cached' as SourceType,
    sourceName: 'Cached from Server',
  },
  {
    id: 4,
    title: 'Unavailable Track',
    artist: 'Missing Artist',
    album: 'Missing Album',
    duration: 150,
    trackNumber: 4,
    format: 'flac',
    sourceType: 'local' as SourceType,
    isAvailable: false,
  },
  {
    id: 5,
    title: 'Minimal Track',
    artist: 'Minimal Artist',
    duration: 120,
    trackNumber: 5,
    // No format or source info
  },
];

describe('TrackList Component', () => {
  beforeEach(() => {
    // Reset player store state
    usePlayerStore.setState({
      currentTrack: null,
      isPlaying: false,
      volume: 0.8,
      progress: 0,
      duration: 0,
      queue: [],
      queueIndex: -1,
      repeatMode: 'off',
      shuffleEnabled: false,
    });
  });

  describe('basic rendering', () => {
    it('should render track list with all tracks', () => {
      const tracks = createSampleTracks();
      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      expect(screen.getByText('Lossless Track')).toBeInTheDocument();
      expect(screen.getByText('Hi-Res Track')).toBeInTheDocument();
      expect(screen.getByText('Lossy Track')).toBeInTheDocument();
      expect(screen.getByText('Unavailable Track')).toBeInTheDocument();
      expect(screen.getByText('Minimal Track')).toBeInTheDocument();
    });

    it('should render header columns', () => {
      const tracks = createSampleTracks();
      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      expect(screen.getByText('#')).toBeInTheDocument();
      expect(screen.getByText('Title')).toBeInTheDocument();
      expect(screen.getByText('Artist')).toBeInTheDocument();
      expect(screen.getByText('Album')).toBeInTheDocument();
      expect(screen.getByText('Format')).toBeInTheDocument();
      expect(screen.getByText('Source')).toBeInTheDocument();
      expect(screen.getByText('Duration')).toBeInTheDocument();
    });

    it('should render empty state when no tracks', () => {
      renderWithProvider(
        <TrackList tracks={[]} buildQueue={defaultBuildQueue} />
      );

      expect(screen.getByText('No tracks found')).toBeInTheDocument();
      expect(screen.getByText('Add music to get started')).toBeInTheDocument();
    });

    it('should render artist information', () => {
      const tracks = createSampleTracks();
      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      expect(screen.getByText('Test Artist')).toBeInTheDocument();
      expect(screen.getByText('Hi-Res Artist')).toBeInTheDocument();
      expect(screen.getByText('MP3 Artist')).toBeInTheDocument();
    });

    it('should render album information', () => {
      const tracks = createSampleTracks();
      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      expect(screen.getByText('Test Album')).toBeInTheDocument();
      expect(screen.getByText('Hi-Res Album')).toBeInTheDocument();
      expect(screen.getByText('MP3 Album')).toBeInTheDocument();
    });

    it('should show dash for missing album', () => {
      const tracks = [{ id: 1, title: 'No Album Track', artist: 'Artist' }];
      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      // Album column should show dash
      const dashes = screen.getAllByText('—');
      expect(dashes.length).toBeGreaterThan(0);
    });

    it('should show "Unknown Artist" when artist is missing', () => {
      const tracks = [{ id: 1, title: 'No Artist Track' }];
      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      expect(screen.getByText('Unknown Artist')).toBeInTheDocument();
    });
  });

  describe('format column rendering', () => {
    it('should render TrackQualityBadge for tracks with format', () => {
      const tracks = createSampleTracks();
      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      // FLAC badge should appear (may appear multiple times)
      const flacBadges = screen.getAllByText('FLAC');
      expect(flacBadges.length).toBeGreaterThanOrEqual(1);
      // Hi-Res badge should appear
      expect(screen.getByText('Hi-Res 192kHz')).toBeInTheDocument();
      // MP3 with bitrate
      expect(screen.getByText('MP3 320')).toBeInTheDocument();
    });

    it('should render dash for tracks without format', () => {
      const tracks = [{ id: 1, title: 'No Format Track', artist: 'Artist' }];
      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      // Should have dashes for format, source, and album
      const dashes = screen.getAllByText('—');
      expect(dashes.length).toBeGreaterThanOrEqual(2);
    });

    it('should pass bitrate to TrackQualityBadge', () => {
      const tracks = [
        {
          id: 1,
          title: 'MP3 Track',
          artist: 'Artist',
          format: 'mp3',
          bitrate: 256,
        },
      ];
      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      expect(screen.getByText('MP3 256')).toBeInTheDocument();
    });

    it('should pass sampleRate to TrackQualityBadge for hi-res detection', () => {
      const tracks = [
        {
          id: 1,
          title: 'Hi-Res Track',
          artist: 'Artist',
          format: 'flac',
          sampleRate: 96000,
        },
      ];
      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      expect(screen.getByText('Hi-Res 96kHz')).toBeInTheDocument();
    });

    it('should pass channels to TrackQualityBadge for tooltip', () => {
      const tracks = [
        {
          id: 1,
          title: 'Stereo Track',
          artist: 'Artist',
          format: 'flac',
          channels: 2,
        },
      ];
      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      const badge = screen.getByText('FLAC');
      expect(badge.getAttribute('title')).toContain('Stereo');
    });
  });

  describe('source column rendering', () => {
    it('should render SourceIndicator for tracks with sourceType', () => {
      const tracks = createSampleTracks();
      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      // Local source indicator
      expect(screen.getByTitle('Local Library')).toBeInTheDocument();
      // Server source indicator (online)
      expect(screen.getByTitle('Music Server (Online)')).toBeInTheDocument();
      // Cached source indicator
      expect(screen.getByTitle('Cached from Server')).toBeInTheDocument();
    });

    it('should render dash for tracks without sourceType', () => {
      const tracks = [{ id: 1, title: 'No Source Track', artist: 'Artist' }];
      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      // Should have dashes for format, source, and album
      const dashes = screen.getAllByText('—');
      expect(dashes.length).toBeGreaterThanOrEqual(2);
    });

    it('should show online status for server sources', () => {
      const tracks = [
        {
          id: 1,
          title: 'Online Server Track',
          artist: 'Artist',
          sourceType: 'server' as SourceType,
          sourceName: 'My Server',
          sourceOnline: true,
        },
      ];
      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      expect(screen.getByTitle('My Server (Online)')).toBeInTheDocument();
    });

    it('should show offline status for server sources', () => {
      const tracks = [
        {
          id: 1,
          title: 'Offline Server Track',
          artist: 'Artist',
          sourceType: 'server' as SourceType,
          sourceName: 'My Server',
          sourceOnline: false,
        },
      ];
      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      expect(screen.getByTitle('My Server (Offline)')).toBeInTheDocument();
    });

    it('should render local source indicator', () => {
      const tracks = [
        {
          id: 1,
          title: 'Local Track',
          artist: 'Artist',
          sourceType: 'local' as SourceType,
          sourceName: 'Music Folder',
        },
      ];
      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      expect(screen.getByTitle('Music Folder')).toBeInTheDocument();
    });

    it('should render cached source indicator', () => {
      const tracks = [
        {
          id: 1,
          title: 'Cached Track',
          artist: 'Artist',
          sourceType: 'cached' as SourceType,
          sourceName: 'Cached Files',
        },
      ];
      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      expect(screen.getByTitle('Cached Files')).toBeInTheDocument();
    });
  });

  describe('track availability', () => {
    it('should show warning icon for unavailable tracks', () => {
      const tracks = [
        {
          id: 1,
          title: 'Missing Track',
          artist: 'Artist',
          isAvailable: false,
        },
      ];
      const { container } = renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      // Warning icon should be present (AlertTriangle)
      const warningIcon = container.querySelector('.lucide-triangle-alert');
      expect(warningIcon).toBeInTheDocument();
    });

    it('should apply opacity to unavailable tracks', () => {
      const tracks = [
        {
          id: 1,
          title: 'Missing Track',
          artist: 'Artist',
          isAvailable: false,
        },
      ];
      const { container } = renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      const row = container.querySelector('.opacity-60');
      expect(row).toBeInTheDocument();
    });

    it('should apply strikethrough to unavailable track titles', () => {
      const tracks = [
        {
          id: 1,
          title: 'Missing Track',
          artist: 'Artist',
          isAvailable: false,
        },
      ];
      const { container } = renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      const strikethroughTitle = container.querySelector('.line-through');
      expect(strikethroughTitle).toBeInTheDocument();
      expect(strikethroughTitle?.textContent).toBe('Missing Track');
    });

    it('should show track number for available tracks', () => {
      const tracks = [
        {
          id: 1,
          title: 'Available Track',
          artist: 'Artist',
          trackNumber: 5,
          isAvailable: true,
        },
      ];
      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      expect(screen.getByText('5')).toBeInTheDocument();
    });
  });

  describe('duration formatting', () => {
    it('should format duration correctly', () => {
      const tracks = [
        { id: 1, title: 'Track', artist: 'Artist', duration: 240 },
      ];
      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      expect(screen.getByText('4:00')).toBeInTheDocument();
    });

    it('should show --:-- for missing duration', () => {
      const tracks = [{ id: 1, title: 'Track', artist: 'Artist' }];
      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      expect(screen.getByText('--:--')).toBeInTheDocument();
    });

    it('should pad seconds with leading zero', () => {
      const tracks = [
        { id: 1, title: 'Track', artist: 'Artist', duration: 65 },
      ];
      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      expect(screen.getByText('1:05')).toBeInTheDocument();
    });

    it('should handle long durations', () => {
      const tracks = [
        { id: 1, title: 'Track', artist: 'Artist', duration: 3661 }, // 1h 1m 1s
      ];
      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      expect(screen.getByText('61:01')).toBeInTheDocument();
    });
  });

  describe('play/pause interaction', () => {
    it('should call playQueue when track is double-clicked', async () => {
      const tracks = createSampleTracks();
      const mockContext = createMockContext();

      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />,
        mockContext
      );

      // Find the track row by finding the title and getting its parent row
      const trackTitle = screen.getByText('Lossless Track');
      const firstTrackRow = trackTitle.closest('.group');
      expect(firstTrackRow).toBeInTheDocument();

      fireEvent.doubleClick(firstTrackRow!);

      await waitFor(() => {
        expect(mockContext.commands.playQueue).toHaveBeenCalled();
      });
    });

    it('should not play unavailable tracks on double-click', async () => {
      const tracks = [
        {
          id: 1,
          title: 'Missing Track',
          artist: 'Artist',
          isAvailable: false,
        },
      ];
      const mockContext = createMockContext();

      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />,
        mockContext
      );

      const trackTitle = screen.getByText('Missing Track');
      const trackRow = trackTitle.closest('.group');
      fireEvent.doubleClick(trackRow!);

      // Should not call playQueue
      expect(mockContext.commands.playQueue).not.toHaveBeenCalled();
    });

    it('should show play button on hover', async () => {
      const tracks = [{ id: 1, title: 'Track', artist: 'Artist' }];
      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      const trackTitle = screen.getByText('Track');
      const trackRow = trackTitle.closest('.group');
      fireEvent.mouseEnter(trackRow!);

      await waitFor(() => {
        expect(screen.getByLabelText('Play')).toBeInTheDocument();
      });
    });

    it('should show pause button for currently playing track', async () => {
      const tracks = [{ id: 1, title: 'Track', artist: 'Artist' }];

      // Set the current track as playing
      usePlayerStore.setState({
        currentTrack: { id: 1, title: 'Track', artist: 'Artist', album: '', filePath: '' },
        isPlaying: true,
      });

      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      expect(screen.getByLabelText('Pause')).toBeInTheDocument();
    });

    it('should call pausePlayback when pause button is clicked', async () => {
      const tracks = [{ id: 1, title: 'Track', artist: 'Artist' }];
      const mockContext = createMockContext();

      usePlayerStore.setState({
        currentTrack: { id: 1, title: 'Track', artist: 'Artist', album: '', filePath: '' },
        isPlaying: true,
      });

      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />,
        mockContext
      );

      const pauseButton = screen.getByLabelText('Pause');
      fireEvent.click(pauseButton);

      await waitFor(() => {
        expect(mockContext.commands.pausePlayback).toHaveBeenCalled();
      });
    });

    it('should highlight currently playing track', () => {
      const tracks = createSampleTracks();

      usePlayerStore.setState({
        currentTrack: { id: 1, title: 'Lossless Track', artist: 'Test Artist', album: '', filePath: '' },
        isPlaying: true,
      });

      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      // Current track title should have primary styling
      const trackTitle = screen.getByText('Lossless Track');
      expect(trackTitle.className).toContain('text-primary');
    });
  });

  describe('buildQueue callback', () => {
    it('should call buildQueue with correct parameters', async () => {
      const tracks = createSampleTracks();
      const mockBuildQueue = vi.fn().mockReturnValue([]);
      const mockContext = createMockContext();

      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={mockBuildQueue} />,
        mockContext
      );

      const trackTitle = screen.getByText('Lossless Track');
      const firstTrackRow = trackTitle.closest('.group');
      fireEvent.doubleClick(firstTrackRow!);

      await waitFor(() => {
        expect(mockBuildQueue).toHaveBeenCalledWith(
          tracks,
          tracks[0],
          0
        );
      });
    });
  });

  describe('track action callback', () => {
    it('should call onTrackAction when track is played', async () => {
      const tracks = createSampleTracks();
      const mockTrackAction = vi.fn();
      const mockContext = createMockContext();

      renderWithProvider(
        <TrackList
          tracks={tracks}
          buildQueue={defaultBuildQueue}
          onTrackAction={mockTrackAction}
        />,
        mockContext
      );

      const trackTitle = screen.getByText('Lossless Track');
      const firstTrackRow = trackTitle.closest('.group');
      fireEvent.doubleClick(firstTrackRow!);

      await waitFor(() => {
        expect(mockTrackAction).toHaveBeenCalledWith(tracks[0]);
      });
    });
  });

  describe('custom menu rendering', () => {
    it('should render custom menu for each track', () => {
      const tracks = [{ id: 1, title: 'Track', artist: 'Artist' }];
      const renderMenu = vi.fn((track: Track) => (
        <button data-testid={`menu-${track.id}`}>Menu</button>
      ));

      renderWithProvider(
        <TrackList
          tracks={tracks}
          buildQueue={defaultBuildQueue}
          renderMenu={renderMenu}
        />
      );

      expect(screen.getByTestId('menu-1')).toBeInTheDocument();
      expect(renderMenu).toHaveBeenCalledWith(tracks[0]);
    });

    it('should render menu for each track in list', () => {
      const tracks = createSampleTracks();
      const renderMenu = vi.fn((track: Track) => (
        <button data-testid={`menu-${track.id}`}>Menu</button>
      ));

      renderWithProvider(
        <TrackList
          tracks={tracks}
          buildQueue={defaultBuildQueue}
          renderMenu={renderMenu}
        />
      );

      expect(screen.getByTestId('menu-1')).toBeInTheDocument();
      expect(screen.getByTestId('menu-2')).toBeInTheDocument();
      expect(screen.getByTestId('menu-3')).toBeInTheDocument();
      expect(screen.getByTestId('menu-4')).toBeInTheDocument();
      expect(screen.getByTestId('menu-5')).toBeInTheDocument();
    });
  });

  describe('mixed source and format scenarios', () => {
    it('should render tracks with various source and format combinations', () => {
      const tracks: Track[] = [
        // Local FLAC
        {
          id: 1,
          title: 'Local FLAC',
          artist: 'Artist',
          format: 'flac',
          sourceType: 'local',
        },
        // Server MP3
        {
          id: 2,
          title: 'Server MP3',
          artist: 'Artist',
          format: 'mp3',
          bitrate: 320,
          sourceType: 'server',
          sourceOnline: true,
        },
        // Cached Hi-Res
        {
          id: 3,
          title: 'Cached Hi-Res',
          artist: 'Artist',
          format: 'flac',
          sampleRate: 192000,
          sourceType: 'cached',
        },
        // Server offline
        {
          id: 4,
          title: 'Offline Server',
          artist: 'Artist',
          format: 'aac',
          bitrate: 256,
          sourceType: 'server',
          sourceOnline: false,
        },
      ];

      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      // All titles should be present
      expect(screen.getByText('Local FLAC')).toBeInTheDocument();
      expect(screen.getByText('Server MP3')).toBeInTheDocument();
      expect(screen.getByText('Cached Hi-Res')).toBeInTheDocument();
      expect(screen.getByText('Offline Server')).toBeInTheDocument();

      // Format badges
      expect(screen.getByText('FLAC')).toBeInTheDocument();
      expect(screen.getByText('MP3 320')).toBeInTheDocument();
      expect(screen.getByText('Hi-Res 192kHz')).toBeInTheDocument();
      expect(screen.getByText('AAC 256')).toBeInTheDocument();

      // Source indicators
      expect(screen.getByTitle('Local')).toBeInTheDocument();
      expect(screen.getByTitle('Server (Online)')).toBeInTheDocument();
      expect(screen.getByTitle('Cached')).toBeInTheDocument();
      expect(screen.getByTitle('Server (Offline)')).toBeInTheDocument();
    });

    it('should handle tracks with partial information', () => {
      const tracks: Track[] = [
        // Only format, no source
        { id: 1, title: 'Format Only', artist: 'Artist', format: 'mp3' },
        // Only source, no format
        { id: 2, title: 'Source Only', artist: 'Artist', sourceType: 'local' },
        // Neither format nor source
        { id: 3, title: 'Minimal', artist: 'Artist' },
      ];

      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      // All titles visible
      expect(screen.getByText('Format Only')).toBeInTheDocument();
      expect(screen.getByText('Source Only')).toBeInTheDocument();
      expect(screen.getByText('Minimal')).toBeInTheDocument();

      // MP3 badge visible
      expect(screen.getByText('MP3')).toBeInTheDocument();

      // Local indicator visible
      expect(screen.getByTitle('Local')).toBeInTheDocument();

      // Dashes for missing data
      const dashes = screen.getAllByText('—');
      expect(dashes.length).toBeGreaterThanOrEqual(4); // Multiple columns with dashes
    });
  });

  describe('performance', () => {
    it('should render large track lists efficiently', () => {
      const tracks: Track[] = Array.from({ length: 100 }, (_, i) => ({
        id: i + 1,
        title: `Track ${i + 1}`,
        artist: `Artist ${i + 1}`,
        album: `Album ${Math.floor(i / 10) + 1}`,
        duration: 180 + (i % 120),
        trackNumber: (i % 12) + 1,
        format: ['flac', 'mp3', 'aac', 'wav'][i % 4],
        bitrate: i % 4 === 1 ? 320 : undefined,
        sourceType: ['local', 'server', 'cached'][i % 3] as SourceType,
        sourceOnline: i % 3 === 1 ? i % 2 === 0 : undefined,
      }));

      const start = performance.now();

      const { unmount } = renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      const duration = performance.now() - start;

      // Should render 100 tracks in under 500ms
      expect(duration).toBeLessThan(500);

      // Verify first and last track rendered
      expect(screen.getByText('Track 1')).toBeInTheDocument();
      expect(screen.getByText('Track 100')).toBeInTheDocument();

      unmount();
    });
  });

  describe('edge cases', () => {
    it('should handle track with string id', () => {
      const tracks = [
        { id: 'uuid-1234-5678', title: 'String ID Track', artist: 'Artist' },
      ];
      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      expect(screen.getByText('String ID Track')).toBeInTheDocument();
    });

    it('should handle track with numeric id', () => {
      const tracks = [
        { id: 12345, title: 'Numeric ID Track', artist: 'Artist' },
      ];
      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      expect(screen.getByText('Numeric ID Track')).toBeInTheDocument();
    });

    it('should handle special characters in track names', () => {
      const tracks = [
        { id: 1, title: 'Track with <special> & "chars"', artist: 'Artist' },
      ];
      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      expect(screen.getByText('Track with <special> & "chars"')).toBeInTheDocument();
    });

    it('should handle very long track titles', () => {
      const longTitle = 'This is a very long track title that should be truncated properly in the UI to prevent layout issues';
      const tracks = [{ id: 1, title: longTitle, artist: 'Artist' }];

      const { container } = renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      // Title should be truncated with CSS
      const truncatedElement = container.querySelector('.truncate');
      expect(truncatedElement).toBeInTheDocument();
    });

    it('should handle zero duration', () => {
      const tracks = [{ id: 1, title: 'Track', artist: 'Artist', duration: 0 }];
      renderWithProvider(
        <TrackList tracks={tracks} buildQueue={defaultBuildQueue} />
      );

      // Zero duration should show --:--
      expect(screen.getByText('--:--')).toBeInTheDocument();
    });
  });
});
