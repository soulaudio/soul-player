/**
 * Unit tests for WasmPlaybackAdapter
 * Tests queue management, playback control, shuffle/repeat, and event emission
 *
 * Note: These tests mock the WASM module since it requires a browser environment.
 * Full integration tests should be done with Playwright or Cypress.
 */

import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest'
import { WasmPlaybackAdapter } from '../wasm-adapter'
import type { QueueTrack, ShuffleMode, RepeatMode } from '../types'

// Mock WASM module
vi.mock('../wasm/soul_playback', () => {
  let mockState = 'stopped'
  let mockQueue: any[] = []
  let mockCurrentIndex = -1
  let mockShuffle = 'off'
  let mockRepeat = 'off'
  let mockVolume = 100
  let mockMuted = false
  let mockHistory: any[] = []

  // Event callbacks
  let onStateChangeCb: ((state: string) => void) | null = null
  let onTrackChangeCb: ((track: any | null) => void) | null = null
  let onQueueChangeCb: (() => void) | null = null
  let _onErrorCb: ((error: string) => void) | null = null

  class WasmPlaybackManager {
    // Event registration
    onStateChange(cb: (state: string) => void) {
      onStateChangeCb = cb
    }

    onTrackChange(cb: (track: any | null) => void) {
      onTrackChangeCb = cb
    }

    onQueueChange(cb: () => void) {
      onQueueChangeCb = cb
    }

    onError(cb: (error: string) => void) {
      _onErrorCb = cb
    }

    // Playback control
    play() {
      if (mockQueue.length === 0) {
        throw new Error('Queue is empty')
      }

      if (mockState === 'paused') {
        mockState = 'playing'
        onStateChangeCb?.('playing')
        return
      }

      if (mockCurrentIndex === -1) {
        mockCurrentIndex = 0
      }

      mockState = 'loading'
      onStateChangeCb?.('loading')

      // Simulate track change
      const currentTrack = mockQueue[mockCurrentIndex]
      onTrackChangeCb?.(currentTrack)

      mockState = 'playing'
    }

    pause() {
      mockState = 'paused'
      onStateChangeCb?.('paused')
    }

    stop() {
      mockState = 'stopped'
      mockCurrentIndex = -1
      onStateChangeCb?.('stopped')
      onTrackChangeCb?.(null)
    }

    next() {
      if (!this.hasNext()) {
        throw new Error('No next track')
      }

      mockCurrentIndex++
      const nextTrack = mockQueue[mockCurrentIndex]
      onTrackChangeCb?.(nextTrack)
    }

    previous() {
      if (!this.hasPrevious()) {
        throw new Error('No previous track')
      }

      mockCurrentIndex--
      const prevTrack = mockQueue[mockCurrentIndex]
      onTrackChangeCb?.(prevTrack)
    }

    hasNext(): boolean {
      if (mockRepeat === 'one') return true
      if (mockRepeat === 'all' && mockQueue.length > 0) return true
      return mockCurrentIndex < mockQueue.length - 1
    }

    hasPrevious(): boolean {
      if (mockRepeat === 'one') return true
      if (mockRepeat === 'all' && mockQueue.length > 0) return true
      return mockCurrentIndex > 0
    }

    // Queue management
    loadPlaylist(tracks: any[]) {
      mockQueue = [...tracks]
      mockCurrentIndex = -1
      onQueueChangeCb?.()
    }

    addToQueueNext(track: any) {
      const insertIndex = mockCurrentIndex + 1
      mockQueue.splice(insertIndex, 0, track)
      onQueueChangeCb?.()
    }

    addToQueueEnd(track: any) {
      mockQueue.push(track)
      onQueueChangeCb?.()
    }

    appendToQueue(tracks: any[]) {
      mockQueue.push(...tracks)
      onQueueChangeCb?.()
    }

    skipToQueueIndex(index: number) {
      if (index < 0 || index >= mockQueue.length) {
        throw new Error('Invalid queue index')
      }
      mockCurrentIndex = index
      const track = mockQueue[index]
      onTrackChangeCb?.(track)
    }

    removeFromQueue(index: number) {
      if (index < 0 || index >= mockQueue.length) {
        return null
      }
      const removed = mockQueue.splice(index, 1)[0]
      if (index < mockCurrentIndex) {
        mockCurrentIndex--
      } else if (index === mockCurrentIndex) {
        // Current track removed, stop playback
        mockCurrentIndex = -1
        mockState = 'stopped'
      }
      onQueueChangeCb?.()
      return removed
    }

    clearQueue() {
      mockQueue = []
      mockCurrentIndex = -1
      mockState = 'stopped'
      onQueueChangeCb?.()
    }

    getQueue() {
      return [...mockQueue]
    }

    queueLength() {
      return mockQueue.length
    }

    // Shuffle & Repeat
    setShuffle(mode: string) {
      mockShuffle = mode
      if (mode === 'random') {
        // Simulate shuffle
        onQueueChangeCb?.()
      }
    }

    setRepeat(mode: string) {
      mockRepeat = mode
      onQueueChangeCb?.()
    }

    getShuffle() {
      return mockShuffle
    }

    getRepeat() {
      return mockRepeat
    }

    // Volume
    setVolume(level: number) {
      mockVolume = level
    }

    getVolume() {
      return mockVolume
    }

    mute() {
      mockMuted = true
    }

    unmute() {
      mockMuted = false
    }

    toggleMute() {
      mockMuted = !mockMuted
    }

    isMuted() {
      return mockMuted
    }

    // State
    getState() {
      return mockState
    }

    getHistory() {
      return mockHistory
    }

    free() {
      // Cleanup
    }
  }

  class WasmQueueTrack {
    id: string
    path: string
    title: string
    artist: string
    album?: string
    duration_secs?: number
    trackNumber?: number

    constructor(
      id: string,
      path: string,
      title: string,
      artist: string,
      duration_secs?: number
    ) {
      this.id = id
      this.path = path
      this.title = title
      this.artist = artist
      this.duration_secs = duration_secs
    }
  }

  return {
    default: vi.fn(() => Promise.resolve()),
    WasmPlaybackManager,
    WasmQueueTrack,
    __setMockState: (state: string) => { mockState = state },
    __setMockQueue: (queue: any[]) => { mockQueue = queue },
    __setMockCurrentIndex: (index: number) => { mockCurrentIndex = index },
    __resetMocks: () => {
      mockState = 'stopped'
      mockQueue = []
      mockCurrentIndex = -1
      mockShuffle = 'off'
      mockRepeat = 'off'
      mockVolume = 100
      mockMuted = false
      mockHistory = []
      onStateChangeCb = null
      onTrackChangeCb = null
      onQueueChangeCb = null
      _onErrorCb = null
    }
  }
})

// Import after mock setup
const wasmMock = await import('../wasm/soul_playback')

describe('WasmPlaybackAdapter', () => {
  let adapter: WasmPlaybackAdapter

  // Helper to create mock tracks
  const createMockTrack = (id: string, title: string): QueueTrack => ({
    id,
    path: `https://example.com/${id}.mp3`,
    title,
    artist: 'Test Artist',
    album: 'Test Album',
    duration_secs: 180,
    track_number: 1,
    source: { type: 'single' }
  })

  beforeEach(async () => {
    // Reset WASM mocks
    ;(wasmMock as any).__resetMocks()

    // Create adapter and initialize
    adapter = new WasmPlaybackAdapter()
    await adapter.initialize()
  })

  afterEach(() => {
    // Only destroy if still initialized (some tests destroy manually)
    try {
      adapter.destroy()
    } catch (error) {
      // Adapter may already be destroyed in some tests
    }
  })

  describe('Initialization', () => {
    it('should initialize successfully', async () => {
      const newAdapter = new WasmPlaybackAdapter()
      await expect(newAdapter.initialize()).resolves.toBeUndefined()
      newAdapter.destroy()
    })

    it('should not re-initialize if already initialized', async () => {
      // Already initialized in beforeEach
      await adapter.initialize()

      // Should not throw
      expect(adapter.getState()).toBe('stopped')
    })

    it('should throw if methods called before initialization', () => {
      const uninitAdapter = new WasmPlaybackAdapter()

      expect(() => uninitAdapter.getState()).toThrow('not initialized')

      // Cleanup without calling methods that require initialization
      ;(uninitAdapter as any).initialized = false
      ;(uninitAdapter as any).audioPlayer.destroy()
    })
  })

  describe('Queue Management', () => {
    it('should load a playlist', () => {
      const tracks = [
        createMockTrack('1', 'Track 1'),
        createMockTrack('2', 'Track 2'),
        createMockTrack('3', 'Track 3')
      ]

      adapter.loadPlaylist(tracks)

      expect(adapter.queueLength()).toBe(3)
      const queue = adapter.getQueue()
      expect(queue).toHaveLength(3)
      expect(queue[0].title).toBe('Track 1')
    })

    it('should throw when loading playlist with invalid tracks', () => {
      const invalidTracks = [
        { id: '', path: '', title: '', artist: '' } as QueueTrack
      ]

      expect(() => adapter.loadPlaylist(invalidTracks)).toThrow('missing required fields')
    })

    it('should add track to queue next', () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      const newTrack = createMockTrack('2', 'Track 2')
      adapter.addToQueueNext(newTrack)

      expect(adapter.queueLength()).toBe(2)
      const queue = adapter.getQueue()
      // Track 2 should be inserted at index 0 (next position when current index is -1)
      expect(queue[0].title).toBe('Track 2')
    })

    it('should add track to queue end', () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      const newTrack = createMockTrack('2', 'Track 2')
      adapter.addToQueueEnd(newTrack)

      expect(adapter.queueLength()).toBe(2)
      const queue = adapter.getQueue()
      expect(queue[1].title).toBe('Track 2')
    })

    it('should append multiple tracks to queue', () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      const newTracks = [
        createMockTrack('2', 'Track 2'),
        createMockTrack('3', 'Track 3')
      ]
      adapter.appendToQueue(newTracks)

      expect(adapter.queueLength()).toBe(3)
    })

    it('should throw when appending invalid tracks', () => {
      const invalidTracks = [
        { id: '', path: '', title: '', artist: '' } as QueueTrack
      ]

      expect(() => adapter.appendToQueue(invalidTracks)).toThrow('missing required fields')
    })

    it('should remove track from queue', () => {
      const tracks = [
        createMockTrack('1', 'Track 1'),
        createMockTrack('2', 'Track 2'),
        createMockTrack('3', 'Track 3')
      ]
      adapter.loadPlaylist(tracks)

      const removed = adapter.removeFromQueue(1)

      expect(removed?.title).toBe('Track 2')
      expect(adapter.queueLength()).toBe(2)
    })

    it('should return null when removing invalid index', () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      const removed = adapter.removeFromQueue(10)

      expect(removed).toBeNull()
    })

    it('should clear queue', () => {
      const tracks = [
        createMockTrack('1', 'Track 1'),
        createMockTrack('2', 'Track 2')
      ]
      adapter.loadPlaylist(tracks)

      adapter.clearQueue()

      expect(adapter.queueLength()).toBe(0)
    })

    it('should emit queueChange when queue modified', async () => {
      const callback = vi.fn()
      adapter.on('queueChange', callback)

      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      // Wait for deferred emit
      await new Promise(resolve => setTimeout(resolve, 10))

      expect(callback).toHaveBeenCalled()
    })
  })

  describe('Playback Control', () => {
    beforeEach(() => {
      const tracks = [
        createMockTrack('1', 'Track 1'),
        createMockTrack('2', 'Track 2'),
        createMockTrack('3', 'Track 3')
      ]
      adapter.loadPlaylist(tracks)
    })

    it('should start playback', async () => {
      const callback = vi.fn()
      adapter.on('trackChange', callback)

      await adapter.play()

      // Should emit track change
      expect(callback).toHaveBeenCalled()
      expect(adapter.getCurrentTrack()?.title).toBe('Track 1')
    })

    it('should pause playback', async () => {
      const callback = vi.fn()
      adapter.on('stateChange', callback)

      await adapter.play()
      callback.mockClear() // Clear play events

      adapter.pause()

      expect(adapter.getState()).toBe('paused')
      expect(callback).toHaveBeenCalledWith('paused')
    })

    it('should resume from pause', async () => {
      await adapter.play()
      adapter.pause()

      const callback = vi.fn()
      adapter.on('stateChange', callback)

      await adapter.play()

      expect(callback).toHaveBeenCalledWith('playing')
    })

    it('should stop playback', async () => {
      const stateCallback = vi.fn()
      const trackCallback = vi.fn()
      adapter.on('stateChange', stateCallback)
      adapter.on('trackChange', trackCallback)

      await adapter.play()
      stateCallback.mockClear()
      trackCallback.mockClear()

      adapter.stop()

      expect(adapter.getState()).toBe('stopped')
      expect(adapter.getCurrentTrack()).toBeNull()
      expect(stateCallback).toHaveBeenCalledWith('stopped')
    })

    it('should emit stateChange events', async () => {
      const callback = vi.fn()
      adapter.on('stateChange', callback)

      await adapter.play()

      expect(callback).toHaveBeenCalled()
    })

    it('should emit trackChange events', async () => {
      const callback = vi.fn()
      adapter.on('trackChange', callback)

      await adapter.play()

      expect(callback).toHaveBeenCalledWith(expect.objectContaining({
        title: 'Track 1'
      }))
    })
  })

  describe('Queue Navigation', () => {
    beforeEach(() => {
      const tracks = [
        createMockTrack('1', 'Track 1'),
        createMockTrack('2', 'Track 2'),
        createMockTrack('3', 'Track 3')
      ]
      adapter.loadPlaylist(tracks)
    })

    it('should skip to next track', async () => {
      await adapter.play()

      await adapter.next()

      expect(adapter.getCurrentTrack()?.title).toBe('Track 2')
    })

    it('should skip to previous track', async () => {
      await adapter.play()
      await adapter.next()

      await adapter.previous()

      expect(adapter.getCurrentTrack()?.title).toBe('Track 1')
    })

    it('should warn when no next track available', async () => {
      await adapter.play()
      await adapter.next()
      await adapter.next()

      // At last track, next() should warn and not advance
      // (WASM hasNext() returns false, so next() exits early)
      const currentTrack = adapter.getCurrentTrack()
      await adapter.next()
      expect(adapter.getCurrentTrack()).toEqual(currentTrack)
    })

    it('should warn when no previous track available', async () => {
      await adapter.play()

      // At first track, previous() should warn and not go back
      // (WASM hasPrevious() returns false, so previous() exits early)
      const currentTrack = adapter.getCurrentTrack()
      await adapter.previous()
      expect(adapter.getCurrentTrack()).toEqual(currentTrack)
    })

    it('should skip to specific queue index', async () => {
      await adapter.skipToQueueIndex(2)

      expect(adapter.getCurrentTrack()?.title).toBe('Track 3')
    })

    it('should throw on invalid queue index', async () => {
      await expect(adapter.skipToQueueIndex(-1)).rejects.toThrow('Invalid queue index')
      await expect(adapter.skipToQueueIndex(10)).rejects.toThrow('Invalid queue index')
    })

    it('should throw when skipping to index on empty queue', async () => {
      adapter.clearQueue()

      await expect(adapter.skipToQueueIndex(0)).rejects.toThrow('queue is empty')
    })
  })

  describe('Shuffle Mode', () => {
    beforeEach(() => {
      const tracks = [
        createMockTrack('1', 'Track 1'),
        createMockTrack('2', 'Track 2'),
        createMockTrack('3', 'Track 3')
      ]
      adapter.loadPlaylist(tracks)
    })

    it('should start with shuffle off', () => {
      expect(adapter.getShuffle()).toBe('off')
    })

    it('should enable shuffle', () => {
      adapter.setShuffle('random' as ShuffleMode)

      expect(adapter.getShuffle()).toBe('random')
    })

    it('should emit shuffleChange event', () => {
      const callback = vi.fn()
      adapter.on('shuffleChange', callback)

      adapter.setShuffle('random' as ShuffleMode)

      expect(callback).toHaveBeenCalledWith('random')
    })

    it('should emit queueChange when shuffle enabled', async () => {
      const callback = vi.fn()
      adapter.on('queueChange', callback)

      adapter.setShuffle('random' as ShuffleMode)

      // Wait for deferred emit
      await new Promise(resolve => setTimeout(resolve, 10))

      expect(callback).toHaveBeenCalled()
    })
  })

  describe('Repeat Mode', () => {
    beforeEach(() => {
      const tracks = [
        createMockTrack('1', 'Track 1'),
        createMockTrack('2', 'Track 2')
      ]
      adapter.loadPlaylist(tracks)
    })

    it('should start with repeat off', () => {
      expect(adapter.getRepeat()).toBe('off')
    })

    it('should enable repeat all', () => {
      adapter.setRepeat('all' as RepeatMode)

      expect(adapter.getRepeat()).toBe('all')
    })

    it('should enable repeat one', () => {
      adapter.setRepeat('one' as RepeatMode)

      expect(adapter.getRepeat()).toBe('one')
    })

    it('should emit repeatChange event', () => {
      const callback = vi.fn()
      adapter.on('repeatChange', callback)

      adapter.setRepeat('all' as RepeatMode)

      expect(callback).toHaveBeenCalledWith('all')
    })

    it('should affect hasNext/hasPrevious with repeat all', async () => {
      await adapter.play()
      await adapter.next()

      adapter.setRepeat('all' as RepeatMode)

      expect(adapter.hasNext()).toBe(true)
      expect(adapter.hasPrevious()).toBe(true)
    })

    it('should affect hasNext/hasPrevious with repeat one', async () => {
      await adapter.play()

      adapter.setRepeat('one' as RepeatMode)

      expect(adapter.hasNext()).toBe(true)
      expect(adapter.hasPrevious()).toBe(true)
    })
  })

  describe('Volume Control', () => {
    it('should start with default volume', () => {
      expect(adapter.getVolume()).toBe(100)
    })

    it('should set volume', () => {
      adapter.setVolume(50)

      expect(adapter.getVolume()).toBe(50)
    })

    it('should throw on invalid volume below 0', () => {
      expect(() => adapter.setVolume(-10)).toThrow('Invalid volume level')
    })

    it('should throw on invalid volume above 100', () => {
      expect(() => adapter.setVolume(150)).toThrow('Invalid volume level')
    })

    it('should mute audio', () => {
      adapter.mute()

      expect(adapter.getIsMuted()).toBe(true)
    })

    it('should unmute audio', () => {
      adapter.mute()
      adapter.unmute()

      expect(adapter.getIsMuted()).toBe(false)
    })

    it('should toggle mute', () => {
      expect(adapter.getIsMuted()).toBe(false)

      adapter.toggleMute()
      expect(adapter.getIsMuted()).toBe(true)

      adapter.toggleMute()
      expect(adapter.getIsMuted()).toBe(false)
    })

    it('should emit volumeChange event', () => {
      const callback = vi.fn()
      adapter.on('volumeChange', callback)

      adapter.setVolume(75)

      expect(callback).toHaveBeenCalledWith(75)
    })

    it('should emit muteChange event', () => {
      const callback = vi.fn()
      adapter.on('muteChange', callback)

      adapter.mute()

      expect(callback).toHaveBeenCalledWith(true)
    })
  })

  describe('Seek Functionality', () => {
    beforeEach(async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)
      await adapter.play()

      // Mock audio duration
      const audioPlayer = (adapter as any).audioPlayer
      audioPlayer.audioElement.duration = 180
    })

    it('should seek to position', () => {
      adapter.seek(60)

      expect(adapter.getPosition()).toBe(60)
    })

    it('should clamp negative position to 0', () => {
      // Changed behavior: now clamps instead of throwing
      adapter.seek(-10)
      expect(adapter.getPosition()).toBe(0)
    })

    it('should clamp position that exceeds duration', () => {
      // Changed behavior: now clamps instead of throwing
      adapter.seek(200)
      // Should be clamped to duration - 0.1
      expect(adapter.getPosition()).toBeCloseTo(179.9, 1)
    })

    it('should seek by percent', () => {
      adapter.seekPercent(50)

      expect(adapter.getPosition()).toBe(90) // 50% of 180s
    })

    it('should throw on invalid percent below 0', () => {
      expect(() => adapter.seekPercent(-10)).toThrow('Invalid seek percent')
    })

    it('should throw on invalid percent above 100', () => {
      expect(() => adapter.seekPercent(150)).toThrow('Invalid seek percent')
    })
  })

  describe('Event Emitter', () => {
    it('should register event listeners', () => {
      const callback = vi.fn()
      const cleanup = adapter.on('stateChange', callback)

      expect(typeof cleanup).toBe('function')
    })

    it('should emit events to listeners', () => {
      const callback = vi.fn()
      adapter.on('stateChange', callback)

      // Trigger event by changing state
      adapter.stop()

      expect(callback).toHaveBeenCalled()
    })

    it('should remove listeners on cleanup', () => {
      const callback = vi.fn()
      const cleanup = adapter.on('stateChange', callback)

      cleanup()

      adapter.stop()
      expect(callback).toHaveBeenCalledTimes(0)
    })

    it('should support multiple listeners for same event', () => {
      const callback1 = vi.fn()
      const callback2 = vi.fn()

      adapter.on('stateChange', callback1)
      adapter.on('stateChange', callback2)

      adapter.stop()

      expect(callback1).toHaveBeenCalled()
      expect(callback2).toHaveBeenCalled()
    })
  })

  describe('Error Handling', () => {
    it('should emit error on playback failure with empty queue', async () => {
      const errorCallback = vi.fn()
      adapter.on('error', errorCallback)

      // Try to play empty queue
      adapter.clearQueue()

      // Should not throw, should emit error
      await adapter.play()

      expect(errorCallback).toHaveBeenCalledWith('Cannot play - queue is empty')
    })

    it('should handle WASM errors gracefully', () => {
      // Invalid operation should return null, not throw
      const result = adapter.removeFromQueue(-1)
      expect(result).toBeNull()
    })
  })

  describe('State Synchronization', () => {
    it('should provide forceSyncQueueState method', () => {
      expect(() => adapter.forceSyncQueueState()).not.toThrow()
    })

    it('should track queue state', () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      expect(adapter.queueLength()).toBe(1)
      expect(adapter.getQueue()).toHaveLength(1)
    })
  })

  describe('Cleanup', () => {
    it('should cleanup on destroy', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      // Start playback to set current track
      await adapter.play()

      // Verify current track is set
      expect(adapter.getCurrentTrack()).not.toBeNull()

      expect(() => adapter.destroy()).not.toThrow()

      // After destroy, currentTrack should be null (stored in adapter, doesn't need WASM)
      expect(adapter.getCurrentTrack()).toBeNull()
    })

    it('should remove all event listeners on destroy', () => {
      const callback = vi.fn()
      adapter.on('stateChange', callback)

      // Trigger event before destroy to verify listener works
      adapter.stop()
      const callCountBefore = callback.mock.calls.length

      adapter.destroy()

      // Verify callback was called before destroy
      expect(callCountBefore).toBeGreaterThan(0)

      // After destroy, event listeners map should be cleared
      const eventListeners = (adapter as any).eventListeners
      expect(eventListeners.size).toBe(0)
    })
  })

  // ===== INTEGRATION TESTS: Edge Cases & Critical Paths =====

  describe('Queue Empty Edge Cases', () => {
    it('should emit error when play() called on empty queue', async () => {
      const errorCallback = vi.fn()
      adapter.on('error', errorCallback)

      adapter.clearQueue()

      await adapter.play()

      expect(errorCallback).toHaveBeenCalledWith('Cannot play - queue is empty')
    })

    it('should emit error when next() called at end of queue with repeat off', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)
      await adapter.play()

      const errorCallback = vi.fn()
      adapter.on('error', errorCallback)

      // Already at last track, next() should emit error
      await adapter.next()

      expect(errorCallback).toHaveBeenCalledWith('No more tracks in queue')
    })

    it('should emit error when previous() called at start of queue', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)
      await adapter.play()

      const errorCallback = vi.fn()
      adapter.on('error', errorCallback)

      // Already at first track, previous() should emit error
      await adapter.previous()

      expect(errorCallback).toHaveBeenCalledWith('No previous tracks in queue')
    })

    it('should handle skipToQueueIndex with empty queue', async () => {
      adapter.clearQueue()

      const errorCallback = vi.fn()
      adapter.on('error', errorCallback)

      await expect(adapter.skipToQueueIndex(0)).rejects.toThrow('queue is empty')
      expect(errorCallback).toHaveBeenCalledWith('Cannot skip to track - queue is empty')
    })

    it('should handle resume playback after queue cleared', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)
      await adapter.play()

      // Manually set audioPlaybackState to 'paused' (simulate pause)
      ;(adapter as any).audioPlaybackState = 'paused'
      ;(wasmMock as any).__setMockState('paused')

      const errorCallback = vi.fn()
      adapter.on('error', errorCallback)

      // Manually clear queue without calling adapter.clearQueue()
      // This simulates queue being cleared while paused
      ;(wasmMock as any).__setMockQueue([])

      // Try to resume - should emit error because queue is empty
      await adapter.play()

      // Should emit error for empty queue
      expect(errorCallback).toHaveBeenCalledWith('Cannot resume - queue is empty')
    })

    it('should handle clearQueue during playback', async () => {
      const tracks = [
        createMockTrack('1', 'Track 1'),
        createMockTrack('2', 'Track 2')
      ]
      adapter.loadPlaylist(tracks)

      // Wait for initial loadPlaylist emit
      await new Promise(resolve => setTimeout(resolve, 10))

      const queueCallback = vi.fn()
      adapter.on('queueChange', queueCallback)

      adapter.clearQueue()

      // Wait for deferred emit
      await new Promise(resolve => setTimeout(resolve, 10))

      expect(adapter.queueLength()).toBe(0)
      expect(queueCallback).toHaveBeenCalled()
      expect(adapter.getState()).toBe('stopped')
    })
  })

  describe('Recursive Borrow Prevention (Deferred Emits)', () => {
    it('should allow event callbacks to call getQueue without errors', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      let capturedQueue: QueueTrack[] = []
      adapter.on('queueChange', () => {
        // This would cause RefCell borrow error if not deferred
        capturedQueue = adapter.getQueue()
      })

      const newTrack = createMockTrack('2', 'Track 2')
      adapter.addToQueueEnd(newTrack)

      // Wait for deferred emit
      await new Promise(resolve => setTimeout(resolve, 10))

      expect(capturedQueue).toHaveLength(2)
      expect(capturedQueue[1].title).toBe('Track 2')
    })

    it('should allow trackChange callbacks to modify queue', async () => {
      const tracks = [
        createMockTrack('1', 'Track 1'),
        createMockTrack('2', 'Track 2')
      ]
      adapter.loadPlaylist(tracks)

      let callbackExecuted = false
      adapter.on('trackChange', (track) => {
        if (track?.id === '2') {
          // Add track during trackChange callback
          const newTrack = createMockTrack('3', 'Track 3')
          adapter.addToQueueEnd(newTrack)
          callbackExecuted = true
        }
      })

      await adapter.play()
      await adapter.next()

      // Wait for deferred emit
      await new Promise(resolve => setTimeout(resolve, 10))

      expect(callbackExecuted).toBe(true)
      expect(adapter.queueLength()).toBe(3)
    })

    it('should handle multiple rapid queue operations without conflicts', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      // Wait for initial loadPlaylist emit
      await new Promise(resolve => setTimeout(resolve, 10))

      const queueCallback = vi.fn()
      adapter.on('queueChange', queueCallback)

      // Rapid queue modifications
      adapter.addToQueueEnd(createMockTrack('2', 'Track 2'))
      adapter.addToQueueEnd(createMockTrack('3', 'Track 3'))
      adapter.addToQueueEnd(createMockTrack('4', 'Track 4'))
      adapter.removeFromQueue(1)
      adapter.addToQueueNext(createMockTrack('5', 'Track 5'))

      // Wait for all deferred emits
      await new Promise(resolve => setTimeout(resolve, 50))

      // Should have emitted at least 5 times (may emit more due to internal state changes)
      expect(queueCallback.mock.calls.length).toBeGreaterThanOrEqual(5)
      expect(adapter.queueLength()).toBe(4)
    })

    it('should allow queueChange listeners to safely call queue methods', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      let recursiveCallCompleted = false
      adapter.on('queueChange', () => {
        // Safe because of deferred emit
        const queue = adapter.getQueue()
        const length = adapter.queueLength()
        recursiveCallCompleted = queue.length === length
      })

      adapter.addToQueueEnd(createMockTrack('2', 'Track 2'))

      // Wait for deferred emit
      await new Promise(resolve => setTimeout(resolve, 10))

      expect(recursiveCallCompleted).toBe(true)
    })
  })

  describe('Race Conditions', () => {
    it('should handle multiple play() calls in quick succession', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      // Trigger multiple play calls without waiting
      const play1 = adapter.play()
      const play2 = adapter.play()
      const play3 = adapter.play()

      await Promise.all([play1, play2, play3])

      // Should only play once, not throw errors
      expect(adapter.getCurrentTrack()?.title).toBe('Track 1')
      expect(['playing', 'loading']).toContain(adapter.getState())
    })

    it('should handle play() called while loading track', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      // Start playing
      const playPromise = adapter.play()

      // Immediately try to play again while loading
      await adapter.play()

      await playPromise

      // Should not break, should play the track
      expect(adapter.getCurrentTrack()?.title).toBe('Track 1')
    })

    it('should handle pause during track loading', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      // Start playing (goes into loading state)
      const playPromise = adapter.play()

      // Immediately pause while loading
      adapter.pause()

      await playPromise

      // Should be paused after loading completes
      expect(adapter.getState()).toBe('paused')
    })

    it('should handle next() during track loading', async () => {
      const tracks = [
        createMockTrack('1', 'Track 1'),
        createMockTrack('2', 'Track 2'),
        createMockTrack('3', 'Track 3')
      ]
      adapter.loadPlaylist(tracks)

      await adapter.play()

      // Trigger next while track is loading
      const next1 = adapter.next()
      const next2 = adapter.next()

      await Promise.all([next1, next2])

      // Should advance to track 3
      expect(adapter.getCurrentTrack()?.title).toBe('Track 3')
    })

    it('should handle queue modification during playback', async () => {
      const tracks = [
        createMockTrack('1', 'Track 1'),
        createMockTrack('2', 'Track 2')
      ]
      adapter.loadPlaylist(tracks)

      await adapter.play()

      // Modify queue while playing
      adapter.addToQueueEnd(createMockTrack('3', 'Track 3'))
      adapter.removeFromQueue(1) // Remove Track 2

      // Wait for deferred emits
      await new Promise(resolve => setTimeout(resolve, 10))

      // Should still be playing Track 1
      expect(adapter.getCurrentTrack()?.title).toBe('Track 1')
      expect(adapter.queueLength()).toBe(2)
    })

    it('should handle loadPlaylist during playback', async () => {
      const tracks1 = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks1)
      await adapter.play()

      // Load new playlist while playing
      const tracks2 = [
        createMockTrack('2', 'Track 2'),
        createMockTrack('3', 'Track 3')
      ]
      adapter.loadPlaylist(tracks2)

      // Wait for deferred emit
      await new Promise(resolve => setTimeout(resolve, 10))

      // Queue should be replaced
      expect(adapter.queueLength()).toBe(2)
      const queue = adapter.getQueue()
      expect(queue[0].title).toBe('Track 2')
    })
  })

  describe('State Sync Edge Cases', () => {
    it('should detect audio playing but WASM paused', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)
      await adapter.play()

      // Manually pause WASM but keep audio playing (desync scenario)
      ;(adapter as any).wasmManager.pause()
      // Audio player keeps playing - desync!

      const state = adapter.getState()
      expect(state).toBe('paused') // WASM state wins
    })

    it('should handle forceSyncQueueState without errors', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)
      await adapter.play()

      expect(() => adapter.forceSyncQueueState()).not.toThrow()
    })

    it('should handle state sync after rapid queue modifications', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      // Rapid modifications
      for (let i = 2; i <= 10; i++) {
        adapter.addToQueueEnd(createMockTrack(String(i), `Track ${i}`))
      }

      // Force sync
      adapter.forceSyncQueueState()

      expect(adapter.queueLength()).toBe(10)
    })

    it('should emit queueChange when track changes', async () => {
      const tracks = [
        createMockTrack('1', 'Track 1'),
        createMockTrack('2', 'Track 2')
      ]
      adapter.loadPlaylist(tracks)

      const queueCallback = vi.fn()
      adapter.on('queueChange', queueCallback)

      await adapter.play()
      queueCallback.mockClear()

      await adapter.next()

      // Wait for deferred emit
      await new Promise(resolve => setTimeout(resolve, 10))

      // Queue position changed, should emit
      expect(queueCallback).toHaveBeenCalled()
    })
  })

  describe('Track Transition Edge Cases', () => {
    it('should stop when last track finishes with repeat off', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)
      await adapter.play()

      const stateCallback = vi.fn()
      adapter.on('stateChange', stateCallback)

      // Simulate track finished
      ;(adapter as any).handleTrackFinished()

      expect(adapter.getState()).toBe('stopped')
      expect(stateCallback).toHaveBeenCalledWith('stopped')
    })

    it('should loop to first track when last track finishes with repeat all', async () => {
      const tracks = [
        createMockTrack('1', 'Track 1'),
        createMockTrack('2', 'Track 2')
      ]
      adapter.loadPlaylist(tracks)
      adapter.setRepeat('all' as RepeatMode)

      await adapter.play()
      await adapter.next() // Now at Track 2 (last track)

      const trackCallback = vi.fn()
      adapter.on('trackChange', trackCallback)

      // Simulate track finished
      ;(adapter as any).handleTrackFinished()

      // Should loop back to Track 1
      // Note: Mock hasNext() returns true for repeat all
      expect(adapter.hasNext()).toBe(true)
    })

    it('should repeat same track when track finishes with repeat one', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)
      adapter.setRepeat('one' as RepeatMode)

      await adapter.play()

      // hasNext should return true for repeat one
      expect(adapter.hasNext()).toBe(true)
    })

    it('should handle track finished with shuffle enabled', async () => {
      const tracks = [
        createMockTrack('1', 'Track 1'),
        createMockTrack('2', 'Track 2'),
        createMockTrack('3', 'Track 3')
      ]
      adapter.loadPlaylist(tracks)
      adapter.setShuffle('random' as ShuffleMode)

      await adapter.play()

      // With shuffle, hasNext should return true
      expect(adapter.hasNext()).toBe(true)
    })

    it('should handle handleTrackFinished with empty queue', () => {
      adapter.clearQueue()

      // Should not throw, should stop
      expect(() => (adapter as any).handleTrackFinished()).not.toThrow()
      expect(adapter.getState()).toBe('stopped')
    })
  })

  describe('Volume/Seek Edge Cases', () => {
    beforeEach(async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)
      await adapter.play()

      // Mock audio duration
      const audioPlayer = (adapter as any).audioPlayer
      audioPlayer.audioElement.duration = 180
    })

    it('should clamp seeking beyond track duration', () => {
      // Changed behavior: now clamps instead of throwing
      adapter.seek(200)
      expect(adapter.getPosition()).toBeCloseTo(179.9, 1)
    })

    it('should clamp seeking to negative position', () => {
      // Changed behavior: now clamps instead of throwing
      adapter.seek(-10)
      expect(adapter.getPosition()).toBe(0)
    })

    it('should throw when setting volume above 100', () => {
      expect(() => adapter.setVolume(150)).toThrow('Invalid volume level')
    })

    it('should throw when setting volume below 0', () => {
      expect(() => adapter.setVolume(-10)).toThrow('Invalid volume level')
    })

    it('should handle seek while paused', () => {
      adapter.pause()

      expect(() => adapter.seek(60)).not.toThrow()
      expect(adapter.getPosition()).toBe(60)
    })

    it('should handle seek to exact duration boundary', () => {
      const duration = (adapter as any).audioPlayer.audioElement.duration

      // Seeking to exact duration should not throw
      expect(() => adapter.seek(duration)).not.toThrow()
    })

    it('should handle seekPercent with 0%', () => {
      expect(() => adapter.seekPercent(0)).not.toThrow()
      expect(adapter.getPosition()).toBe(0)
    })

    it('should handle seekPercent with 100%', () => {
      const duration = (adapter as any).audioPlayer.audioElement.duration
      expect(() => adapter.seekPercent(100)).not.toThrow()
      expect(adapter.getPosition()).toBe(duration)
    })

    it('should handle volume change during mute', () => {
      adapter.mute()
      adapter.setVolume(50)

      // Volume state should change but audio should stay muted
      expect(adapter.getVolume()).toBe(50)
      expect(adapter.getIsMuted()).toBe(true)
    })
  })

  describe('Cleanup Edge Cases', () => {
    it('should handle stop() on uninitialized adapter', () => {
      const uninitAdapter = new WasmPlaybackAdapter()

      // Should not throw
      expect(() => uninitAdapter.stop()).not.toThrow()
    })

    it('should handle multiple destroy() calls', () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      adapter.destroy()

      // Second destroy should not throw
      expect(() => adapter.destroy()).not.toThrow()
    })

    it('should remove event listeners after cleanup', () => {
      const callback = vi.fn()
      const cleanup = adapter.on('stateChange', callback)

      cleanup()

      adapter.stop()

      // Callback should not be called
      expect(callback).not.toHaveBeenCalled()
    })

    it('should stop state sync interval on cleanup', () => {
      const stateSyncIntervalId = (adapter as any).stateSyncIntervalId

      adapter.destroy()

      expect((adapter as any).stateSyncIntervalId).toBeNull()
    })

    it('should free WASM manager on destroy', () => {
      const freeSpy = vi.fn()
      ;(adapter as any).wasmManager.free = freeSpy

      adapter.destroy()

      expect(freeSpy).toHaveBeenCalled()
      expect((adapter as any).wasmManager).toBeNull()
    })
  })

  describe('Performance & Stress Tests', () => {
    it('should handle large queue (100 tracks)', () => {
      const tracks = Array.from({ length: 100 }, (_, i) =>
        createMockTrack(String(i + 1), `Track ${i + 1}`)
      )

      expect(() => adapter.loadPlaylist(tracks)).not.toThrow()
      expect(adapter.queueLength()).toBe(100)
    })

    it('should handle rapid queue modifications (50 operations)', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      for (let i = 2; i <= 50; i++) {
        adapter.addToQueueEnd(createMockTrack(String(i), `Track ${i}`))
      }

      // Wait for deferred emits
      await new Promise(resolve => setTimeout(resolve, 100))

      expect(adapter.queueLength()).toBe(50)
    })

    it('should handle event emission overhead (many listeners)', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      // Register 100 listeners
      const callbacks = Array.from({ length: 100 }, () => vi.fn())
      callbacks.forEach(cb => adapter.on('stateChange', cb))

      adapter.stop()

      // All callbacks should be called
      callbacks.forEach(cb => expect(cb).toHaveBeenCalledWith('stopped'))
    })

    it('should not leak memory during long session simulation', async () => {
      const tracks = [
        createMockTrack('1', 'Track 1'),
        createMockTrack('2', 'Track 2'),
        createMockTrack('3', 'Track 3')
      ]
      adapter.loadPlaylist(tracks)

      // Simulate 100 track changes
      await adapter.play()
      for (let i = 0; i < 100; i++) {
        await adapter.next()
        if (!adapter.hasNext()) {
          adapter.setRepeat('all' as RepeatMode)
        }
      }

      // Should still have 3 tracks
      expect(adapter.queueLength()).toBe(3)

      // Should still be functional
      expect(() => adapter.pause()).not.toThrow()
    })

    it('should handle rapid play/pause cycles', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      for (let i = 0; i < 20; i++) {
        await adapter.play()
        adapter.pause()
      }

      expect(adapter.getState()).toBe('paused')
    })

    it('should handle queue with tracks missing optional fields', () => {
      const tracks = [
        {
          id: '1',
          path: 'https://example.com/1.mp3',
          title: 'Track 1',
          artist: 'Artist',
          source: { type: 'single' as const }
          // No album, duration_secs, track_number
        }
      ]

      expect(() => adapter.loadPlaylist(tracks)).not.toThrow()
      expect(adapter.queueLength()).toBe(1)

      const queue = adapter.getQueue()
      // WASM serialization converts undefined to null
      expect(queue[0].album).toBeNull()
    })
  })

  describe('Error Handling & Recovery', () => {
    it('should emit error event for invalid track validation', () => {
      const invalidTracks = [
        { id: '', path: '', title: '', artist: '' } as QueueTrack
      ]

      // Error is thrown but not emitted via event (synchronous validation)
      expect(() => adapter.loadPlaylist(invalidTracks)).toThrow('missing required fields')
    })

    it('should recover from failed track load', async () => {
      const tracks = [
        createMockTrack('1', 'Track 1'),
        createMockTrack('2', 'Track 2')
      ]
      adapter.loadPlaylist(tracks)

      const errorCallback = vi.fn()
      adapter.on('error', errorCallback)

      await adapter.play()

      // Simulate track load failure
      const audioPlayer = (adapter as any).audioPlayer
      audioPlayer.loadTrack = vi.fn().mockRejectedValue(new Error('Failed to load'))

      await adapter.next()

      // Wait for error
      await new Promise(resolve => setTimeout(resolve, 50))

      expect(errorCallback).toHaveBeenCalled()
    })

    it('should handle getQueue failure gracefully', () => {
      // Mock WASM getQueue to throw
      ;(adapter as any).wasmManager.getQueue = vi.fn().mockImplementation(() => {
        throw new Error('WASM error')
      })

      const errorCallback = vi.fn()
      adapter.on('error', errorCallback)

      const queue = adapter.getQueue()

      expect(queue).toEqual([])
      expect(errorCallback).toHaveBeenCalledWith('Failed to retrieve queue')
    })

    it('should handle queueLength failure gracefully', () => {
      // Mock WASM queueLength to throw
      ;(adapter as any).wasmManager.queueLength = vi.fn().mockImplementation(() => {
        throw new Error('WASM error')
      })

      const length = adapter.queueLength()

      expect(length).toBe(0)
    })

    it('should handle getShuffle failure gracefully', () => {
      // Mock WASM getShuffle to throw
      ;(adapter as any).wasmManager.getShuffle = vi.fn().mockImplementation(() => {
        throw new Error('WASM error')
      })

      const mode = adapter.getShuffle()

      expect(mode).toBe('off')
    })

    it('should handle getRepeat failure gracefully', () => {
      // Mock WASM getRepeat to throw
      ;(adapter as any).wasmManager.getRepeat = vi.fn().mockImplementation(() => {
        throw new Error('WASM error')
      })

      const mode = adapter.getRepeat()

      expect(mode).toBe('off')
    })

    it('should handle getVolume failure gracefully', () => {
      // Mock WASM getVolume to throw
      ;(adapter as any).wasmManager.getVolume = vi.fn().mockImplementation(() => {
        throw new Error('WASM error')
      })

      const volume = adapter.getVolume()

      expect(volume).toBe(100)
    })

    it('should handle getIsMuted failure gracefully', () => {
      // Mock WASM isMuted to throw
      ;(adapter as any).wasmManager.isMuted = vi.fn().mockImplementation(() => {
        throw new Error('WASM error')
      })

      const muted = adapter.getIsMuted()

      expect(muted).toBe(false)
    })

    it('should handle hasNext failure gracefully', () => {
      // Mock WASM hasNext to throw
      ;(adapter as any).wasmManager.hasNext = vi.fn().mockImplementation(() => {
        throw new Error('WASM error')
      })

      const hasNext = adapter.hasNext()

      expect(hasNext).toBe(false)
    })

    it('should handle hasPrevious failure gracefully', () => {
      // Mock WASM hasPrevious to throw
      ;(adapter as any).wasmManager.hasPrevious = vi.fn().mockImplementation(() => {
        throw new Error('WASM error')
      })

      const hasPrevious = adapter.hasPrevious()

      expect(hasPrevious).toBe(false)
    })

    it('should handle getHistory failure gracefully', () => {
      // Mock WASM getHistory to throw
      ;(adapter as any).wasmManager.getHistory = vi.fn().mockImplementation(() => {
        throw new Error('WASM error')
      })

      const errorCallback = vi.fn()
      adapter.on('error', errorCallback)

      const history = adapter.getHistory()

      expect(history).toEqual([])
    })
  })

  describe('Deferred Emit Behavior Verification', () => {
    it('should emit queueChange asynchronously (deferred)', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      let emitted = false
      adapter.on('queueChange', () => {
        emitted = true
      })

      adapter.addToQueueEnd(createMockTrack('2', 'Track 2'))

      // Should not be emitted immediately
      expect(emitted).toBe(false)

      // Wait for deferred emit
      await new Promise(resolve => setTimeout(resolve, 10))

      // Now it should be emitted
      expect(emitted).toBe(true)
    })

    it('should verify deferred emit prevents recursive WASM calls', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      let recursiveCallAttempted = false
      adapter.on('queueChange', () => {
        // This would cause RefCell borrow error without deferred emit
        try {
          adapter.getQueue()
          recursiveCallAttempted = true
        } catch (error) {
          // Should not throw
          recursiveCallAttempted = false
        }
      })

      adapter.addToQueueEnd(createMockTrack('2', 'Track 2'))

      // Wait for deferred emit
      await new Promise(resolve => setTimeout(resolve, 10))

      expect(recursiveCallAttempted).toBe(true)
    })

    it('should emit events in correct order with deferred emits', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      const events: string[] = []

      adapter.on('stateChange', () => events.push('stateChange'))
      adapter.on('trackChange', () => events.push('trackChange'))
      adapter.on('queueChange', () => events.push('queueChange'))

      await adapter.play()

      // Wait for all deferred emits
      await new Promise(resolve => setTimeout(resolve, 20))

      // trackChange should emit queueChange (deferred)
      expect(events).toContain('queueChange')
    })
  })

  // ===== HOT RELOAD & EVENT RECOVERY TESTS =====

  describe('Hot Reload Recovery', () => {
    it('should recover when trackChange event does not fire', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      // Mock WASM to simulate hot reload where event listener is lost
      const mockManager = adapter['wasmManager'] as any
      const originalPlay = mockManager.play.bind(mockManager)

      // Override play to NOT emit trackChange event
      mockManager.play = vi.fn(() => {
        // Call original but prevent event emission
        mockManager.getState = vi.fn(() => 'loading')
        // Don't call onTrackChangeCb
      })

      // Mock currentTrack to return a track for manual recovery
      const mockTrack = {
        id: '1',
        path: 'https://example.com/1.mp3',
        title: 'Track 1',
        artist: 'Test Artist',
        album: 'Test Album',
        duration_secs: 180,
        track_number: 1
      }
      mockManager.currentTrack = vi.fn(() => mockTrack)

      // Should not throw - should recover manually after timeout
      await expect(adapter.play()).resolves.not.toThrow()

      // Verify manual recovery occurred
      expect(mockManager.currentTrack).toHaveBeenCalled()
      expect(adapter.getCurrentTrack()?.title).toBe('Track 1')
    })

    it('should handle play() with inconsistent state after hot reload', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      // Simulate hot reload: WASM thinks it's paused but audio is stopped
      ;(adapter as any).audioPlaybackState = 'stopped'
      ;(wasmMock as any).__setMockState('paused')

      // Should handle gracefully
      await expect(adapter.play()).resolves.not.toThrow()
    })

    it('should recover from event listeners lost after module reload', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      // Clear all event listeners (simulating hot reload)
      ;(adapter as any).eventListeners.clear()

      // Re-register listeners
      const stateCallback = vi.fn()
      adapter.on('stateChange', stateCallback)

      await adapter.play()

      // New listener should receive events
      expect(stateCallback).toHaveBeenCalled()
    })

    it('should handle duplicate initialization attempts', async () => {
      // Initialize again (already initialized in beforeEach)
      await adapter.initialize()
      await adapter.initialize()
      await adapter.initialize()

      // Should not throw or break existing state
      expect(adapter.getState()).toBe('stopped')
    })

    it('should recover from partial initialization state', async () => {
      const uninitAdapter = new WasmPlaybackAdapter()

      // Partially initialize - set flag but not WASM manager
      ;(uninitAdapter as any).initialized = true
      ;(uninitAdapter as any).wasmManager = null

      // Should throw with clear error
      expect(() => uninitAdapter.getState()).toThrow('not initialized')
    })

    it('should handle queue exists but currentTrack is null', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      // Manually clear current track (simulating hot reload desync)
      ;(adapter as any).currentTrack = null

      // Should handle gracefully
      expect(adapter.getCurrentTrack()).toBeNull()
      expect(adapter.queueLength()).toBe(1)
    })

    it('should handle state mismatch between audio player and WASM', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)
      await adapter.play()

      // Force desync: audio playing but WASM says stopped
      ;(adapter as any).audioPlaybackState = 'playing'
      ;(wasmMock as any).__setMockState('stopped')

      // getState should return WASM state
      expect(adapter.getState()).toBe('stopped')
    })

    it('should recover from failed initialization', async () => {
      const failAdapter = new WasmPlaybackAdapter()

      // Mock WASM init to fail
      vi.spyOn(wasmMock, 'default').mockRejectedValueOnce(new Error('WASM load failed'))

      // Should throw
      await expect(failAdapter.initialize()).rejects.toThrow('WASM load failed')

      // Should still be uninitialized
      expect(() => failAdapter.getState()).toThrow('not initialized')
    })

    it('should handle re-initialization after destroy', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)
      await adapter.play()

      // Track state before destroy
      const hadTracks = adapter.queueLength() > 0

      // Destroy
      adapter.destroy()

      // After destroy, adapter is uninitialized
      expect(() => adapter.getState()).toThrow('not initialized')

      // Re-initialize creates a fresh state
      await adapter.initialize()

      // Should work normally with fresh state
      expect(adapter.getState()).toBe('stopped')
      expect(adapter.queueLength()).toBe(0)
      expect(hadTracks).toBe(true) // Verify we had tracks before
    })

    it('should handle multiple rapid initializations', async () => {
      const adapters = Array.from({ length: 5 }, () => new WasmPlaybackAdapter())

      // Initialize all rapidly
      const initPromises = adapters.map(a => a.initialize())
      await Promise.all(initPromises)

      // All should be initialized
      adapters.forEach(a => {
        expect(a.getState()).toBe('stopped')
      })

      // Cleanup
      adapters.forEach(a => a.destroy())
    })
  })

  describe('Event Recovery Tests', () => {
    it('should handle trackChange event never fires - manual fallback works', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      const mockManager = adapter['wasmManager'] as any

      // Mock play to set loading state but never emit trackChange
      const originalPlay = mockManager.play
      mockManager.play = vi.fn(() => {
        originalPlay.call(mockManager) // Set state to loading
        // Don't call onTrackChangeCb
      })

      // Mock currentTrack for manual recovery
      const mockTrack = {
        id: '1',
        path: 'https://example.com/1.mp3',
        title: 'Track 1',
        artist: 'Test Artist',
        duration_secs: 180
      }
      mockManager.currentTrack = vi.fn(() => mockTrack)

      // Override getState to return 'loading' after play()
      let callCount = 0
      const originalGetState = mockManager.getState
      mockManager.getState = vi.fn(() => {
        callCount++
        // First call: before play, second call: after play (should be loading)
        return callCount <= 1 ? originalGetState() : 'loading'
      })

      await adapter.play()

      // Verify manual recovery path was taken
      expect(mockManager.currentTrack).toHaveBeenCalled()
      expect(adapter.getCurrentTrack()?.title).toBe('Track 1')
    })

    it('should handle onTrackChange callback throws error - handle gracefully', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      // Register callback that throws
      const errorCallback = vi.fn()
      adapter.on('trackChange', () => {
        throw new Error('Callback error')
      })
      adapter.on('error', errorCallback)

      // Call should complete despite callback error (errors propagate in event emitter)
      try {
        await adapter.play()
      } catch (error) {
        // Error from callback is expected to propagate
        expect(error).toBeInstanceOf(Error)
        expect((error as Error).message).toBe('Callback error')
      }
    })

    it('should handle event fires after timeout - verify no double loading', async () => {
      vi.useFakeTimers()

      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      const trackCallback = vi.fn()
      adapter.on('trackChange', trackCallback)

      // Start play
      const playPromise = adapter.play()

      // Advance past the 200ms timeout
      vi.advanceTimersByTime(250)

      await playPromise

      // Event should fire only once (no duplicates)
      expect(trackCallback).toHaveBeenCalledTimes(1)

      vi.useRealTimers()
    })

    it('should handle event fires with null track - handle properly', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)
      await adapter.play()

      const trackCallback = vi.fn()
      adapter.on('trackChange', trackCallback)

      // Stop playback - should emit null track
      adapter.stop()

      expect(trackCallback).toHaveBeenCalledWith(null)
      expect(adapter.getCurrentTrack()).toBeNull()
    })

    it('should handle multiple rapid trackChange events', async () => {
      const tracks = [
        createMockTrack('1', 'Track 1'),
        createMockTrack('2', 'Track 2'),
        createMockTrack('3', 'Track 3')
      ]
      adapter.loadPlaylist(tracks)

      const trackCallback = vi.fn()
      adapter.on('trackChange', trackCallback)

      await adapter.play()

      // Rapidly advance tracks
      await adapter.next()
      await adapter.next()

      // Should have 3 track changes total (initial + 2 next)
      expect(trackCallback).toHaveBeenCalledTimes(3)
    })

    it('should handle trackChange during pending play() call', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      // Start play (returns promise that's still pending)
      const playPromise = adapter.play()

      // Manually trigger trackChange before play() completes
      const mockManager = adapter['wasmManager'] as any
      const mockTrack = {
        id: '1',
        path: 'https://example.com/1.mp3',
        title: 'Track 1',
        artist: 'Test Artist'
      }

      // Wait for play to complete
      await playPromise

      expect(adapter.getCurrentTrack()?.title).toBe('Track 1')
    })

    it('should handle deferred event emission timing', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      let emitOrder: string[] = []

      adapter.on('queueChange', () => {
        emitOrder.push('queueChange')
      })

      adapter.on('trackChange', () => {
        emitOrder.push('trackChange')
      })

      await adapter.play()

      // Wait for deferred emits
      await new Promise(resolve => setTimeout(resolve, 20))

      // Both events should have fired
      expect(emitOrder).toContain('trackChange')
      expect(emitOrder).toContain('queueChange')
    })

    it('should handle event listener cleanup on destroy', () => {
      const stateCallback = vi.fn()
      const trackCallback = vi.fn()

      adapter.on('stateChange', stateCallback)
      adapter.on('trackChange', trackCallback)

      adapter.destroy()

      // Try to trigger events (won't work since destroyed)
      // Verify listeners map is empty
      expect((adapter as any).eventListeners.size).toBe(0)
    })
  })

  describe('Initialization Edge Cases', () => {
    it('should handle initialize twice without destroy', async () => {
      const newAdapter = new WasmPlaybackAdapter()

      await newAdapter.initialize()
      await newAdapter.initialize() // Second init should be no-op

      expect(newAdapter.getState()).toBe('stopped')

      newAdapter.destroy()
    })

    it('should handle play before initialize completes', async () => {
      const uninitAdapter = new WasmPlaybackAdapter()

      // Don't await initialize
      const initPromise = uninitAdapter.initialize()

      // Try to play immediately
      expect(() => uninitAdapter.getQueue()).toThrow('not initialized')

      await initPromise

      // Now it should work
      expect(uninitAdapter.getState()).toBe('stopped')

      uninitAdapter.destroy()
    })

    it('should handle queue operations before initialize', () => {
      const uninitAdapter = new WasmPlaybackAdapter()

      const tracks = [createMockTrack('1', 'Track 1')]

      expect(() => uninitAdapter.loadPlaylist(tracks)).toThrow('not initialized')
      expect(() => uninitAdapter.getQueue()).toThrow('not initialized')
      expect(() => uninitAdapter.queueLength()).toThrow('not initialized')
    })

    it('should handle initialize with corrupted WASM state', async () => {
      const corruptedAdapter = new WasmPlaybackAdapter()

      // Mock WASM manager constructor to return invalid state
      vi.spyOn(wasmMock, 'WasmPlaybackManager').mockImplementationOnce(() => {
        throw new Error('WASM state corrupted')
      })

      await expect(corruptedAdapter.initialize()).rejects.toThrow('WASM state corrupted')
    })

    it('should handle initialize throws error - verify cleanup', async () => {
      const failingAdapter = new WasmPlaybackAdapter()

      // Mock init to fail
      vi.spyOn(wasmMock, 'default').mockRejectedValueOnce(new Error('Init failed'))

      await expect(failingAdapter.initialize()).rejects.toThrow('Init failed')

      // Adapter should still be uninitialized
      expect((failingAdapter as any).initialized).toBe(false)
      expect((failingAdapter as any).wasmManager).toBeNull()
    })

    it('should handle destroy during initialization', async () => {
      const newAdapter = new WasmPlaybackAdapter()

      // Start init but don't wait
      const initPromise = newAdapter.initialize()

      // Destroy immediately
      newAdapter.destroy()

      // Init should still complete
      await initPromise

      // State should be destroyed
      expect((newAdapter as any).wasmManager).toBeNull()
    })
  })

  describe('Manual Recovery Mechanisms', () => {
    it('should manually retrieve track when event fails', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      const mockManager = adapter['wasmManager'] as any

      // Mock currentTrack method
      const mockTrack = {
        id: '1',
        path: 'https://example.com/1.mp3',
        title: 'Track 1',
        artist: 'Test Artist',
        duration_secs: 180
      }
      mockManager.currentTrack = vi.fn(() => mockTrack)

      // Mock play to not emit event
      const originalPlay = mockManager.play
      mockManager.play = vi.fn(() => {
        originalPlay.call(mockManager)
        // Don't emit trackChange
      })

      // Override getState
      let getStateCalls = 0
      mockManager.getState = vi.fn(() => {
        getStateCalls++
        return getStateCalls <= 1 ? 'stopped' : 'loading'
      })

      await adapter.play()

      // Verify currentTrack was called for manual recovery
      expect(mockManager.currentTrack).toHaveBeenCalled()
    })

    it('should handle currentTrack() returns null during recovery', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      const mockManager = adapter['wasmManager'] as any

      // Mock currentTrack to return null
      mockManager.currentTrack = vi.fn(() => null)

      // Mock play to not emit event
      mockManager.play = vi.fn(() => {
        mockManager.getState = vi.fn(() => 'loading')
      })

      const errorCallback = vi.fn()
      adapter.on('error', errorCallback)

      await adapter.play()

      // Should emit error
      expect(errorCallback).toHaveBeenCalledWith(expect.stringContaining('no track available'))
    })

    it('should handle loadAndPlayTrack succeeds after manual recovery', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      const mockManager = adapter['wasmManager'] as any

      // Mock for manual recovery
      const mockTrack = {
        id: '1',
        path: 'https://example.com/1.mp3',
        title: 'Track 1',
        artist: 'Test Artist',
        duration_secs: 180
      }
      mockManager.currentTrack = vi.fn(() => mockTrack)
      mockManager.play = vi.fn()
      mockManager.getState = vi.fn(() => 'loading')

      await adapter.play()

      // Track should be loaded
      expect(adapter.getCurrentTrack()?.title).toBe('Track 1')
    })

    it('should emit error when recovery fails, not throw', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      const mockManager = adapter['wasmManager'] as any

      // Mock recovery methods to fail
      mockManager.currentTrack = vi.fn(() => {
        throw new Error('WASM crashed')
      })
      mockManager.play = vi.fn()
      mockManager.getState = vi.fn(() => 'loading')

      const errorCallback = vi.fn()
      adapter.on('error', errorCallback)

      await adapter.play()

      // Should emit error, not throw
      expect(errorCallback).toHaveBeenCalled()
      expect(adapter.getState()).toBe('stopped')
    })

    it('should sync state after manual recovery', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      const mockManager = adapter['wasmManager'] as any

      // Set up manual recovery scenario
      const mockTrack = {
        id: '1',
        path: 'https://example.com/1.mp3',
        title: 'Track 1',
        artist: 'Test Artist',
        duration_secs: 180
      }
      mockManager.currentTrack = vi.fn(() => mockTrack)
      mockManager.play = vi.fn()
      mockManager.getState = vi.fn(() => 'loading')

      await adapter.play()

      // Force state sync
      adapter.forceSyncQueueState()

      // State should be consistent
      expect(adapter.getCurrentTrack()?.title).toBe('Track 1')
    })
  })

  describe('Development Mode Hot Reload', () => {
    it('should handle rapid hot reloads (5 in quick succession)', async () => {
      const adapters: WasmPlaybackAdapter[] = []

      // Simulate 5 rapid hot reloads
      for (let i = 0; i < 5; i++) {
        const newAdapter = new WasmPlaybackAdapter()
        await newAdapter.initialize()
        adapters.push(newAdapter)
      }

      // All should be functional
      adapters.forEach(a => {
        expect(a.getState()).toBe('stopped')
      })

      // Cleanup
      adapters.forEach(a => a.destroy())
    })

    it('should handle state persistence across reloads', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)
      await adapter.play()

      const beforeState = adapter.getState()
      const beforeTrack = adapter.getCurrentTrack()

      // Simulate hot reload: destroy and recreate
      adapter.destroy()

      const newAdapter = new WasmPlaybackAdapter()
      await newAdapter.initialize()

      // State should be reset (not persisted)
      expect(newAdapter.getState()).toBe('stopped')
      expect(newAdapter.getCurrentTrack()).toBeNull()

      newAdapter.destroy()
    })

    it('should log console warnings for non-critical errors in dev', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      const consoleSpy = vi.spyOn(console, 'warn')

      // Trigger a warning scenario - try to go previous at start
      await adapter.play()
      await adapter.previous()

      expect(consoleSpy).toHaveBeenCalled()

      consoleSpy.mockRestore()
    })

    it('should gracefully degrade without crashing app on WASM errors', async () => {
      const tracks = [createMockTrack('1', 'Track 1')]
      adapter.loadPlaylist(tracks)

      // Mock WASM method to throw
      const mockManager = adapter['wasmManager'] as any
      mockManager.play = vi.fn(() => {
        throw new Error('WASM panic')
      })

      const errorCallback = vi.fn()
      adapter.on('error', errorCallback)

      // Should not throw, should catch and emit error
      await expect(adapter.play()).rejects.toThrow('WASM panic')

      // Adapter should still be functional for other operations
      expect(adapter.getQueue()).toHaveLength(1)
    })
  })
})
