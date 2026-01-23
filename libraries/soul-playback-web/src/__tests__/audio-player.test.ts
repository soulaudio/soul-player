/**
 * Unit tests for WebAudioPlayer
 * Tests audio playback control, volume, seeking, and event emission
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { WebAudioPlayer } from '../audio-player'

describe('WebAudioPlayer', () => {
  let player: WebAudioPlayer

  beforeEach(() => {
    player = new WebAudioPlayer()
  })

  afterEach(() => {
    player.destroy()
  })

  describe('Initialization', () => {
    it('should create AudioContext and gain node', () => {
      expect(player).toBeDefined()
      // Player should be created successfully
    })

    it('should start with default state', () => {
      expect(player.position).toBe(0)
      expect(player.duration).toBe(0)
      expect(player.isPaused).toBe(true)
      expect(player.isFinished).toBe(false)
    })
  })

  describe('Track Loading', () => {
    it('should load a track successfully', async () => {
      const trackUrl = 'https://example.com/audio.mp3'

      const loadPromise = player.loadTrack(trackUrl)

      // Wait for load to complete
      await expect(loadPromise).resolves.toBeUndefined()
    })

    it('should reject on load error', async () => {
      const trackUrl = 'https://example.com/invalid.mp3'

      // Mock error event
      const loadPromise = player.loadTrack(trackUrl)

      // Simulate error - access private audioElement
      const audioElement = (player as any).audioElement
      audioElement.error = { message: 'Network error', code: 4 }
      audioElement.dispatchEvent(new Event('error'))

      await expect(loadPromise).rejects.toThrow('Failed to load audio')
    })

    it('should reset playback position when loading new track', async () => {
      // Set position
      const audioElement = (player as any).audioElement
      audioElement.currentTime = 30

      await player.loadTrack('https://example.com/audio.mp3')

      // Position should be reset to 0
      expect(audioElement.currentTime).toBe(0)
    })
  })

  describe('Playback Control', () => {
    beforeEach(async () => {
      await player.loadTrack('https://example.com/audio.mp3')
    })

    it('should play audio', async () => {
      await player.play()

      expect(player.isPaused).toBe(false)
    })

    it('should pause audio', async () => {
      await player.play()
      player.pause()

      expect(player.isPaused).toBe(true)
    })

    it('should stop audio and reset position', async () => {
      const audioElement = (player as any).audioElement
      await player.play()
      audioElement.currentTime = 30

      player.stop()

      expect(player.isPaused).toBe(true)
      expect(audioElement.currentTime).toBe(0)
    })

    it('should resume from pause', async () => {
      await player.play()
      player.pause()

      const pausedPosition = player.position
      await player.play()

      expect(player.isPaused).toBe(false)
      expect(player.position).toBe(pausedPosition)
    })
  })

  describe('Volume Control', () => {
    it('should set volume to 0 (muted)', () => {
      player.setVolume(0)

      const gainNode = (player as any).gainNode
      expect(gainNode.gain.value).toBe(0)
    })

    it('should set volume to 50 (medium)', () => {
      player.setVolume(50)

      const gainNode = (player as any).gainNode
      // Quadratic scaling: (50/100)^2 = 0.25
      expect(gainNode.gain.value).toBe(0.25)
    })

    it('should set volume to 100 (max)', () => {
      player.setVolume(100)

      const gainNode = (player as any).gainNode
      // Quadratic scaling: (100/100)^2 = 1.0
      expect(gainNode.gain.value).toBe(1)
    })

    it('should clamp volume below 0', () => {
      player.setVolume(-10)

      const gainNode = (player as any).gainNode
      expect(gainNode.gain.value).toBe(0)
    })

    it('should clamp volume above 100', () => {
      player.setVolume(150)

      const gainNode = (player as any).gainNode
      expect(gainNode.gain.value).toBe(1) // (100/100)^2
    })
  })

  describe('Seeking', () => {
    beforeEach(async () => {
      await player.loadTrack('https://example.com/audio.mp3')
      const audioElement = (player as any).audioElement
      audioElement.duration = 180 // 3 minutes
    })

    it('should seek to specific position', () => {
      player.seek(60) // 1 minute

      expect(player.position).toBe(60)
    })

    it('should seek to start', () => {
      player.seek(0)

      expect(player.position).toBe(0)
    })

    it('should allow seeking to end', () => {
      player.seek(180)

      expect(player.position).toBe(180)
    })

    it('should update position getter', () => {
      const audioElement = (player as any).audioElement
      audioElement.currentTime = 45

      expect(player.position).toBe(45)
    })

    it('should update duration getter', () => {
      const audioElement = (player as any).audioElement
      audioElement.duration = 200

      expect(player.duration).toBe(200)
    })
  })

  describe('Event Listeners', () => {
    beforeEach(async () => {
      await player.loadTrack('https://example.com/audio.mp3')
    })

    it('should emit onTimeUpdate events', () => {
      const callback = vi.fn()
      const cleanup = player.onTimeUpdate(callback)

      const audioElement = (player as any).audioElement
      audioElement.currentTime = 10
      audioElement.dispatchEvent(new Event('timeupdate'))

      expect(callback).toHaveBeenCalledWith(10)

      cleanup()
    })

    it('should emit onEnded events', () => {
      const callback = vi.fn()
      const cleanup = player.onEnded(callback)

      const audioElement = (player as any).audioElement
      audioElement.dispatchEvent(new Event('ended'))

      expect(callback).toHaveBeenCalled()

      cleanup()
    })

    it('should emit onError events', () => {
      const callback = vi.fn()
      const cleanup = player.onError(callback)

      const audioElement = (player as any).audioElement
      audioElement.dispatchEvent(new Event('error'))

      expect(callback).toHaveBeenCalledWith(expect.any(Error))

      cleanup()
    })

    it('should emit onPlay events', () => {
      const callback = vi.fn()
      const cleanup = player.onPlay(callback)

      const audioElement = (player as any).audioElement
      audioElement.dispatchEvent(new Event('play'))

      expect(callback).toHaveBeenCalled()

      cleanup()
    })

    it('should emit onPause events', () => {
      const callback = vi.fn()
      const cleanup = player.onPause(callback)

      const audioElement = (player as any).audioElement
      audioElement.dispatchEvent(new Event('pause'))

      expect(callback).toHaveBeenCalled()

      cleanup()
    })

    it('should emit onLoadStart events', () => {
      const callback = vi.fn()
      const cleanup = player.onLoadStart(callback)

      const audioElement = (player as any).audioElement
      audioElement.dispatchEvent(new Event('loadstart'))

      expect(callback).toHaveBeenCalled()

      cleanup()
    })

    it('should remove event listeners on cleanup', () => {
      const callback = vi.fn()
      const cleanup = player.onEnded(callback)

      cleanup()

      const audioElement = (player as any).audioElement
      audioElement.dispatchEvent(new Event('ended'))

      expect(callback).not.toHaveBeenCalled()
    })
  })

  describe('State Tracking', () => {
    beforeEach(async () => {
      await player.loadTrack('https://example.com/audio.mp3')
    })

    it('should track isPaused correctly', async () => {
      expect(player.isPaused).toBe(true)

      await player.play()
      expect(player.isPaused).toBe(false)

      player.pause()
      expect(player.isPaused).toBe(true)
    })

    it('should track isFinished correctly', () => {
      const audioElement = (player as any).audioElement

      expect(player.isFinished).toBe(false)

      audioElement.ended = true
      expect(player.isFinished).toBe(true)
    })
  })

  describe('Cleanup', () => {
    it('should disconnect nodes on destroy', async () => {
      // Load track to create source node
      await player.loadTrack('https://example.com/audio.mp3')

      const sourceNode = (player as any).sourceNode
      const gainNode = (player as any).gainNode

      expect(sourceNode).toBeDefined()
      expect(gainNode).toBeDefined()

      // Verify destroy doesn't throw
      expect(() => player.destroy()).not.toThrow()
    })

    it('should clear audio source on destroy', () => {
      const audioElement = (player as any).audioElement
      audioElement.src = 'https://example.com/audio.mp3'

      player.destroy()

      expect(audioElement.src).toBe('')
    })

    it('should stop playback on destroy', async () => {
      await player.loadTrack('https://example.com/audio.mp3')
      await player.play()

      player.destroy()

      expect(player.isPaused).toBe(true)
    })
  })

  describe('Audio Context State Management', () => {
    it('should resume suspended audio context on play', async () => {
      const audioContext = (player as any).audioContext
      audioContext.state = 'suspended'

      await player.loadTrack('https://example.com/audio.mp3')
      await player.play()

      expect(audioContext.state).toBe('running')
    })

    it('should resume audio context on loadTrack if suspended', async () => {
      const audioContext = (player as any).audioContext
      audioContext.state = 'suspended'

      await player.loadTrack('https://example.com/audio.mp3')

      expect(audioContext.state).toBe('running')
    })
  })
})
