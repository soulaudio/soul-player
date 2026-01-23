/**
 * Mock utilities for E2E playback tests
 * Provides HTMLAudioElement mocks and test helpers
 */

import { vi } from 'vitest';

/**
 * Mock HTMLAudioElement for testing audio playback
 * Simulates real audio behavior without actual audio files
 */
export class MockHTMLAudioElement {
  public src = '';
  public volume = 1;
  public currentTime = 0;
  public duration = 180; // 3 minutes default
  public paused = true;
  public ended = false;
  public readyState = 0;
  public networkState = 0;
  public playbackRate = 1;
  public muted = false;
  public loop = false;
  public preload = 'auto';
  public autoplay = false;
  public controls = false;

  private eventListeners: Map<string, Set<EventListener>> = new Map();
  private playInterval: NodeJS.Timeout | null = null;

  constructor() {
    // Simulate loading when src is set
    Object.defineProperty(this, 'src', {
      get: () => this._src,
      set: (value: string) => {
        this._src = value;
        if (value) {
          this.readyState = 4; // HAVE_ENOUGH_DATA
          this.networkState = 2; // NETWORK_LOADING
          this.dispatchEvent(new Event('loadedmetadata'));
          this.dispatchEvent(new Event('canplay'));
        }
      },
    });
  }

  private _src = '';

  async play(): Promise<void> {
    if (!this.src) {
      throw new Error('Cannot play without src');
    }

    this.paused = false;
    this.ended = false;
    this.dispatchEvent(new Event('play'));
    this.dispatchEvent(new Event('playing'));

    // Simulate playback progress
    this.startPlaybackSimulation();

    return Promise.resolve();
  }

  pause(): void {
    this.paused = true;
    this.stopPlaybackSimulation();
    this.dispatchEvent(new Event('pause'));
  }

  load(): void {
    this.currentTime = 0;
    this.readyState = 4;
    this.dispatchEvent(new Event('loadstart'));
    this.dispatchEvent(new Event('loadedmetadata'));
    this.dispatchEvent(new Event('canplay'));
  }

  addEventListener(type: string, listener: EventListener): void {
    if (!this.eventListeners.has(type)) {
      this.eventListeners.set(type, new Set());
    }
    this.eventListeners.get(type)!.add(listener);
  }

  removeEventListener(type: string, listener: EventListener): void {
    const listeners = this.eventListeners.get(type);
    if (listeners) {
      listeners.delete(listener);
    }
  }

  dispatchEvent(event: Event): boolean {
    const listeners = this.eventListeners.get(event.type);
    if (listeners) {
      listeners.forEach((listener) => {
        listener(event);
      });
    }
    return true;
  }

  private startPlaybackSimulation(): void {
    this.stopPlaybackSimulation();

    // Update currentTime every 100ms
    this.playInterval = setInterval(() => {
      if (!this.paused && this.currentTime < this.duration) {
        this.currentTime += 0.1;
        this.dispatchEvent(new Event('timeupdate'));

        // Check if reached end
        if (this.currentTime >= this.duration) {
          this.currentTime = this.duration;
          this.ended = true;
          this.paused = true;
          this.stopPlaybackSimulation();
          this.dispatchEvent(new Event('ended'));
        }
      }
    }, 100);
  }

  private stopPlaybackSimulation(): void {
    if (this.playInterval) {
      clearInterval(this.playInterval);
      this.playInterval = null;
    }
  }

  // Cleanup method for tests
  destroy(): void {
    this.stopPlaybackSimulation();
    this.eventListeners.clear();
  }
}

/**
 * Setup mock HTMLAudioElement globally for tests
 */
export function setupAudioMocks() {
  // Store original Audio constructor
  const OriginalAudio = globalThis.Audio;

  // Mock Audio constructor
  globalThis.Audio = vi.fn().mockImplementation(() => {
    return new MockHTMLAudioElement();
  }) as any;

  // Mock HTMLAudioElement
  globalThis.HTMLAudioElement = MockHTMLAudioElement as any;

  // Cleanup function
  return () => {
    globalThis.Audio = OriginalAudio;
    globalThis.HTMLAudioElement = HTMLAudioElement;
  };
}

/**
 * Get the most recent Audio instance created
 */
export function getMostRecentAudioElement(): MockHTMLAudioElement | null {
  const audioMock = globalThis.Audio as any;
  if (audioMock?.mock?.results?.length > 0) {
    const lastCall = audioMock.mock.results[audioMock.mock.results.length - 1];
    return lastCall.value;
  }
  return null;
}

/**
 * Wait for audio to reach playing state
 */
export async function waitForAudioPlaying(
  audioElement: MockHTMLAudioElement,
  timeout = 1000
): Promise<void> {
  const startTime = Date.now();

  while (audioElement.paused && Date.now() - startTime < timeout) {
    await new Promise((resolve) => setTimeout(resolve, 50));
  }

  if (audioElement.paused) {
    throw new Error('Audio did not start playing within timeout');
  }
}

/**
 * Wait for audio to reach paused state
 */
export async function waitForAudioPaused(
  audioElement: MockHTMLAudioElement,
  timeout = 1000
): Promise<void> {
  const startTime = Date.now();

  while (!audioElement.paused && Date.now() - startTime < timeout) {
    await new Promise((resolve) => setTimeout(resolve, 50));
  }

  if (!audioElement.paused) {
    throw new Error('Audio did not pause within timeout');
  }
}

/**
 * Simulate audio ending
 */
export function simulateAudioEnd(audioElement: MockHTMLAudioElement): void {
  audioElement.currentTime = audioElement.duration;
  audioElement.ended = true;
  audioElement.paused = true;
  audioElement.dispatchEvent(new Event('ended'));
}

/**
 * Simulate audio time update
 */
export function simulateTimeUpdate(audioElement: MockHTMLAudioElement, time: number): void {
  audioElement.currentTime = Math.min(time, audioElement.duration);
  audioElement.dispatchEvent(new Event('timeupdate'));
}
