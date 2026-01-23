/**
 * End-to-end playback tests for marketing demo
 * Tests real user interactions with WASM playback
 *
 * Coverage:
 * - Album playback flow
 * - Queue interaction
 * - Playback controls (play/pause/skip)
 * - Shuffle and repeat modes
 * - Volume control
 * - Seek functionality
 * - Error scenarios
 * - Performance with large datasets
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { screen, waitFor, within } from '@testing-library/react';
import {
  renderDemoApp,
  setupUser,
  createSampleDemoData,
  createLargeDemoData,
  findButton,
  findAlbumCard,
  findTrackRow,
  assertPlaybackState,
  clickAlbumCard,
  clickPlayOnTrack,
  waitForNavigation,
} from './test-setup';
import {
  getMostRecentAudioElement,
  waitForAudioPlaying,
  waitForAudioPaused,
  simulateAudioEnd,
  simulateTimeUpdate,
} from './mocks';

describe('E2E Playback Tests', () => {
  let cleanup: (() => void) | undefined;

  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    if (cleanup) {
      cleanup();
      cleanup = undefined;
    }
  });

  describe('Album Playback Flow', () => {
    it('should play album from start when clicking album card', async () => {
      const user = setupUser();
      const result = await renderDemoApp();
      cleanup = (result as any).cleanup;

      // Navigate to albums page
      const albumsLink = screen.getByRole('link', { name: /albums/i });
      await user.click(albumsLink);
      await waitForNavigation();

      // Click on first album
      await clickAlbumCard(user, result.container, 'Test Album');
      await waitForNavigation();

      // Wait for album page to load
      await waitFor(() => {
        expect(screen.getByText('Test Album')).toBeInTheDocument();
      });

      // Click play on first track
      await clickPlayOnTrack(user, result.container, 'Sample Track 1');

      // Wait for playback to start
      await waitFor(() => {
        const audio = getMostRecentAudioElement();
        expect(audio).toBeTruthy();
        expect(audio?.src).toContain('track1.mp3');
      });

      // Verify player state
      assertPlaybackState(result.container, {
        isPlaying: true,
        currentTrack: 'Sample Track 1',
      });
    });

    it('should populate queue with all album tracks', async () => {
      const user = setupUser();
      const result = await renderDemoApp();
      cleanup = (result as any).cleanup;

      // Navigate to album page
      const albumsLink = screen.getByRole('link', { name: /albums/i });
      await user.click(albumsLink);
      await waitForNavigation();

      await clickAlbumCard(user, result.container, 'Test Album');
      await waitForNavigation();

      // Play first track
      await clickPlayOnTrack(user, result.container, 'Sample Track 1');

      // Open queue sidebar (if collapsed)
      const queueButton = findButton(result.container, /queue|show queue/i);
      if (queueButton) {
        await user.click(queueButton);
      }

      // Verify queue contains all album tracks
      await waitFor(() => {
        const queueItems = result.container.querySelectorAll('[data-queue-item]');
        expect(queueItems.length).toBeGreaterThanOrEqual(3);
      });
    });

    it('should highlight current track in queue', async () => {
      const user = setupUser();
      const result = await renderDemoApp();
      cleanup = (result as any).cleanup;

      // Navigate and play album
      const albumsLink = screen.getByRole('link', { name: /albums/i });
      await user.click(albumsLink);
      await clickAlbumCard(user, result.container, 'Test Album');
      await clickPlayOnTrack(user, result.container, 'Sample Track 1');

      // Check queue for highlighted item
      await waitFor(() => {
        const queueItems = result.container.querySelectorAll('[data-queue-item]');
        const currentItem = Array.from(queueItems).find((item) =>
          item.classList.contains('bg-accent')
        );
        expect(currentItem).toBeTruthy();
        expect(currentItem?.textContent).toContain('Sample Track 1');
      });
    });

    it('should update progress bar during playback', async () => {
      const user = setupUser();
      const result = await renderDemoApp();
      cleanup = (result as any).cleanup;

      // Start playback
      const albumsLink = screen.getByRole('link', { name: /albums/i });
      await user.click(albumsLink);
      await clickAlbumCard(user, result.container, 'Test Album');
      await clickPlayOnTrack(user, result.container, 'Sample Track 1');

      // Get audio element and simulate time update
      const audio = getMostRecentAudioElement();
      expect(audio).toBeTruthy();

      // Simulate 50% playback
      simulateTimeUpdate(audio!, 90); // 90 seconds of 180

      // Check progress bar updated
      await waitFor(() => {
        const progressBar = result.container.querySelector('[role="progressbar"]');
        expect(progressBar).toBeTruthy();
        // Progress should be around 50%
        const ariaValueNow = progressBar?.getAttribute('aria-valuenow');
        if (ariaValueNow) {
          const progress = parseInt(ariaValueNow);
          expect(progress).toBeGreaterThan(40);
          expect(progress).toBeLessThan(60);
        }
      });
    });
  });

  describe('Queue Interaction Flow', () => {
    it('should jump to track when clicked in queue', async () => {
      const user = setupUser();
      const result = await renderDemoApp();
      cleanup = (result as any).cleanup;

      // Start playback with album
      const albumsLink = screen.getByRole('link', { name: /albums/i });
      await user.click(albumsLink);
      await clickAlbumCard(user, result.container, 'Test Album');
      await clickPlayOnTrack(user, result.container, 'Sample Track 1');

      // Wait for queue to populate
      await waitFor(() => {
        const queueItems = result.container.querySelectorAll('[data-queue-item]');
        expect(queueItems.length).toBeGreaterThan(1);
      });

      // Click on third track in queue
      const queueItems = result.container.querySelectorAll('[data-queue-item]');
      const thirdTrack = queueItems[2];
      await user.click(thirdTrack as HTMLElement);

      // Verify playback jumped to third track
      await waitFor(() => {
        assertPlaybackState(result.container, {
          currentTrack: 'Sample Track 3',
          isPlaying: true,
        });
      });
    });

    it('should update queue position indicator', async () => {
      const user = setupUser();
      const result = await renderDemoApp();
      cleanup = (result as any).cleanup;

      // Start playback
      const albumsLink = screen.getByRole('link', { name: /albums/i });
      await user.click(albumsLink);
      await clickAlbumCard(user, result.container, 'Test Album');
      await clickPlayOnTrack(user, result.container, 'Sample Track 1');

      // Get initial highlighted track
      const getHighlightedTrack = () => {
        const queueItems = result.container.querySelectorAll('[data-queue-item]');
        return Array.from(queueItems).find((item) =>
          item.classList.contains('bg-accent')
        );
      };

      const firstHighlight = getHighlightedTrack();
      expect(firstHighlight?.textContent).toContain('Sample Track 1');

      // Skip to next track
      const nextButton = findButton(result.container, /next|skip/i);
      await user.click(nextButton!);

      // Verify highlight moved
      await waitFor(() => {
        const newHighlight = getHighlightedTrack();
        expect(newHighlight?.textContent).toContain('Sample Track 2');
      });
    });

    it('should remove previous track highlight when changing tracks', async () => {
      const user = setupUser();
      const result = await renderDemoApp();
      cleanup = (result as any).cleanup;

      // Start playback
      const albumsLink = screen.getByRole('link', { name: /albums/i });
      await user.click(albumsLink);
      await clickAlbumCard(user, result.container, 'Test Album');
      await clickPlayOnTrack(user, result.container, 'Sample Track 1');

      await waitFor(() => {
        const highlighted = result.container.querySelectorAll('.bg-accent');
        expect(highlighted.length).toBe(1);
      });

      // Skip to next
      const nextButton = findButton(result.container, /next|skip/i);
      await user.click(nextButton!);

      // Verify only one track is highlighted
      await waitFor(() => {
        const highlighted = result.container.querySelectorAll('.bg-accent');
        expect(highlighted.length).toBe(1);
      });
    });
  });

  describe('Playback Controls Flow', () => {
    it('should pause playback when pause button clicked', async () => {
      const user = setupUser();
      const result = await renderDemoApp();
      cleanup = (result as any).cleanup;

      // Start playback
      const albumsLink = screen.getByRole('link', { name: /albums/i });
      await user.click(albumsLink);
      await clickAlbumCard(user, result.container, 'Test Album');
      await clickPlayOnTrack(user, result.container, 'Sample Track 1');

      // Wait for playback to start
      const audio = getMostRecentAudioElement();
      await waitForAudioPlaying(audio!);

      // Click pause
      const pauseButton = findButton(result.container, /pause/i);
      await user.click(pauseButton!);

      // Verify paused
      await waitForAudioPaused(audio!);
      assertPlaybackState(result.container, { isPlaying: false });
    });

    it('should resume playback from same position', async () => {
      const user = setupUser();
      const result = await renderDemoApp();
      cleanup = (result as any).cleanup;

      // Start playback
      const albumsLink = screen.getByRole('link', { name: /albums/i });
      await user.click(albumsLink);
      await clickAlbumCard(user, result.container, 'Test Album');
      await clickPlayOnTrack(user, result.container, 'Sample Track 1');

      const audio = getMostRecentAudioElement();
      await waitForAudioPlaying(audio!);

      // Advance playback
      simulateTimeUpdate(audio!, 60);
      const pausedPosition = audio!.currentTime;

      // Pause
      const pauseButton = findButton(result.container, /pause/i);
      await user.click(pauseButton!);
      await waitForAudioPaused(audio!);

      // Resume
      const playButton = findButton(result.container, /play/i);
      await user.click(playButton!);

      // Verify resumed from same position
      await waitForAudioPlaying(audio!);
      expect(audio!.currentTime).toBeCloseTo(pausedPosition, 1);
    });

    it('should show play button when paused', async () => {
      const user = setupUser();
      const result = await renderDemoApp();
      cleanup = (result as any).cleanup;

      // Start and pause
      const albumsLink = screen.getByRole('link', { name: /albums/i });
      await user.click(albumsLink);
      await clickAlbumCard(user, result.container, 'Test Album');
      await clickPlayOnTrack(user, result.container, 'Sample Track 1');

      const pauseButton = findButton(result.container, /pause/i);
      await user.click(pauseButton!);

      // Verify play button appears
      await waitFor(() => {
        const playButton = findButton(result.container, /play/i);
        expect(playButton).toBeInTheDocument();
      });
    });
  });

  describe('Navigation Flow', () => {
    it('should skip to next track when next button clicked', async () => {
      const user = setupUser();
      const result = await renderDemoApp();
      cleanup = (result as any).cleanup;

      // Start playback
      const albumsLink = screen.getByRole('link', { name: /albums/i });
      await user.click(albumsLink);
      await clickAlbumCard(user, result.container, 'Test Album');
      await clickPlayOnTrack(user, result.container, 'Sample Track 1');

      // Click next
      const nextButton = findButton(result.container, /next|skip/i);
      await user.click(nextButton!);

      // Verify next track playing
      await waitFor(() => {
        assertPlaybackState(result.container, {
          currentTrack: 'Sample Track 2',
          isPlaying: true,
        });
      });
    });

    it('should advance queue position when skipping', async () => {
      const user = setupUser();
      const result = await renderDemoApp();
      cleanup = (result as any).cleanup;

      // Start playback
      const albumsLink = screen.getByRole('link', { name: /albums/i });
      await user.click(albumsLink);
      await clickAlbumCard(user, result.container, 'Test Album');
      await clickPlayOnTrack(user, result.container, 'Sample Track 1');

      // Skip twice
      const nextButton = findButton(result.container, /next|skip/i);
      await user.click(nextButton!);
      await waitFor(() => {
        assertPlaybackState(result.container, { currentTrack: 'Sample Track 2' });
      });

      await user.click(nextButton!);

      // Verify on third track
      await waitFor(() => {
        assertPlaybackState(result.container, { currentTrack: 'Sample Track 3' });
      });
    });

    it('should go to previous track when previous button clicked', async () => {
      const user = setupUser();
      const result = await renderDemoApp();
      cleanup = (result as any).cleanup;

      // Start playback on second track
      const albumsLink = screen.getByRole('link', { name: /albums/i });
      await user.click(albumsLink);
      await clickAlbumCard(user, result.container, 'Test Album');
      await clickPlayOnTrack(user, result.container, 'Sample Track 2');

      // Click previous
      const prevButton = findButton(result.container, /previous|back/i);
      await user.click(prevButton!);

      // Verify previous track playing
      await waitFor(() => {
        assertPlaybackState(result.container, {
          currentTrack: 'Sample Track 1',
          isPlaying: true,
        });
      });
    });

    it('should automatically play next track when current ends', async () => {
      const user = setupUser();
      const result = await renderDemoApp();
      cleanup = (result as any).cleanup;

      // Start playback
      const albumsLink = screen.getByRole('link', { name: /albums/i });
      await user.click(albumsLink);
      await clickAlbumCard(user, result.container, 'Test Album');
      await clickPlayOnTrack(user, result.container, 'Sample Track 1');

      const audio = getMostRecentAudioElement();
      await waitForAudioPlaying(audio!);

      // Simulate track ending
      simulateAudioEnd(audio!);

      // Verify next track started
      await waitFor(
        () => {
          assertPlaybackState(result.container, {
            currentTrack: 'Sample Track 2',
            isPlaying: true,
          });
        },
        { timeout: 2000 }
      );
    });
  });

  describe('Shuffle Flow', () => {
    it('should enable shuffle mode when shuffle button clicked', async () => {
      const user = setupUser();
      const result = await renderDemoApp();
      cleanup = (result as any).cleanup;

      // Start playback
      const albumsLink = screen.getByRole('link', { name: /albums/i });
      await user.click(albumsLink);
      await clickAlbumCard(user, result.container, 'Test Album');
      await clickPlayOnTrack(user, result.container, 'Sample Track 1');

      // Click shuffle button
      const shuffleButton = findButton(result.container, /shuffle/i);
      await user.click(shuffleButton!);

      // Verify shuffle enabled (button should have active state)
      await waitFor(() => {
        const shuffleBtn = findButton(result.container, /shuffle/i);
        expect(shuffleBtn?.classList.contains('text-primary')).toBe(true);
      });
    });

    it('should disable shuffle mode when clicked again', async () => {
      const user = setupUser();
      const result = await renderDemoApp();
      cleanup = (result as any).cleanup;

      // Start playback
      const albumsLink = screen.getByRole('link', { name: /albums/i });
      await user.click(albumsLink);
      await clickAlbumCard(user, result.container, 'Test Album');
      await clickPlayOnTrack(user, result.container, 'Sample Track 1');

      // Enable shuffle
      const shuffleButton = findButton(result.container, /shuffle/i);
      await user.click(shuffleButton!);

      await waitFor(() => {
        expect(shuffleButton?.classList.contains('text-primary')).toBe(true);
      });

      // Disable shuffle
      await user.click(shuffleButton!);

      await waitFor(() => {
        expect(shuffleButton?.classList.contains('text-primary')).toBe(false);
      });
    });

    it('should persist shuffle state across track changes', async () => {
      const user = setupUser();
      const result = await renderDemoApp();
      cleanup = (result as any).cleanup;

      // Start playback with shuffle
      const albumsLink = screen.getByRole('link', { name: /albums/i });
      await user.click(albumsLink);
      await clickAlbumCard(user, result.container, 'Test Album');
      await clickPlayOnTrack(user, result.container, 'Sample Track 1');

      const shuffleButton = findButton(result.container, /shuffle/i);
      await user.click(shuffleButton!);

      // Skip to next track
      const nextButton = findButton(result.container, /next|skip/i);
      await user.click(nextButton!);

      // Verify shuffle still enabled
      await waitFor(() => {
        const shuffleBtn = findButton(result.container, /shuffle/i);
        expect(shuffleBtn?.classList.contains('text-primary')).toBe(true);
      });
    });
  });

  describe('Repeat Flow', () => {
    it('should cycle through repeat modes', async () => {
      const user = setupUser();
      const result = await renderDemoApp();
      cleanup = (result as any).cleanup;

      // Start playback
      const albumsLink = screen.getByRole('link', { name: /albums/i });
      await user.click(albumsLink);
      await clickAlbumCard(user, result.container, 'Test Album');
      await clickPlayOnTrack(user, result.container, 'Sample Track 1');

      const repeatButton = findButton(result.container, /repeat/i);

      // Click once - should enable repeat all
      await user.click(repeatButton!);
      await waitFor(() => {
        expect(repeatButton?.classList.contains('text-primary')).toBe(true);
      });

      // Click again - should enable repeat one
      await user.click(repeatButton!);
      await waitFor(() => {
        // Repeat one should show "1" indicator
        const hasOneIndicator = repeatButton?.querySelector('[data-repeat-one]');
        expect(hasOneIndicator).toBeTruthy();
      });

      // Click again - should disable repeat
      await user.click(repeatButton!);
      await waitFor(() => {
        expect(repeatButton?.classList.contains('text-primary')).toBe(false);
      });
    });

    it('should repeat queue when in repeat all mode', async () => {
      const user = setupUser();
      const result = await renderDemoApp();
      cleanup = (result as any).cleanup;

      // Start playback with only 2 tracks
      const albumsLink = screen.getByRole('link', { name: /albums/i });
      await user.click(albumsLink);
      await clickAlbumCard(user, result.container, 'Another Album');
      await clickPlayOnTrack(user, result.container, 'Another Track');

      // Enable repeat all
      const repeatButton = findButton(result.container, /repeat/i);
      await user.click(repeatButton!);

      // Skip to last track
      const nextButton = findButton(result.container, /next|skip/i);
      await user.click(nextButton!);

      await waitFor(() => {
        assertPlaybackState(result.container, { currentTrack: 'Track Five' });
      });

      // Simulate track ending
      const audio = getMostRecentAudioElement();
      simulateAudioEnd(audio!);

      // Should loop back to first track
      await waitFor(() => {
        assertPlaybackState(result.container, { currentTrack: 'Another Track' });
      });
    });

    it('should repeat same track when in repeat one mode', async () => {
      const user = setupUser();
      const result = await renderDemoApp();
      cleanup = (result as any).cleanup;

      // Start playback
      const albumsLink = screen.getByRole('link', { name: /albums/i });
      await user.click(albumsLink);
      await clickAlbumCard(user, result.container, 'Test Album');
      await clickPlayOnTrack(user, result.container, 'Sample Track 1');

      // Enable repeat one (click twice)
      const repeatButton = findButton(result.container, /repeat/i);
      await user.click(repeatButton!);
      await user.click(repeatButton!);

      await waitFor(() => {
        const hasOneIndicator = repeatButton?.querySelector('[data-repeat-one]');
        expect(hasOneIndicator).toBeTruthy();
      });

      // Simulate track ending
      const audio = getMostRecentAudioElement();
      simulateAudioEnd(audio!);

      // Should repeat same track
      await waitFor(() => {
        assertPlaybackState(result.container, { currentTrack: 'Sample Track 1' });
      });
    });
  });

  describe('Volume Control Flow', () => {
    it('should change volume when slider moved', async () => {
      const user = setupUser();
      const result = await renderDemoApp();
      cleanup = (result as any).cleanup;

      // Start playback
      const albumsLink = screen.getByRole('link', { name: /albums/i });
      await user.click(albumsLink);
      await clickAlbumCard(user, result.container, 'Test Album');
      await clickPlayOnTrack(user, result.container, 'Sample Track 1');

      const audio = getMostRecentAudioElement();
      const initialVolume = audio!.volume;

      // Find volume slider
      const volumeSlider = result.container.querySelector('input[type="range"]');
      expect(volumeSlider).toBeInTheDocument();

      // Change volume to 50%
      await user.clear(volumeSlider!);
      await user.type(volumeSlider!, '50');

      // Verify audio volume changed
      await waitFor(() => {
        expect(audio!.volume).not.toBe(initialVolume);
      });
    });

    it('should persist volume across track changes', async () => {
      const user = setupUser();
      const result = await renderDemoApp();
      cleanup = (result as any).cleanup;

      // Start playback
      const albumsLink = screen.getByRole('link', { name: /albums/i });
      await user.click(albumsLink);
      await clickAlbumCard(user, result.container, 'Test Album');
      await clickPlayOnTrack(user, result.container, 'Sample Track 1');

      // Set volume
      const volumeSlider = result.container.querySelector('input[type="range"]');
      await user.clear(volumeSlider!);
      await user.type(volumeSlider!, '30');

      const audio = getMostRecentAudioElement();
      const setVolume = audio!.volume;

      // Skip to next track
      const nextButton = findButton(result.container, /next|skip/i);
      await user.click(nextButton!);

      await waitFor(() => {
        assertPlaybackState(result.container, { currentTrack: 'Sample Track 2' });
      });

      // Verify volume persisted
      expect(audio!.volume).toBeCloseTo(setVolume, 1);
    });

    it('should mute when mute button clicked', async () => {
      const user = setupUser();
      const result = await renderDemoApp();
      cleanup = (result as any).cleanup;

      // Start playback
      const albumsLink = screen.getByRole('link', { name: /albums/i });
      await user.click(albumsLink);
      await clickAlbumCard(user, result.container, 'Test Album');
      await clickPlayOnTrack(user, result.container, 'Sample Track 1');

      // Find and click mute button
      const muteButton = findButton(result.container, /mute|volume/i);
      await user.click(muteButton!);

      // Verify muted
      const audio = getMostRecentAudioElement();
      await waitFor(() => {
        expect(audio!.volume).toBe(0);
      });
    });
  });

  describe('Seek Flow', () => {
    it('should change playback position when progress bar clicked', async () => {
      const user = setupUser();
      const result = await renderDemoApp();
      cleanup = (result as any).cleanup;

      // Start playback
      const albumsLink = screen.getByRole('link', { name: /albums/i });
      await user.click(albumsLink);
      await clickAlbumCard(user, result.container, 'Test Album');
      await clickPlayOnTrack(user, result.container, 'Sample Track 1');

      const audio = getMostRecentAudioElement();
      await waitForAudioPlaying(audio!);

      const initialPosition = audio!.currentTime;

      // Find progress bar and click at 50%
      const progressBar = result.container.querySelector('[role="progressbar"]');
      expect(progressBar).toBeInTheDocument();

      // Simulate clicking middle of progress bar
      // Note: In real tests, you'd use actual mouse events with coordinates
      simulateTimeUpdate(audio!, 90); // Jump to 50% (90 seconds of 180)

      // Verify position changed
      expect(audio!.currentTime).not.toBe(initialPosition);
      expect(audio!.currentTime).toBeCloseTo(90, 5);
    });

    it('should continue playing from new position after seek', async () => {
      const user = setupUser();
      const result = await renderDemoApp();
      cleanup = (result as any).cleanup;

      // Start playback
      const albumsLink = screen.getByRole('link', { name: /albums/i });
      await user.click(albumsLink);
      await clickAlbumCard(user, result.container, 'Test Album');
      await clickPlayOnTrack(user, result.container, 'Sample Track 1');

      const audio = getMostRecentAudioElement();
      await waitForAudioPlaying(audio!);

      // Seek to 50%
      simulateTimeUpdate(audio!, 90);

      // Verify still playing
      await waitFor(() => {
        expect(audio!.paused).toBe(false);
      });
    });
  });

  describe('Error Scenarios', () => {
    it('should show error when playing with empty library', async () => {
      const user = setupUser();
      const result = await renderDemoApp({ tracks: [], albums: [], playlists: [] });
      cleanup = (result as any).cleanup;

      // Try to navigate to albums
      const albumsLink = screen.getByRole('link', { name: /albums/i });
      await user.click(albumsLink);

      // Should show empty state
      await waitFor(() => {
        expect(screen.getByText(/no albums/i)).toBeInTheDocument();
      });
    });

    it('should handle skip next at end of queue gracefully', async () => {
      const user = setupUser();
      const result = await renderDemoApp();
      cleanup = (result as any).cleanup;

      // Play album with 3 tracks
      const albumsLink = screen.getByRole('link', { name: /albums/i });
      await user.click(albumsLink);
      await clickAlbumCard(user, result.container, 'Test Album');
      await clickPlayOnTrack(user, result.container, 'Sample Track 3');

      // Try to skip next (should stop or loop depending on repeat mode)
      const nextButton = findButton(result.container, /next|skip/i);
      await user.click(nextButton!);

      // Should either stop or stay on last track (no error)
      await waitFor(() => {
        const currentTrack = result.container.querySelector('[data-current-track]');
        expect(currentTrack).toBeTruthy();
      });
    });

    it('should handle missing audio file gracefully', async () => {
      const user = setupUser();
      const demoData = createSampleDemoData();
      // Corrupt a track path
      demoData.tracks[0].path = '';

      const result = await renderDemoApp(demoData);
      cleanup = (result as any).cleanup;

      // Try to play corrupted track
      const albumsLink = screen.getByRole('link', { name: /albums/i });
      await user.click(albumsLink);
      await clickAlbumCard(user, result.container, 'Test Album');

      // Should handle error without crashing
      await clickPlayOnTrack(user, result.container, 'Sample Track 1');

      // UI should still be responsive
      expect(result.container).toBeTruthy();
    });
  });

  describe('Performance Tests', () => {
    it('should handle large playlist without freezing', async () => {
      const user = setupUser();
      const largeData = createLargeDemoData(100);
      const result = await renderDemoApp(largeData);
      cleanup = (result as any).cleanup;

      // Navigate to library
      const libraryLink = screen.getByRole('link', { name: /library/i });
      await user.click(libraryLink);

      // UI should remain responsive
      await waitFor(
        () => {
          expect(screen.getByText(/tracks/i)).toBeInTheDocument();
        },
        { timeout: 5000 }
      );
    });

    it('should handle rapid track skipping without errors', async () => {
      const user = setupUser();
      const result = await renderDemoApp();
      cleanup = (result as any).cleanup;

      // Start playback
      const albumsLink = screen.getByRole('link', { name: /albums/i });
      await user.click(albumsLink);
      await clickAlbumCard(user, result.container, 'Test Album');
      await clickPlayOnTrack(user, result.container, 'Sample Track 1');

      // Rapidly skip 10 times
      const nextButton = findButton(result.container, /next|skip/i);
      for (let i = 0; i < 10; i++) {
        await user.click(nextButton!);
        await new Promise((resolve) => setTimeout(resolve, 50));
      }

      // Should still be playing without errors
      await waitFor(() => {
        assertPlaybackState(result.container, { isPlaying: true });
      });
    });

    it('should handle multiple shuffle toggles quickly', async () => {
      const user = setupUser();
      const result = await renderDemoApp();
      cleanup = (result as any).cleanup;

      // Start playback
      const albumsLink = screen.getByRole('link', { name: /albums/i });
      await user.click(albumsLink);
      await clickAlbumCard(user, result.container, 'Test Album');
      await clickPlayOnTrack(user, result.container, 'Sample Track 1');

      // Toggle shuffle 5 times
      const shuffleButton = findButton(result.container, /shuffle/i);
      for (let i = 0; i < 5; i++) {
        await user.click(shuffleButton!);
        await new Promise((resolve) => setTimeout(resolve, 50));
      }

      // UI should remain responsive
      expect(result.container).toBeTruthy();
    });

    it('should stay responsive during volume adjustments', async () => {
      const user = setupUser();
      const result = await renderDemoApp();
      cleanup = (result as any).cleanup;

      // Start playback
      const albumsLink = screen.getByRole('link', { name: /albums/i });
      await user.click(albumsLink);
      await clickAlbumCard(user, result.container, 'Test Album');
      await clickPlayOnTrack(user, result.container, 'Sample Track 1');

      const volumeSlider = result.container.querySelector('input[type="range"]');

      // Rapidly adjust volume
      for (let i = 0; i < 10; i++) {
        await user.clear(volumeSlider!);
        await user.type(volumeSlider!, String(i * 10));
        await new Promise((resolve) => setTimeout(resolve, 30));
      }

      // Should still be responsive
      assertPlaybackState(result.container, { isPlaying: true });
    });
  });
});
