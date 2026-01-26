import { afterEach, vi } from 'vitest';
import { cleanup } from '@testing-library/react';
import '@testing-library/jest-dom/vitest';

// Cleanup after each test
afterEach(() => {
  cleanup();
});

// Mock Next.js router
vi.mock('next/navigation', () => ({
  useRouter: () => ({
    push: vi.fn(),
    replace: vi.fn(),
    prefetch: vi.fn(),
    back: vi.fn(),
    forward: vi.fn(),
    refresh: vi.fn(),
  }),
  usePathname: () => '/',
  useSearchParams: () => new URLSearchParams(),
}));

// Mock Next.js image
vi.mock('next/image', () => ({
  default: vi.fn((props: any) => props),
}));

// Mock WASM module (for tests that don't need real WASM)
vi.mock('@soul-player/playback-web', async () => {
  const actual = await vi.importActual('@soul-player/playback-web');
  return {
    ...actual,
    // Keep real exports but allow mocking in specific tests
  };
});

// Mock window.matchMedia (used by responsive components)
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: vi.fn().mockImplementation((query) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

// Mock IntersectionObserver (used by virtualization)
global.IntersectionObserver = class IntersectionObserver {
  constructor() {}
  disconnect() {}
  observe() {}
  unobserve() {}
  takeRecords() {
    return [];
  }
} as any;

// Mock ResizeObserver (used by responsive components)
global.ResizeObserver = class ResizeObserver {
  constructor() {}
  disconnect() {}
  observe() {}
  unobserve() {}
} as any;

// Mock Web Audio API (used by WASM playback)
globalThis.AudioContext = class AudioContext {
  destination: any = {};
  sampleRate = 44100;
  currentTime = 0;
  state = 'running';

  constructor() {}

  createGain() {
    return {
      gain: { value: 1, setValueAtTime: vi.fn() },
      connect: vi.fn(),
      disconnect: vi.fn(),
    };
  }

  createBufferSource() {
    return {
      buffer: null,
      connect: vi.fn(),
      disconnect: vi.fn(),
      start: vi.fn(),
      stop: vi.fn(),
    };
  }

  createBuffer() {
    return {
      getChannelData: () => new Float32Array(1024),
    };
  }

  decodeAudioData() {
    return Promise.resolve({
      sampleRate: 44100,
      length: 1024,
      duration: 1,
      numberOfChannels: 2,
      getChannelData: () => new Float32Array(1024),
    });
  }

  resume() {
    return Promise.resolve();
  }

  suspend() {
    return Promise.resolve();
  }

  close() {
    return Promise.resolve();
  }
} as any;

// Suppress console errors in tests (unless debugging)
const originalConsoleError = console.error;
console.error = (...args: any[]) => {
  // Filter out expected errors
  const errorMessage = args[0]?.toString() || '';

  // Allow these expected errors through
  const allowedErrors = [
    'Not implemented: HTMLFormElement.prototype.requestSubmit',
    'Warning: ReactDOM.render',
  ];

  if (allowedErrors.some((allowed) => errorMessage.includes(allowed))) {
    return;
  }

  originalConsoleError(...args);
};
