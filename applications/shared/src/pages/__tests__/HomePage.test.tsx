/**
 * Comprehensive tests for HomePage mount state handling
 * Tests prevention of state updates after unmount
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, act } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { HomePage } from '../HomePage';
import { BackendContext, BackendInterface, BackendAlbum } from '../../contexts/BackendContext';
import { ScrollVisibilityProvider } from '../../contexts/ScrollVisibilityContext';

// Helper to create mock backend
const createMockBackend = (): Partial<BackendInterface> => ({
  // Album operations
  getAlbumById: vi.fn(),
  getAlbumTracks: vi.fn(),
  getAllAlbums: vi.fn().mockResolvedValue([]),
  
  

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
  getRecentContexts: vi.fn().mockResolvedValue([]),

  // Health check
  checkDatabaseHealth: vi.fn(),

  // Version
  getVersion: vi.fn(),

  // Settings
  getUserSetting: vi.fn(),
  setUserSetting: vi.fn(),
});

// Helper to render HomePage with providers
const renderHomePage = (backend: BackendInterface = createMockBackend()) => {
  return render(
    <MemoryRouter>
      <BackendContext.Provider value={backend}>
        <ScrollVisibilityProvider>
          <HomePage />
        </ScrollVisibilityProvider>
      </BackendContext.Provider>
    </MemoryRouter>
  );
};

// Sample data
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

describe('HomePage - Mount State Handling', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('basic rendering', () => {
    it('should render home page with albums', async () => {
      const mockBackend = createMockBackend();
      mockBackend.getAllAlbums = vi.fn().mockResolvedValue(createSampleAlbums());
      mockBackend.getRecentContexts = vi.fn().mockResolvedValue([]);

      renderHomePage(mockBackend);

      await waitFor(() => {
        expect(mockBackend.getAllAlbums).toHaveBeenCalled();
        expect(mockBackend.getRecentContexts).toHaveBeenCalled();
      });
    });
  });

  describe('mount state prevention', () => {
    it('should not update state after unmount', async () => {
      const mockBackend = createMockBackend();

      let resolveData: () => void;
      const dataPromise = new Promise<void>((resolve) => {
        resolveData = resolve;
      });

      mockBackend.getAllAlbums = vi.fn().mockImplementation(async () => {
        await dataPromise;
        return createSampleAlbums();
      });
      mockBackend.getRecentContexts = vi.fn().mockImplementation(async () => {
        await dataPromise;
        return [];
      });

      const { unmount } = renderHomePage(mockBackend);

      // Unmount before data loads
      unmount();

      // Resolve data after unmount
      act(() => {
        resolveData!();
      });

      // Wait for promises to resolve
      await new Promise(resolve => setTimeout(resolve, 100));

      // No errors should occur from setState on unmounted component
    });

    it('should handle navigation away during loading', async () => {
      const mockBackend = createMockBackend();

      let resolveData: () => void;
      const dataPromise = new Promise<void>((resolve) => {
        resolveData = resolve;
      });

      mockBackend.getAllAlbums = vi.fn().mockImplementation(async () => {
        await dataPromise;
        return createSampleAlbums();
      });
      mockBackend.getRecentContexts = vi.fn().mockImplementation(async () => {
        await dataPromise;
        return [];
      });

      const { unmount } = renderHomePage(mockBackend);

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
      const mockBackend = createMockBackend();

      let rejectData: (error: Error) => void;
      const dataPromise = new Promise<BackendAlbum[]>((_resolve, reject) => {
        rejectData = reject;
      });

      mockBackend.getAllAlbums = vi.fn().mockReturnValue(dataPromise);
      mockBackend.getRecentContexts = vi.fn().mockReturnValue(dataPromise);

      const { unmount } = renderHomePage(mockBackend);

      // Unmount
      unmount();

      // Reject after unmount
      act(() => {
        rejectData!(new Error('Network error'));
      });

      await new Promise(resolve => setTimeout(resolve, 50));

      // Should not cause errors
    });
  });

  describe('concurrent request handling', () => {
    it('should handle parallel requests correctly', async () => {
      const mockBackend = createMockBackend();
      const albums = createSampleAlbums();

      mockBackend.getAllAlbums = vi.fn().mockResolvedValue(albums);
      mockBackend.getRecentContexts = vi.fn().mockResolvedValue([]);

      renderHomePage(mockBackend);

      await waitFor(() => {
        expect(mockBackend.getAllAlbums).toHaveBeenCalledTimes(1);
        expect(mockBackend.getRecentContexts).toHaveBeenCalledTimes(1);
      });
    });

    it('should handle errors gracefully', async () => {
      const mockBackend = createMockBackend();
      mockBackend.getAllAlbums = vi.fn().mockRejectedValue(new Error('Failed'));
      mockBackend.getRecentContexts = vi.fn().mockRejectedValue(new Error('Failed'));

      renderHomePage(mockBackend);

      await new Promise(resolve => setTimeout(resolve, 100));

      // Should not crash (errors are logged but not shown in UI)
      expect(mockBackend.getAllAlbums).toHaveBeenCalled();
    });
  });
});
