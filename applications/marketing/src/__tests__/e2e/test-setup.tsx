/**
 * Test setup utilities for E2E playback tests
 * Provides helpers for rendering the full demo app with all providers
 */

import { render, RenderResult, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { vi, expect } from 'vitest';
import { DemoApp } from '@/components/demo/DemoApp';
import { DemoStorage, DemoData } from '@soul-player/shared';
import { setupAudioMocks } from './mocks';

/**
 * Sample demo data for testing
 */
export function createSampleDemoData(): DemoData {
  const tracks = [
    {
      id: '1',
      title: 'Sample Track 1',
      artist: 'Test Artist',
      album: 'Test Album',
      duration: 180,
      trackNumber: 1,
      path: '/audio/track1.mp3',
      coverUrl: '/covers/album1.jpg',
    },
    {
      id: '2',
      title: 'Sample Track 2',
      artist: 'Test Artist',
      album: 'Test Album',
      duration: 200,
      trackNumber: 2,
      path: '/audio/track2.mp3',
      coverUrl: '/covers/album1.jpg',
    },
    {
      id: '3',
      title: 'Sample Track 3',
      artist: 'Test Artist',
      album: 'Test Album',
      duration: 220,
      trackNumber: 3,
      path: '/audio/track3.mp3',
      coverUrl: '/covers/album1.jpg',
    },
    {
      id: '4',
      title: 'Another Track',
      artist: 'Another Artist',
      album: 'Another Album',
      duration: 190,
      trackNumber: 1,
      path: '/audio/track4.mp3',
      coverUrl: '/covers/album2.jpg',
    },
    {
      id: '5',
      title: 'Track Five',
      artist: 'Another Artist',
      album: 'Another Album',
      duration: 210,
      trackNumber: 2,
      path: '/audio/track5.mp3',
      coverUrl: '/covers/album2.jpg',
    },
  ];

  const albums = [
    {
      id: '1',
      title: 'Test Album',
      artist: 'Test Artist',
      year: 2023,
      trackIds: ['1', '2', '3'],
      coverUrl: '/covers/album1.jpg',
    },
    {
      id: '2',
      title: 'Another Album',
      artist: 'Another Artist',
      year: 2024,
      trackIds: ['4', '5'],
      coverUrl: '/covers/album2.jpg',
    },
  ];

  const playlists = [
    {
      id: '1',
      name: 'My Playlist',
      description: 'A test playlist',
      trackIds: ['1', '3', '5'],
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    },
  ];

  return { tracks, albums, playlists };
}

/**
 * Create a large demo dataset for performance testing
 */
export function createLargeDemoData(trackCount: number = 100): DemoData {
  const tracks = Array.from({ length: trackCount }, (_, i) => ({
    id: String(i + 1),
    title: `Track ${i + 1}`,
    artist: `Artist ${Math.floor(i / 10) + 1}`,
    album: `Album ${Math.floor(i / 5) + 1}`,
    duration: 180 + (i % 60),
    trackNumber: (i % 10) + 1,
    path: `/audio/track${i + 1}.mp3`,
    coverUrl: `/covers/album${Math.floor(i / 5) + 1}.jpg`,
  }));

  const albumCount = Math.ceil(trackCount / 5);
  const albums = Array.from({ length: albumCount }, (_, i) => ({
    id: String(i + 1),
    title: `Album ${i + 1}`,
    artist: `Artist ${Math.floor(i / 2) + 1}`,
    year: 2020 + (i % 5),
    trackIds: tracks
      .filter((t) => Math.floor((parseInt(t.id) - 1) / 5) === i)
      .map((t) => t.id),
    coverUrl: `/covers/album${i + 1}.jpg`,
  }));

  return { tracks, albums, playlists: [] };
}

/**
 * Render the full DemoApp with pre-loaded data
 */
export async function renderDemoApp(demoData?: DemoData): Promise<RenderResult> {
  // Setup audio mocks
  const cleanupAudio = setupAudioMocks();

  // Create and load demo storage
  const storage = new DemoStorage();
  storage.loadFromData(demoData || createSampleDemoData());

  // Mock the singleton instance used by DemoApp
  // Note: This is a bit hacky but necessary since DemoApp creates its own storage
  vi.mock('@/components/demo/DemoApp', async () => {
    const actual = await vi.importActual('@/components/demo/DemoApp');
    return {
      ...actual,
      // We can't easily mock the internal storage, so we'll use the real component
      // and mock the fetch instead
    };
  });

  // Mock fetch to return our demo data
  global.fetch = vi.fn().mockResolvedValue({
    ok: true,
    json: async () => demoData || createSampleDemoData(),
  });

  const result = render(<DemoApp />);

  // Wait for demo to load
  await waitFor(
    () => {
      const loadingText = result.queryByText(/Loading demo/i);
      expect(loadingText).not.toBeInTheDocument();
    },
    { timeout: 3000 }
  );

  // Cleanup function
  (result as any).cleanup = () => {
    cleanupAudio();
    vi.restoreAllMocks();
  };

  return result;
}

/**
 * User event helper
 */
export function setupUser() {
  return userEvent.setup();
}

/**
 * Wait for element to appear in DOM
 */
export async function waitForElement(
  getElement: () => HTMLElement | null,
  timeout = 3000
): Promise<HTMLElement> {
  const startTime = Date.now();

  while (Date.now() - startTime < timeout) {
    const element = getElement();
    if (element) return element;
    await new Promise((resolve) => setTimeout(resolve, 50));
  }

  throw new Error('Element did not appear within timeout');
}

/**
 * Wait for WASM to initialize
 */
export async function waitForWasmInit(timeout = 5000): Promise<void> {
  const startTime = Date.now();

  // Check if WASM module is loaded
  while (Date.now() - startTime < timeout) {
    try {
      // Try to access WASM module (this is implementation-specific)
      // In real tests, we'd check for specific WASM initialization markers
      await new Promise((resolve) => setTimeout(resolve, 100));

      // For now, just wait a fixed time
      // TODO: Add proper WASM initialization detection
      if (Date.now() - startTime > 1000) {
        return;
      }
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
  }

  throw new Error('WASM did not initialize within timeout');
}

/**
 * Find album card by title
 */
export function findAlbumCard(container: HTMLElement, albumTitle: string): HTMLElement | null {
  const headings = Array.from(container.querySelectorAll('h3, h4, [role="heading"]'));
  for (const heading of headings) {
    if (heading.textContent?.includes(albumTitle)) {
      // Find parent card element
      let element = heading.parentElement;
      while (element && !element.hasAttribute('data-album-card')) {
        element = element.parentElement;
      }
      return element;
    }
  }
  return null;
}

/**
 * Find track row by title
 */
export function findTrackRow(container: HTMLElement, trackTitle: string): HTMLElement | null {
  const elements = Array.from(container.querySelectorAll('[role="row"], [data-track-row]'));
  for (const element of elements) {
    if (element.textContent?.includes(trackTitle)) {
      return element as HTMLElement;
    }
  }
  return null;
}

/**
 * Find button by aria-label or text
 */
export function findButton(
  container: HTMLElement,
  labelOrText: string | RegExp
): HTMLElement | null {
  // Try aria-label first
  const byAriaLabel = container.querySelector(`[aria-label="${labelOrText}"]`);
  if (byAriaLabel) return byAriaLabel as HTMLElement;

  // Try button text
  const buttons = Array.from(container.querySelectorAll('button'));
  for (const button of buttons) {
    if (typeof labelOrText === 'string') {
      if (button.textContent?.includes(labelOrText)) return button;
      if (button.getAttribute('aria-label')?.includes(labelOrText)) return button;
    } else {
      if (labelOrText.test(button.textContent || '')) return button;
      if (labelOrText.test(button.getAttribute('aria-label') || '')) return button;
    }
  }

  return null;
}

/**
 * Assert playback state
 */
export function assertPlaybackState(
  container: HTMLElement,
  options: {
    isPlaying?: boolean;
    currentTrack?: string;
    queueLength?: number;
  }
): void {
  if (options.isPlaying !== undefined) {
    const playPauseButton = findButton(container, /play|pause/i);
    expect(playPauseButton).toBeInTheDocument();

    if (options.isPlaying) {
      expect(
        playPauseButton?.getAttribute('aria-label')?.toLowerCase().includes('pause')
      ).toBe(true);
    } else {
      expect(
        playPauseButton?.getAttribute('aria-label')?.toLowerCase().includes('play')
      ).toBe(true);
    }
  }

  if (options.currentTrack) {
    const trackInfo = container.querySelector('[data-current-track], .track-info');
    expect(trackInfo?.textContent).toContain(options.currentTrack);
  }

  if (options.queueLength !== undefined) {
    const queueItems = container.querySelectorAll('[data-queue-item]');
    expect(queueItems).toHaveLength(options.queueLength);
  }
}

/**
 * Click play button on track
 */
export async function clickPlayOnTrack(
  user: ReturnType<typeof userEvent.setup>,
  container: HTMLElement,
  trackTitle: string
): Promise<void> {
  const trackRow = findTrackRow(container, trackTitle);
  expect(trackRow).toBeInTheDocument();

  const playButton = trackRow?.querySelector('button[aria-label*="Play"]');
  expect(playButton).toBeInTheDocument();

  await user.click(playButton!);
}

/**
 * Click album card
 */
export async function clickAlbumCard(
  user: ReturnType<typeof userEvent.setup>,
  container: HTMLElement,
  albumTitle: string
): Promise<void> {
  const albumCard = findAlbumCard(container, albumTitle);
  expect(albumCard).toBeInTheDocument();

  await user.click(albumCard!);
}

/**
 * Wait for navigation to complete
 */
export async function waitForNavigation(timeout = 1000): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 100));
  await waitFor(() => {}, { timeout });
}
