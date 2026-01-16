/**
 * Test helpers exposed to e2e tests
 * Only available in test/development mode
 */

import { invoke } from '@tauri-apps/api/core';

interface QueueTrack {
  trackId: string;
  title: string;
  artist: string;
  [key: string]: unknown;
}

interface TestHelpers {
  skipToQueueIndex: (index: number) => Promise<void>;
  getQueueSize: () => Promise<number>;
  getPlaybackState: () => Promise<string>;
  getCurrentTrack: () => Promise<QueueTrack | undefined>;
}

export function initTestHelpers() {
  // @ts-expect-error - Vite sets import.meta.env at build time
  if (import.meta.env?.MODE === 'test' || import.meta.env?.DEV) {
    console.log('[TestHelpers] Initializing test helpers...');

    (window as unknown as Record<string, unknown>).__testHelpers = {
      async skipToQueueIndex(index: number) {
        console.log(`[TestHelpers] Skipping to queue index: ${index}`);
        return await invoke('skip_to_queue_index', { index });
      },

      async getQueueSize() {
        const queue = await invoke<QueueTrack[]>('get_queue');
        console.log(`[TestHelpers] Current queue size: ${queue.length}`);
        return queue.length;
      },

      async getPlaybackState() {
        const state = await invoke<string>('get_playback_state');
        console.log(`[TestHelpers] Current playback state: ${state}`);
        return state;
      },

      async getCurrentTrack() {
        // Get from player store
        const queue = await invoke<QueueTrack[]>('get_queue');
        const currentTrack = queue[0];
        console.log(`[TestHelpers] Current track:`, currentTrack);
        return currentTrack;
      },
    } as TestHelpers;

    console.log('[TestHelpers] ✓ Test helpers initialized');
  }
}
