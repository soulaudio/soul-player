/**
 * Comprehensive tests for AlbumPage race condition handling
 * Tests request ID tracking, stale response prevention, and rapid navigation
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, act } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { AlbumPage } from '../AlbumPage';
import { BackendContext, BackendInterface, BackendTrack, BackendAlbum } from '../../contexts/BackendContext';
import { PlayerCommandsProvider, PlayerContextValue } from '../../contexts/PlayerCommandsContext';
import { PlatformProvider } from '../../contexts/PlatformContext';

// Mock useNavigateWithHistory hook
vi.mock('../../hooks/useNavigateWithHistory', () => ({
  useNavigateWithHistory: vi.fn(() => ({
    navigate: vi.fn(),
    goBack: vi.fn(),
    hasHistory: true,
  })),
}));

// Helper to create mock backend
const createMockBackend = (): Partial<BackendInterface> => ({
  // Album operations
  getAlbumById: vi.fn(),
  getAlbumTracks: vi.fn(),
  getAllAlbums: vi.fn(),

  // Artist operations
  getArtistById: vi.fn(),
  getAllArtists: vi.fn(),
  getArtistAlbums: vi.fn(),

  // Track operations
  getAllTracks: vi.fn(),
  deleteTrack: vi.fn(),

  // Playlist operations
  getAllPlaylists: vi.fn(),
  getPlaylistById: vi.fn(),
  getPlaylistTracks: vi.fn(),
  createPlaylist: vi.fn(),
  deletePlaylist: vi.fn(),
  addTrackToPlaylist: vi.fn(),
  removeTrackFromPlaylist: vi.fn(),

  // Context operations
  recordContext: vi.fn(),
  getRecentContexts: vi.fn(),

  // Health check
  checkDatabaseHealth: vi.fn(),

  // Version
  getVersion: vi.fn(),

  // Settings
  getUserSetting: vi.fn(),
  setUserSetting: vi.fn(),
});

// Helper to create mock player commands
const createMockPlayerCommands = (): PlayerContextValue => ({
  commands: {
    playTrack: vi.fn().mockResolvedValue(undefined),
    pausePlayback: vi.fn().mockResolvedValue(undefined),
    resumePlayback: vi.fn().mockResolvedValue(undefined),
    stopPlayback: vi.fn().mockResolvedValue(undefined),
    skipNext: vi.fn().mockResolvedValue(undefined),
    skipPrevious: vi.fn().mockResolvedValue(undefined),
    seek: vi.fn().mockResolvedValue(undefined),
    setVolume: vi.fn().mockResolvedValue(undefined),
    setShuffle: vi.fn().mockResolvedValue(undefined),
    cycleShuffle: vi.fn().mockResolvedValue('off' as const),
    getShuffle: vi.fn().mockResolvedValue('off' as const),
    setRepeatMode: vi.fn().mockResolvedValue(undefined),
    getPlaybackCapabilities: vi.fn().mockResolvedValue({ hasNext: true, hasPrevious: true }),
    getQueue: vi.fn().mockResolvedValue([]),
    playQueue: vi.fn().mockResolvedValue(undefined),
    playQueueWithContext: vi.fn().mockResolvedValue(undefined),
    skipToQueueIndex: vi.fn().mockResolvedValue(undefined),
    addPlayNext: vi.fn().mockResolvedValue(undefined),
    addToQueueEnd: vi.fn().mockResolvedValue(undefined),
    clearPlayNext: vi.fn().mockResolvedValue(undefined),
    clearAddToQueue: vi.fn().mockResolvedValue(undefined),
    getAllSources: vi.fn().mockResolvedValue([]),
  },
  events: {
    onStateChange: vi.fn(() => () => {}),
    onTrackChange: vi.fn(() => () => {}),
    onPositionUpdate: vi.fn(() => () => {}),
    onVolumeChange: vi.fn(() => () => {}),
    onQueueUpdate: vi.fn(() => () => {}),
    onError: vi.fn(() => () => {}),
  },
});

// Helper to render AlbumPage with all providers
const renderAlbumPage = (
  albumId: string,
  backend: BackendInterface = createMockBackend(),
  playerCommands: PlayerContextValue = createMockPlayerCommands()
) => {
  return render(
    <MemoryRouter initialEntries={[`/albums/${albumId}`]}>
      <BackendContext.Provider value={backend}>
        <PlayerCommandsProvider value={playerCommands}>
          <PlatformProvider platform="desktop">
            <Routes>
              <Route path="/albums/:id" element={<AlbumPage />} />
            </Routes>
          </PlatformProvider>
        </PlayerCommandsProvider>
      </BackendContext.Provider>
    </MemoryRouter>
  );
};

// Sample album data
const createSampleAlbum = (id: number): BackendAlbum => ({
  id,
  title: `Album ${id}`,
  artist_name: `Artist ${id}`,
  artist_id: id * 10,
  year: 2020 + id,
  cover_art_path: `/covers/album${id}.jpg`,
  track_count: 10,
});

// Sample track data
const createSampleTracks = (albumId: number, count: number = 3): BackendTrack[] => {
  return Array.from({ length: count }, (_, i) => ({
    id: albumId * 100 + i + 1,
    title: `Track ${i + 1}`,
    artist_name: `Artist ${albumId}`,
    artist_id: albumId * 10,
    album_title: `Album ${albumId}`,
    album_id: albumId,
    track_number: i + 1,
    duration_seconds: 180 + i * 10,
    file_path: `/music/album${albumId}/track${i + 1}.flac`,
    file_format: 'flac',
    bit_rate: null,
    sample_rate: 44100,
    channels: 2,
    cover_art_path: null,
  }));
};

describe('AlbumPage - Race Condition Handling', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('basic rendering', () => {
    it('should render album details when data loads successfully', async () => {
      const mockBackend = createMockBackend() as any;
      const album = createSampleAlbum(1);
      const tracks = createSampleTracks(1);

      mockBackend.getAlbumById = vi.fn().mockResolvedValue(album);
      mockBackend.getAlbumTracks = vi.fn().mockResolvedValue(tracks);

      renderAlbumPage('1', mockBackend);

      // Should show loading state initially
      expect(screen.getByText('Loading...')).toBeInTheDocument();

      // Wait for data to load
      await waitFor(() => {
        expect(screen.getByText('Album 1')).toBeInTheDocument();
      });

      expect(screen.getByText('Artist 1')).toBeInTheDocument();
      expect(screen.getByText('Track 1')).toBeInTheDocument();
      expect(screen.getByText('Track 2')).toBeInTheDocument();
      expect(screen.getByText('Track 3')).toBeInTheDocument();
    });

    it('should show error state when album not found', async () => {
      const mockBackend = createMockBackend() as any;
      mockBackend.getAlbumById = vi.fn().mockResolvedValue(null);

      renderAlbumPage('999', mockBackend);

      await waitFor(() => {
        expect(screen.getByText(/not found/i)).toBeInTheDocument();
      });
    });

    it('should show error state when loading fails', async () => {
      const mockBackend = createMockBackend() as any;
      mockBackend.getAlbumById = vi.fn().mockRejectedValue(new Error('Network error'));

      renderAlbumPage('1', mockBackend);

      await waitFor(() => {
        expect(screen.getByText(/failed to load/i)).toBeInTheDocument();
      });
    });
  });

  describe('race condition prevention', () => {
    it('should ignore stale responses when navigating rapidly', async () => {
      const mockBackend = createMockBackend() as any;
      const album1 = createSampleAlbum(1);
      const album2 = createSampleAlbum(2);
      const tracks1 = createSampleTracks(1);
      const tracks2 = createSampleTracks(2);

      let resolveAlbum1: (value: BackendAlbum) => void;
      let resolveAlbum2: (value: BackendAlbum) => void;
      let resolveTracks1: (value: BackendTrack[]) => void;
      let resolveTracks2: (value: BackendTrack[]) => void;

      const album1Promise = new Promise<BackendAlbum>((resolve) => {
        resolveAlbum1 = resolve;
      });
      const album2Promise = new Promise<BackendAlbum>((resolve) => {
        resolveAlbum2 = resolve;
      });
      const tracks1Promise = new Promise<BackendTrack[]>((resolve) => {
        resolveTracks1 = resolve;
      });
      const tracks2Promise = new Promise<BackendTrack[]>((resolve) => {
        resolveTracks2 = resolve;
      });

      mockBackend.getAlbumById = vi.fn()
        .mockReturnValueOnce(album1Promise)
        .mockReturnValueOnce(album2Promise);

      mockBackend.getAlbumTracks = vi.fn()
        .mockReturnValueOnce(tracks1Promise)
        .mockReturnValueOnce(tracks2Promise);

      // Render with album ID 1
      const { rerender } = renderAlbumPage('1', mockBackend);

      // Immediately navigate to album ID 2 (before album 1 loads)
      rerender(
        <MemoryRouter initialEntries={['/albums/2']}>
          <BackendContext.Provider value={mockBackend}>
            <PlayerCommandsProvider value={createMockPlayerCommands()}>
              <PlatformProvider platform="desktop">
                <Routes>
                  <Route path="/albums/:id" element={<AlbumPage />} />
                </Routes>
              </PlatformProvider>
            </PlayerCommandsProvider>
          </BackendContext.Provider>
        </MemoryRouter>
      );

      // Now resolve album 2 FIRST (newer request completes first)
      act(() => {
        resolveAlbum2!(album2);
        resolveTracks2!(tracks2);
      });

      await waitFor(() => {
        expect(screen.getByText('Album 2')).toBeInTheDocument();
      });

      // Then resolve album 1 (older request completes second - should be ignored)
      act(() => {
        resolveAlbum1!(album1);
        resolveTracks1!(tracks1);
      });

      // Wait a bit to ensure stale update doesn't happen
      await new Promise(resolve => setTimeout(resolve, 50));

      // Should still show Album 2 (not reverted to Album 1)
      expect(screen.getByText('Album 2')).toBeInTheDocument();
      expect(screen.queryByText('Album 1')).not.toBeInTheDocument();
      expect(screen.getByText('Artist 2')).toBeInTheDocument();
    });

    it('should handle multiple rapid navigations correctly', async () => {
      const mockBackend = createMockBackend() as any;

      // Create 3 albums
      const albums = [1, 2, 3].map(createSampleAlbum);
      const allTracks = [1, 2, 3].map(id => createSampleTracks(id));

      let resolvers: Array<{
        album: (value: BackendAlbum) => void;
        tracks: (value: BackendTrack[]) => void;
      }> = [];

      // Set up mock to capture resolvers
      mockBackend.getAlbumById = vi.fn((id: number) => {
        return new Promise<BackendAlbum>((resolve) => {
          if (!resolvers[id - 1]) resolvers[id - 1] = {} as any;
          resolvers[id - 1].album = resolve;
        });
      });

      mockBackend.getAlbumTracks = vi.fn((id: number) => {
        return new Promise<BackendTrack[]>((resolve) => {
          if (!resolvers[id - 1]) resolvers[id - 1] = {} as any;
          resolvers[id - 1].tracks = resolve;
        });
      });

      // Render album 1
      const { rerender } = renderAlbumPage('1', mockBackend);

      // Rapidly navigate: 1 → 2 → 3
      rerender(
        <MemoryRouter initialEntries={['/albums/2']}>
          <BackendContext.Provider value={mockBackend}>
            <PlayerCommandsProvider value={createMockPlayerCommands()}>
              <PlatformProvider platform="desktop">
                <Routes>
                  <Route path="/albums/:id" element={<AlbumPage />} />
                </Routes>
              </PlatformProvider>
            </PlayerCommandsProvider>
          </BackendContext.Provider>
        </MemoryRouter>
      );

      rerender(
        <MemoryRouter initialEntries={['/albums/3']}>
          <BackendContext.Provider value={mockBackend}>
            <PlayerCommandsProvider value={createMockPlayerCommands()}>
              <PlatformProvider platform="desktop">
                <Routes>
                  <Route path="/albums/:id" element={<AlbumPage />} />
                </Routes>
              </PlatformProvider>
            </PlayerCommandsProvider>
          </BackendContext.Provider>
        </MemoryRouter>
      );

      // Resolve in order: 2, 1, 3 (out of request order)
      act(() => {
        resolvers[1].album(albums[1]);
        resolvers[1].tracks(allTracks[1]);
      });

      await new Promise(resolve => setTimeout(resolve, 10));

      act(() => {
        resolvers[0].album(albums[0]);
        resolvers[0].tracks(allTracks[0]);
      });

      await new Promise(resolve => setTimeout(resolve, 10));

      act(() => {
        resolvers[2].album(albums[2]);
        resolvers[2].tracks(allTracks[2]);
      });

      // Should show Album 3 (latest navigation)
      await waitFor(() => {
        expect(screen.getByText('Album 3')).toBeInTheDocument();
      });

      expect(screen.queryByText('Album 1')).not.toBeInTheDocument();
      expect(screen.queryByText('Album 2')).not.toBeInTheDocument();
    });
  });

  describe('error handling with race conditions', () => {
    it('should ignore errors from stale requests', async () => {
      const mockBackend = createMockBackend() as any;
      const album2 = createSampleAlbum(2);
      const tracks2 = createSampleTracks(2);

      let rejectAlbum1: (error: Error) => void;
      let resolveAlbum2: (value: BackendAlbum) => void;
      let resolveTracks2: (value: BackendTrack[]) => void;

      const album1Promise = new Promise<BackendAlbum>((_resolve, reject) => {
        rejectAlbum1 = reject;
      });
      const album2Promise = new Promise<BackendAlbum>((resolve) => {
        resolveAlbum2 = resolve;
      });
      const tracks2Promise = new Promise<BackendTrack[]>((resolve) => {
        resolveTracks2 = resolve;
      });

      mockBackend.getAlbumById = vi.fn()
        .mockReturnValueOnce(album1Promise)
        .mockReturnValueOnce(album2Promise);

      mockBackend.getAlbumTracks = vi.fn()
        .mockReturnValueOnce(new Promise(() => {})) // Never resolves
        .mockReturnValueOnce(tracks2Promise);

      // Render album 1
      const { rerender } = renderAlbumPage('1', mockBackend);

      // Navigate to album 2
      rerender(
        <MemoryRouter initialEntries={['/albums/2']}>
          <BackendContext.Provider value={mockBackend}>
            <PlayerCommandsProvider value={createMockPlayerCommands()}>
              <PlatformProvider platform="desktop">
                <Routes>
                  <Route path="/albums/:id" element={<AlbumPage />} />
                </Routes>
              </PlatformProvider>
            </PlayerCommandsProvider>
          </BackendContext.Provider>
        </MemoryRouter>
      );

      // Resolve album 2 successfully
      act(() => {
        resolveAlbum2!(album2);
        resolveTracks2!(tracks2);
      });

      await waitFor(() => {
        expect(screen.getByText('Album 2')).toBeInTheDocument();
      });

      // Reject album 1 (stale error)
      act(() => {
        rejectAlbum1!(new Error('Album 1 failed'));
      });

      // Wait to ensure error doesn't show
      await new Promise(resolve => setTimeout(resolve, 50));

      // Should still show Album 2 (not error state)
      expect(screen.getByText('Album 2')).toBeInTheDocument();
      expect(screen.queryByText(/failed/i)).not.toBeInTheDocument();
    });
  });

  describe('loading state management', () => {
    it('should clear loading state only for current request', async () => {
      const mockBackend = createMockBackend() as any;
      const album2 = createSampleAlbum(2);
      const tracks2 = createSampleTracks(2);

      let resolveAlbum2: (value: BackendAlbum) => void;
      let resolveTracks2: (value: BackendTrack[]) => void;

      mockBackend.getAlbumById = vi.fn()
        .mockReturnValueOnce(new Promise(() => {})) // Album 1 never resolves
        .mockReturnValueOnce(new Promise<BackendAlbum>((resolve) => {
          resolveAlbum2 = resolve;
        }));

      mockBackend.getAlbumTracks = vi.fn()
        .mockReturnValueOnce(new Promise(() => {}))
        .mockReturnValueOnce(new Promise<BackendTrack[]>((resolve) => {
          resolveTracks2 = resolve;
        }));

      // Render album 1
      const { rerender } = renderAlbumPage('1', mockBackend);

      // Should show loading
      expect(screen.getByText('Loading...')).toBeInTheDocument();

      // Navigate to album 2
      rerender(
        <MemoryRouter initialEntries={['/albums/2']}>
          <BackendContext.Provider value={mockBackend}>
            <PlayerCommandsProvider value={createMockPlayerCommands()}>
              <PlatformProvider platform="desktop">
                <Routes>
                  <Route path="/albums/:id" element={<AlbumPage />} />
                </Routes>
              </PlatformProvider>
            </PlayerCommandsProvider>
          </BackendContext.Provider>
        </MemoryRouter>
      );

      // Resolve album 2
      act(() => {
        resolveAlbum2!(album2);
        resolveTracks2!(tracks2);
      });

      // Loading should be cleared
      await waitFor(() => {
        expect(screen.queryByText('Loading...')).not.toBeInTheDocument();
      });

      expect(screen.getByText('Album 2')).toBeInTheDocument();
    });
  });

  describe('dependency array correctness', () => {
    it('should reload album when ID changes', async () => {
      const mockBackend = createMockBackend() as any;
      const album1 = createSampleAlbum(1);
      const album2 = createSampleAlbum(2);
      const tracks1 = createSampleTracks(1);
      const tracks2 = createSampleTracks(2);

      mockBackend.getAlbumById = vi.fn()
        .mockResolvedValueOnce(album1)
        .mockResolvedValueOnce(album2);

      mockBackend.getAlbumTracks = vi.fn()
        .mockResolvedValueOnce(tracks1)
        .mockResolvedValueOnce(tracks2);

      // Render album 1
      const { rerender } = renderAlbumPage('1', mockBackend);

      await waitFor(() => {
        expect(screen.getByText('Album 1')).toBeInTheDocument();
      });

      // Navigate to album 2
      rerender(
        <MemoryRouter initialEntries={['/albums/2']}>
          <BackendContext.Provider value={mockBackend}>
            <PlayerCommandsProvider value={createMockPlayerCommands()}>
              <PlatformProvider platform="desktop">
                <Routes>
                  <Route path="/albums/:id" element={<AlbumPage />} />
                </Routes>
              </PlatformProvider>
            </PlayerCommandsProvider>
          </BackendContext.Provider>
        </MemoryRouter>
      );

      await waitFor(() => {
        expect(screen.getByText('Album 2')).toBeInTheDocument();
      });

      // Both getAlbumById calls should have been made
      expect(mockBackend.getAlbumById).toHaveBeenCalledTimes(2);
      expect(mockBackend.getAlbumTracks).toHaveBeenCalledTimes(2);
    });
  });
});
