/**
 * Vitest test setup file
 * Runs before all tests to configure global environment
 */

import { beforeEach, vi } from 'vitest'

// Mock Web Audio API (not fully available in jsdom)
class MockAudioContext {
  state = 'running'
  destination = {}

  createGain() {
    return {
      gain: { value: 1 },
      connect: vi.fn(),
      disconnect: vi.fn(),
    }
  }

  createMediaElementSource() {
    return {
      connect: vi.fn(),
      disconnect: vi.fn(),
    }
  }

  async resume() {
    this.state = 'running'
  }

  async suspend() {
    this.state = 'suspended'
  }
}

// Mock Audio element
class MockAudio {
  src = ''
  currentTime = 0
  duration = 0
  paused = true
  ended = false
  volume = 1
  preload = 'auto'
  error: MediaError | null = null

  private listeners = new Map<string, Set<EventListener>>()

  addEventListener(event: string, listener: EventListener) {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set())
    }
    this.listeners.get(event)!.add(listener)
  }

  removeEventListener(event: string, listener: EventListener) {
    this.listeners.get(event)?.delete(listener)
  }

  dispatchEvent(event: Event): boolean {
    this.listeners.get(event.type)?.forEach(listener => listener(event))
    return true
  }

  async play() {
    this.paused = false
    this.dispatchEvent(new Event('play'))
  }

  pause() {
    this.paused = true
    this.dispatchEvent(new Event('pause'))
  }

  load() {
    this.dispatchEvent(new Event('loadstart'))
    // Simulate successful load
    setTimeout(() => {
      this.dispatchEvent(new Event('canplay'))
    }, 10)
  }
}

// @ts-expect-error - Mocking global browser APIs
global.AudioContext = MockAudioContext
// @ts-expect-error - Mocking global browser APIs
global.Audio = MockAudio

// Reset mocks before each test
beforeEach(() => {
  vi.clearAllMocks()
})
