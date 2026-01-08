/**
 * Playback Manager - Direct TypeScript port of soul-playback PlaybackManager
 * Handles queue, history, shuffle, repeat, and volume
 */

import { WebAudioPlayer } from './audio-player'
import {
  QueueTrack,
  PlaybackState,
  RepeatMode,
  ShuffleMode,
  PlaybackConfig,
  defaultPlaybackConfig
} from './types'

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type EventCallback = (...args: any[]) => void

export class DemoPlaybackManager {
  // State
  private state: PlaybackState = PlaybackState.Stopped
  private currentTrack: QueueTrack | null = null

  // Queue and history (Spotify-style two-tier queue)
  // Queue playback order: explicitQueue → sourceQueue
  private explicitQueue: QueueTrack[] = [] // Manually added tracks (via "Add to Queue" - persists across contexts)
  private sourceQueue: QueueTrack[] = [] // Auto-queue from current context (playlist/album - cleared when new context loads)
  private history: QueueTrack[] = []
  private originalSourceOrder: QueueTrack[] = [] // For un-shuffling
  private isShuffled: boolean = false

  // Settings
  private volume: number
  private isMuted: boolean = false
  private previousVolume: number = 80
  private shuffle: ShuffleMode
  private repeat: RepeatMode
  private historySize: number

  // Audio player
  private audioPlayer: WebAudioPlayer

  // Event emitter
  private eventListeners: Map<string, EventCallback[]> = new Map()

  // Cleanup functions
  private cleanupFunctions: (() => void)[] = []

  constructor(config: Partial<PlaybackConfig> = {}) {
    const cfg = { ...defaultPlaybackConfig, ...config }

    this.volume = cfg.volume
    this.shuffle = cfg.shuffle
    this.repeat = cfg.repeat
    this.historySize = cfg.historySize

    this.audioPlayer = new WebAudioPlayer()
    this.audioPlayer.setVolume(this.volume)

    // Wire up audio events
    this.cleanupFunctions.push(
      this.audioPlayer.onEnded(() => this.handleTrackFinished())
    )
    this.cleanupFunctions.push(
      this.audioPlayer.onTimeUpdate(position => {
        this.emit('positionUpdate', position)
      })
    )
    this.cleanupFunctions.push(
      this.audioPlayer.onError(error => {
        console.error('Audio error:', error)
        this.emit('error', error)
      })
    )
  }

  // ===== Playback Control =====

  async play(): Promise<void> {
    if (this.state === PlaybackState.Paused) {
      // Resume from pause
      this.audioPlayer.play()
      this.state = PlaybackState.Playing
      this.emit('stateChange', this.state)
    } else if (
      this.state === PlaybackState.Stopped ||
      this.state === PlaybackState.Loading
    ) {
      // Start playing from queue
      await this.playNextInQueue()
    }
    // Already playing - do nothing
  }

  pause(): void {
    if (this.state === PlaybackState.Playing) {
      this.audioPlayer.pause()
      this.state = PlaybackState.Paused
      this.emit('stateChange', this.state)
    }
  }

  stop(): void {
    this.audioPlayer.stop()
    this.state = PlaybackState.Stopped
    this.currentTrack = null
    this.emit('stateChange', this.state)
    this.emit('trackChange', null)
  }

  async next(): Promise<void> {
    // Save current to history
    if (this.currentTrack) {
      this.history.push(this.currentTrack)
      if (this.history.length > this.historySize) {
        this.history.shift()
      }
    }

    await this.playNextInQueue()
  }

  async previous(): Promise<void> {
    // If >3 seconds into track, restart current track
    if (this.audioPlayer.position > 3) {
      this.audioPlayer.seek(0)
      return
    }

    // Otherwise go to previous from history
    const prevTrack = this.history.pop()
    if (prevTrack) {
      // Put current track back at front of queue
      if (this.currentTrack) {
        this.explicitQueue.unshift(this.currentTrack)
        this.emit('queueChange')
      }

      await this.loadAndPlayTrack(prevTrack)
    } else {
      // No history, restart current track
      this.audioPlayer.seek(0)
    }
  }

  private async playNextInQueue(): Promise<void> {
    // Handle repeat one
    if (this.repeat === RepeatMode.One && this.currentTrack) {
      this.audioPlayer.seek(0)
      this.audioPlayer.play()
      this.state = PlaybackState.Playing
      this.emit('stateChange', this.state)
      return
    }

    // Get next track
    const nextTrack = this.getNextTrack()
    if (!nextTrack) {
      this.stop()
      return
    }

    await this.loadAndPlayTrack(nextTrack)
  }

  private getNextTrack(): QueueTrack | null {
    // Explicit queue has priority (user added "play next")
    if (this.explicitQueue.length > 0) {
      const track = this.explicitQueue.shift()!
      this.emit('queueChange')
      return track
    }

    // Then source queue (playlist/album)
    if (this.sourceQueue.length > 0) {
      const track = this.sourceQueue.shift()!
      this.emit('queueChange')
      return track
    }

    // Handle repeat all
    if (this.repeat === RepeatMode.All && this.originalSourceOrder.length > 0) {
      this.sourceQueue = [...this.originalSourceOrder]
      if (this.shuffle !== ShuffleMode.Off) {
        this.shuffleArray(this.sourceQueue, this.shuffle)
      }
      this.emit('queueChange')
      return this.sourceQueue.shift()!
    }

    return null
  }

  private async loadAndPlayTrack(track: QueueTrack): Promise<void> {
    this.state = PlaybackState.Loading
    this.currentTrack = track
    this.emit('trackChange', track)
    this.emit('stateChange', this.state)

    try {
      await this.audioPlayer.loadTrack(track.path)
      this.audioPlayer.play()
      this.state = PlaybackState.Playing
      this.emit('stateChange', this.state)
    } catch (error) {
      console.error('Failed to load track:', error)
      this.state = PlaybackState.Stopped
      this.emit('stateChange', this.state)
      this.emit('error', error)
    }
  }

  private handleTrackFinished(): void {
    // Auto-advance to next track
    this.next()
  }

  // ===== Queue Management =====

  /**
   * Add track to play next (manual "Add to Queue" action)
   * Goes to front of explicit queue, persists across playlist changes
   */
  addToQueueNext(track: QueueTrack): void {
    this.explicitQueue.unshift(track)
    this.emit('queueChange')
  }

  /**
   * Add track to end of queue (manual "Add to Queue" action)
   * Goes to end of explicit queue, persists across playlist changes
   */
  addToQueueEnd(track: QueueTrack): void {
    this.explicitQueue.push(track)
    this.emit('queueChange')
  }

  /**
   * Load a new playback context (playlist, album, etc.)
   * Replaces the entire source queue and clears explicit queue
   * Like clicking play on a track in Spotify - starts fresh context
   */
  loadPlaylist(tracks: QueueTrack[]): void {
    // Clear explicit queue when loading a new playlist context (like Spotify)
    // This ensures clicking play on a track replaces the entire queue, not appends
    this.explicitQueue = []
    this.sourceQueue = [...tracks]
    this.originalSourceOrder = [...tracks]

    if (this.shuffle !== ShuffleMode.Off) {
      this.shuffleArray(this.sourceQueue, this.shuffle)
      this.isShuffled = true
    } else {
      this.isShuffled = false
    }

    this.emit('queueChange')
  }

  /**
   * Append tracks to the current context queue
   * Adds to source queue, doesn't clear it (for adding more from same context)
   */
  appendToQueue(tracks: QueueTrack[]): void {
    const tracksToAdd = [...tracks]

    if (this.shuffle !== ShuffleMode.Off) {
      this.shuffleArray(tracksToAdd, this.shuffle)
    }

    this.sourceQueue.push(...tracksToAdd)
    this.originalSourceOrder.push(...tracks)

    this.emit('queueChange')
  }

  removeFromQueue(index: number): QueueTrack | null {
    const allQueue = this.getQueue()
    if (index < 0 || index >= allQueue.length) {
      return null
    }

    // Determine if it's in explicit or source queue
    if (index < this.explicitQueue.length) {
      const removed = this.explicitQueue.splice(index, 1)[0]
      this.emit('queueChange')
      return removed
    } else {
      const sourceIndex = index - this.explicitQueue.length
      const removed = this.sourceQueue.splice(sourceIndex, 1)[0]
      this.emit('queueChange')
      return removed
    }
  }

  clearQueue(): void {
    this.explicitQueue = []
    this.sourceQueue = []
    this.originalSourceOrder = []
    this.emit('queueChange')
  }

  getQueue(): QueueTrack[] {
    return [...this.explicitQueue, ...this.sourceQueue]
  }

  queueLength(): number {
    return this.explicitQueue.length + this.sourceQueue.length
  }

  // ===== Shuffle & Repeat =====

  setShuffle(mode: ShuffleMode): void {
    if (this.shuffle === mode) return

    const oldMode = this.shuffle
    this.shuffle = mode

    if (mode === ShuffleMode.Off) {
      // Restore original order
      this.sourceQueue = [...this.originalSourceOrder]
      this.isShuffled = false
    } else {
      if (oldMode === ShuffleMode.Off) {
        // First time shuffling - save original order
        this.originalSourceOrder = [...this.sourceQueue]
      }

      // Apply shuffle
      this.shuffleArray(this.sourceQueue, mode)
      this.isShuffled = true
    }

    this.emit('shuffleChange', mode)
    this.emit('queueChange')
  }

  setRepeat(mode: RepeatMode): void {
    this.repeat = mode
    this.emit('repeatChange', mode)
  }

  private shuffleArray(array: QueueTrack[], mode: ShuffleMode): void {
    if (mode === ShuffleMode.Random) {
      // Fisher-Yates shuffle
      for (let i = array.length - 1; i > 0; i--) {
        const j = Math.floor(Math.random() * (i + 1))
        ;[array[i], array[j]] = [array[j], array[i]]
      }
    } else if (mode === ShuffleMode.Smart) {
      // Smart shuffle: try to distribute artists/albums
      // Simplified implementation - just random for now
      // Full implementation would track recently played and artist distribution
      this.shuffleArray(array, ShuffleMode.Random)

      // TODO: Implement proper smart shuffle
      // - Don't repeat same artist back-to-back
      // - Distribute tracks from same album
      // - Avoid recently played tracks
    }
  }

  // ===== Volume =====

  setVolume(level: number): void {
    this.volume = Math.max(0, Math.min(100, level))
    if (!this.isMuted) {
      this.audioPlayer.setVolume(this.volume)
    }
    this.emit('volumeChange', this.volume)
  }

  mute(): void {
    if (!this.isMuted) {
      this.previousVolume = this.volume
      this.isMuted = true
      this.audioPlayer.setVolume(0)
      this.emit('muteChange', true)
    }
  }

  unmute(): void {
    if (this.isMuted) {
      this.isMuted = false
      this.audioPlayer.setVolume(this.volume)
      this.emit('muteChange', false)
    }
  }

  toggleMute(): void {
    if (this.isMuted) {
      this.unmute()
    } else {
      this.mute()
    }
  }

  // ===== Seek =====

  seek(position: number): void {
    this.audioPlayer.seek(position)
  }

  seekPercent(percent: number): void {
    const position = (percent / 100) * this.audioPlayer.duration
    this.seek(position)
  }

  // ===== Getters =====

  getState(): PlaybackState {
    return this.state
  }

  getCurrentTrack(): QueueTrack | null {
    return this.currentTrack
  }

  getPosition(): number {
    return this.audioPlayer.position
  }

  getDuration(): number {
    return this.audioPlayer.duration
  }

  getVolume(): number {
    return this.volume
  }

  getIsMuted(): boolean {
    return this.isMuted
  }

  getShuffle(): ShuffleMode {
    return this.shuffle
  }

  getRepeat(): RepeatMode {
    return this.repeat
  }

  getHistory(): QueueTrack[] {
    return [...this.history]
  }

  hasNext(): boolean {
    return this.queueLength() > 0 || this.repeat === RepeatMode.One
  }

  hasPrevious(): boolean {
    return this.history.length > 0 || this.repeat === RepeatMode.One
  }

  // ===== Event Emitter =====

  on(event: string, callback: EventCallback): () => void {
    if (!this.eventListeners.has(event)) {
      this.eventListeners.set(event, [])
    }
    this.eventListeners.get(event)!.push(callback)

    // Return unsubscribe function
    return () => {
      const callbacks = this.eventListeners.get(event)
      if (callbacks) {
        const index = callbacks.indexOf(callback)
        if (index > -1) {
          callbacks.splice(index, 1)
        }
      }
    }
  }

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  private emit(event: string, ...args: any[]): void {
    this.eventListeners.get(event)?.forEach(cb => cb(...args))
  }

  // ===== Cleanup =====

  destroy(): void {
    this.stop()
    this.cleanupFunctions.forEach(cleanup => cleanup())
    this.audioPlayer.destroy()
    this.eventListeners.clear()
  }
}
