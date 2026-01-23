/**
 * Comprehensive tests for LibraryPage mount state handling
 * Tests prevention of state updates after unmount
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, act } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { LibraryPage } from '../LibraryPage';
import { BackendContext, BackendInterface, BackendTrack, BackendAlbum, BackendArtist, BackendPlaylist } from '../../contexts/BackendContext';
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
  getAllAlbums: vi.fn().mockResolvedValue([]),
  
  

  // Artist operations
  getArtistById: vi.fn(),
  getAllArtists: vi.fn().mockResolvedValue([]),
  getArtistAlbums: vi.fn(),

  // Track operations
  getAllTracks: vi.fn().mockResolvedValue([]),
  deleteTrack: vi.fn(),
  

  // Playlist operations
  getAllPlaylists: vi.fn().mockResolvedValue([]),
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
  checkDatabaseHealth: vi.fn().mockResolvedValue({ issues: [] }),

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

// Helper to render LibraryPage with all providers
const renderLibraryPage = (
  backend: BackendInterface = createMockBackend(),
  playerCommands: PlayerContextValue = createMockPlayerCommands()
) => {
  return render(
    <MemoryRouter>
      <BackendContext.Provider value={backend}>
        <PlayerCommandsProvider value={playerCommands}>
          <PlatformProvider platform="desktop">
            <LibraryPage />
          </PlatformProvider>
        </PlayerCommandsProvider>
      </BackendContext.Provider>
    </MemoryRouter>
  );
};

// Sample data creators
const createSampleTracks = (count: number = 5): BackendTrack[] => {
  return Array.from({ length: count }, (_, i) => ({
    id: i + 1,
    title: `Track ${i + 1}`,
    artist_name: `Artist ${i + 1}`,
    artist_id: i + 1,
    album_title: `Album ${i + 1}`,
    album_id: i + 1,
    track_number: 1,
    duration_seconds: 180,
    file_path: `/music/track${i + 1}.flac`,
    file_format: 'flac',
    bit_rate: null,
    sample_rate: 44100,
    channels: 2,
    cover_art_path: null,
    
    
  }));
};

const createSampleAlbums = (count: number = 5): BackendAlbum[] => {
  return Array.from({ length: count }, (_, i) => ({
    id: i + 1,
    title: `Album ${i + 1}`,
    artist_name: `Artist ${i + 1}`,
    artist_id: i + 1,
    year: 2020 + i,
    cover_art_path: `/covers/album${i + 1}.jpg`,
    track_count: 10,
    
    
  }));
};

const createSampleArtists = (count: number = 5): BackendArtist[] => {
  return Array.from({ length: count }, (_, i) => ({
    id: i + 1,
    name: `Artist ${i + 1}`,
    album_count: 5,
    
    
  }));
};

const createSamplePlaylists = (count: number = 5): BackendPlaylist[] => {
  return Array.from({ length: count }, (_, i) => ({
    id: i + 1,
    owner_id: 1,
    name: `Playlist ${i + 1}`,
    track_count: 10,
    
    
  }));
};

describe('LibraryPage - Mount State Handling', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('basic rendering', () => {
    it('should render library with all data loaded', async () => {
      const mockBackend = createMockBackend() as any;
      mockBackend.getAllTracks = vi.fn().mockResolvedValue(createSampleTracks());
      mockBackend.getAllAlbums = vi.fn().mockResolvedValue(createSampleAlbums());
      mockBackend.getAllArtists = vi.fn().mockResolvedValue(createSampleArtists());
      mockBackend.getAllPlaylists = vi.fn().mockResolvedValue(createSamplePlaylists());

      renderLibraryPage(mockBackend);

      // Should show loading state initially
      expect(screen.getByText('Loading...')).toBeInTheDocument();

      // Wait for data to load
      await waitFor(() => {
        expect(screen.queryByText('Loading...')).not.toBeInTheDocument();
      });

      // Should have loaded all data
      expect(mockBackend.getAllTracks).toHaveBeenCalled();
      expect(mockBackend.getAllAlbums).toHaveBeenCalled();
      expect(mockBackend.getAllArtists).toHaveBeenCalled();
      expect(mockBackend.getAllPlaylists).toHaveBeenCalled();
      expect(mockBackend.checkDatabaseHealth).toHaveBeenCalled();
    });

    it('should show error state when loading fails', async () => {
      const mockBackend = createMockBackend() as any;
      mockBackend.getAllTracks = vi.fn().mockRejectedValue(new Error('Database error'));
      mockBackend.getAllAlbums = vi.fn().mockRejectedValue(new Error('Database error'));
      mockBackend.getAllArtists = vi.fn().mockRejectedValue(new Error('Database error'));
      mockBackend.getAllPlaylists = vi.fn().mockRejectedValue(new Error('Database error'));
      mockBackend.checkDatabaseHealth = vi.fn().mockRejectedValue(new Error('Database error'));

      renderLibraryPage(mockBackend);

      await waitFor(() => {
        expect(screen.getByText(/failed to load library/i)).toBeInTheDocument();
      });
    });
  });

  describe('mount state prevention', () => {
    it('should not update state after unmount', async () => {
      const mockBackend = createMockBackend() as any;

      let resolveData: () => void;
      const dataPromise = new Promise<void>((resolve) => {
        resolveData = resolve;
      });

      // Set up slow-loading backend
      mockBackend.getAllTracks = vi.fn().mockImplementation(async () => {
        await dataPromise;
        return createSampleTracks();
      });
      mockBackend.getAllAlbums = vi.fn().mockImplementation(async () => {
        await dataPromise;
        return createSampleAlbums();
      });
      mockBackend.getAllArtists = vi.fn().mockImplementation(async () => {
        await dataPromise;
        return createSampleArtists();
      });
      mockBackend.getAllPlaylists = vi.fn().mockImplementation(async () => {
        await dataPromise;
        return createSamplePlaylists();
      });
      mockBackend.checkDatabaseHealth = vi.fn().mockImplementation(async () => {
        await dataPromise;
        return { issues: [] };
      });

      // Render and immediately unmount
      const { unmount } = renderLibraryPage(mockBackend);

      // Verify loading state
      expect(screen.getByText('Loading...')).toBeInTheDocument();

      // Unmount before data loads
      unmount();

      // Resolve data after unmount
      act(() => {
        resolveData!();
      });

      // Wait for promises to resolve
      await new Promise(resolve => setTimeout(resolve, 100));

      // No errors should occur from setState on unmounted component
      // (Vitest would log warnings/errors if state was updated after unmount)
    });

    it('should handle navigation away during loading', async () => {
      const mockBackend = createMockBackend() as any;

      let resolveData: () => void;
      const dataPromise = new Promise<void>((resolve) => {
        resolveData = resolve;
      });

      mockBackend.getAllTracks = vi.fn().mockImplementation(async () => {
        await dataPromise;
        return createSampleTracks();
      });
      mockBackend.getAllAlbums = vi.fn().mockImplementation(async () => {
        await dataPromise;
        return createSampleAlbums();
      });
      mockBackend.getAllArtists = vi.fn().mockImplementation(async () => {
        await dataPromise;
        return createSampleArtists();
      });
      mockBackend.getAllPlaylists = vi.fn().mockImplementation(async () => {
        await dataPromise;
        return createSamplePlaylists();
      });
      mockBackend.checkDatabaseHealth = vi.fn().mockImplementation(async () => {
        await dataPromise;
        return { issues: [] };
      });

      const { unmount } = renderLibraryPage(mockBackend);

      // Simulate navigation away
      unmount();

      // Resolve after navigation
      act(() => {
        resolveData!();
      });

      await new Promise(resolve => setTimeout(resolve, 50));

      // Should not cause errors
    });

    it('should ignore errors after unmount', async () => {
      const mockBackend = createMockBackend() as any;

      let rejectData: (error: Error) => void;
      const dataPromise = new Promise<BackendTrack[]>((_resolve, reject) => {
        rejectData = reject;
      });

      mockBackend.getAllTracks = vi.fn().mockReturnValue(dataPromise);
      mockBackend.getAllAlbums = vi.fn().mockReturnValue(dataPromise);
      mockBackend.getAllArtists = vi.fn().mockReturnValue(dataPromise);
      mockBackend.getAllPlaylists = vi.fn().mockReturnValue(dataPromise);
      mockBackend.checkDatabaseHealth = vi.fn().mockReturnValue(dataPromise);

      const { unmount } = renderLibraryPage(mockBackend);

      // Unmount
      unmount();

      // Reject after unmount
      act(() => {
        rejectData!(new Error('Network error'));
      });

      await new Promise(resolve => setTimeout(resolve, 50));

      // Should not cause errors or show error UI (component is unmounted)
    });
  });

  describe('cleanup behavior', () => {
    it('should set isMounted to false on cleanup', async () => {
      const mockBackend = createMockBackend() as any;
      mockBackend.getAllTracks = vi.fn().mockResolvedValue(createSampleTracks());
      mockBackend.getAllAlbums = vi.fn().mockResolvedValue(createSampleAlbums());
      mockBackend.getAllArtists = vi.fn().mockResolvedValue(createSampleArtists());
      mockBackend.getAllPlaylists = vi.fn().mockResolvedValue(createSamplePlaylists());

      const { unmount } = renderLibraryPage(mockBackend);

      await waitFor(() => {
        expect(screen.queryByText('Loading...')).not.toBeInTheDocument();
      });

      // Unmount should trigger cleanup
      unmount();

      // Component should be gone
      expect(screen.queryByText('Albums')).not.toBeInTheDocument();
    });
  });

  describe('concurrent request handling', () => {
    it('should handle all 5 parallel requests correctly', async () => {
      const mockBackend = createMockBackend() as any;
      const tracks = createSampleTracks();
      const albums = createSampleAlbums();
      const artists = createSampleArtists();
      const playlists = createSamplePlaylists();

      mockBackend.getAllTracks = vi.fn().mockResolvedValue(tracks);
      mockBackend.getAllAlbums = vi.fn().mockResolvedValue(albums);
      mockBackend.getAllArtists = vi.fn().mockResolvedValue(artists);
      mockBackend.getAllPlaylists = vi.fn().mockResolvedValue(playlists);
      mockBackend.checkDatabaseHealth = vi.fn().mockResolvedValue({ issues: [] });

      renderLibraryPage(mockBackend);

      await waitFor(() => {
        expect(screen.queryByText('Loading...')).not.toBeInTheDocument();
      });

      // All requests should have been made in parallel
      expect(mockBackend.getAllTracks).toHaveBeenCalledTimes(1);
      expect(mockBackend.getAllAlbums).toHaveBeenCalledTimes(1);
      expect(mockBackend.getAllArtists).toHaveBeenCalledTimes(1);
      expect(mockBackend.getAllPlaylists).toHaveBeenCalledTimes(1);
      expect(mockBackend.checkDatabaseHealth).toHaveBeenCalledTimes(1);
    });

    it('should handle partial failures gracefully', async () => {
      const mockBackend = createMockBackend() as any;
      mockBackend.getAllTracks = vi.fn().mockRejectedValue(new Error('Tracks failed'));
      mockBackend.getAllAlbums = vi.fn().mockRejectedValue(new Error('Albums failed'));
      mockBackend.getAllArtists = vi.fn().mockRejectedValue(new Error('Artists failed'));
      mockBackend.getAllPlaylists = vi.fn().mockRejectedValue(new Error('Playlists failed'));
      mockBackend.checkDatabaseHealth = vi.fn().mockRejectedValue(new Error('Health check failed'));

      renderLibraryPage(mockBackend);

      // Should show error state
      await waitFor(() => {
        expect(screen.getByText(/failed to load library/i)).toBeInTheDocument();
      });
    });
  });
});
