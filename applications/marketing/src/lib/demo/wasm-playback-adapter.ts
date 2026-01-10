/**
 * WASM Playback Adapter
 *
 * Bridges the WASM PlaybackManager (queue/state logic) with WebAudioPlayer (audio output).
 * This adapter is required because WASM can't directly access Web Audio API.
 *
 * Architecture:
 * - WASM: Queue management, shuffle, repeat, state tracking
 * - TypeScript: Audio playback, seeking, volume control via Web Audio API
 */

import init, { WasmPlaybackManager, WasmQueueTrack } from '../../wasm/soul-playback/soul_playback'

// Type for track objects coming from WASM (serde_wasm_bindgen uses snake_case)
interface WasmTrackData {
  id: string
  path: string
  title: string
  artist: string | undefined
  album: string | undefined
  album_artist?: string
  duration_secs: number | undefined
  track_number: number | undefined
  disc_number?: number
  genre?: string
  year?: number
  artwork_path?: string
}
import { WebAudioPlayer } from './audio-player'
import {
  PlaybackState,
  type QueueTrack,
  type RepeatMode,
  type ShuffleMode,
  type PlaybackConfig
} from './types'

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type EventCallback = (...args: any[]) => void

export class WasmPlaybackAdapter {
  private wasmManager: WasmPlaybackManager | null = null
  private audioPlayer: WebAudioPlayer
  private initialized: boolean = false
  private eventListeners: Map<string, EventCallback[]> = new Map()
  private cleanupFunctions: (() => void)[] = []

  // State synced from WASM
  private currentTrack: QueueTrack | null = null

  // Track actual audio playback state independently from WASM state
  // WASM state can get stuck in "Loading" after track loads, so we track real state here
  private audioPlaybackState: 'stopped' | 'playing' | 'paused' | 'loading' = 'stopped'

  constructor(_config: Partial<PlaybackConfig> = {}) {
    this.audioPlayer = new WebAudioPlayer()

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
        console.error('[WasmPlaybackAdapter] Audio error:', error)
        this.emit('error', error)
      })
    )
  }

  /**
   * Initialize WASM module (async)
   * Must be called before using any WASM functions
   */
  async initialize(): Promise<void> {
    if (this.initialized) return

    try {
      // Initialize WASM module
      await init()

      // Create WASM playback manager
      this.wasmManager = new WasmPlaybackManager()

      // Register WASM event callbacks
      this.wasmManager.onStateChange((state: string) => {
        console.log('[WasmPlaybackAdapter] State change:', state)
        this.emit('stateChange', this.mapWasmState(state))
      })

      this.wasmManager.onTrackChange((track: WasmTrackData | null) => {
        console.log('[WasmPlaybackAdapter] *** onTrackChange callback invoked ***', track ? 'with track' : 'null track')
        console.log('[WasmPlaybackAdapter] Track change:', track)
        if (track) {
          this.currentTrack = this.mapWasmTrack(track)
          console.log('[WasmPlaybackAdapter] Mapped track:', this.currentTrack)
          this.loadAndPlayTrack(this.currentTrack)
        } else {
          this.currentTrack = null
          this.audioPlayer.stop()
        }
        this.emit('trackChange', this.currentTrack)
      })

      this.wasmManager.onQueueChange(() => {
        console.log('[WasmPlaybackAdapter] Queue change')
        this.emit('queueChange')
      })

      this.wasmManager.onError((error: string) => {
        console.error('[WasmPlaybackAdapter] WASM error:', error)
        this.emit('error', error)
      })

      // Sync initial volume
      const volume = this.wasmManager.getVolume()
      this.audioPlayer.setVolume(volume)

      this.initialized = true
      console.log('[WasmPlaybackAdapter] WASM initialized successfully')
    } catch (error) {
      console.error('[WasmPlaybackAdapter] Failed to initialize WASM:', error)
      throw error
    }
  }

  // ===== Playback Control =====

  async play(): Promise<void> {
    this.ensureInitialized()
    console.log('[WasmPlaybackAdapter] play() called')
    console.log('[WasmPlaybackAdapter] - WASM state:', this.wasmManager!.getState())
    console.log('[WasmPlaybackAdapter] - Audio playback state:', this.audioPlaybackState)
    console.log('[WasmPlaybackAdapter] - Queue length:', this.wasmManager!.queueLength())

    // Check our actual audio playback state, not WASM state
    // WASM state can be stuck in "Loading" even after audio is playing
    if (this.audioPlaybackState === 'paused') {
      // Resume from pause - just resume audio player
      console.log('[WasmPlaybackAdapter] Resuming from pause (using audio state)')
      const wasmState = this.wasmManager!.getState()

      // Only call WASM play() if it's actually in Paused state
      // If stuck in Loading, don't call it or it will load next track!
      if (wasmState === 'paused') {
        console.log('[WasmPlaybackAdapter] WASM state is paused, calling WASM play()')
        this.wasmManager!.play()
      } else {
        console.log('[WasmPlaybackAdapter] WASM state is', wasmState, '- NOT calling WASM play() to avoid loading next track')
      }

      this.audioPlayer.play()
      this.audioPlaybackState = 'playing'
      // Emit Playing state to update UI
      console.log('[WasmPlaybackAdapter] Emitting Playing state after resume')
      this.emit('stateChange', PlaybackState.Playing)
      return
    }

    // Check if already playing - should not happen if UI is correct, but handle it
    if (this.audioPlaybackState === 'playing') {
      console.log('[WasmPlaybackAdapter] Already playing, ignoring play() call')
      return
    }

    // Not paused or playing, so this is a new playback request
    console.log('[WasmPlaybackAdapter] Starting new playback')
    try {
      this.wasmManager!.play()
      const newState = this.wasmManager!.getState()
      console.log('[WasmPlaybackAdapter] WASM play() returned, new state:', newState)

      // If WASM is in Loading state, a new track is being loaded
      // The onTrackChange event will handle loading and playing the track
      if (newState === 'loading') {
        console.log('[WasmPlaybackAdapter] Loading new track, waiting for track change event...')
        this.audioPlaybackState = 'loading'
        await new Promise(resolve => setTimeout(resolve, 100)) // Wait 100ms for event

        // Check if callback fired
        if (!this.currentTrack) {
          console.error('[WasmPlaybackAdapter] ERROR: Track change event never fired!')
          throw new Error('WASM track change event did not fire - cannot start playback')
        }
      } else if (newState === 'playing') {
        // Direct transition to playing (e.g., resume from WASM perspective)
        this.audioPlayer.play()
        this.audioPlaybackState = 'playing'
      }
    } catch (err) {
      console.error('[WasmPlaybackAdapter] WASM play() threw error:', err)
      throw err
    }
  }

  pause(): void {
    this.ensureInitialized()
    console.log('[WasmPlaybackAdapter] pause() called, audio state:', this.audioPlaybackState)
    this.wasmManager!.pause()
    this.audioPlayer.pause()
    this.audioPlaybackState = 'paused'
    console.log('[WasmPlaybackAdapter] Paused, new audio state:', this.audioPlaybackState)
    // Emit Paused state to update UI
    this.emit('stateChange', PlaybackState.Paused)
  }

  stop(): void {
    this.ensureInitialized()
    this.wasmManager!.stop()
    this.audioPlayer.stop()
    this.audioPlaybackState = 'stopped'
    // Emit Stopped state to update UI
    this.emit('stateChange', PlaybackState.Stopped)
  }

  async next(): Promise<void> {
    this.ensureInitialized()
    console.log('[WasmPlaybackAdapter] next() called')
    this.audioPlaybackState = 'loading' // Will be set to 'playing' by loadAndPlayTrack
    this.wasmManager!.next()
  }

  async previous(): Promise<void> {
    this.ensureInitialized()
    console.log('[WasmPlaybackAdapter] previous() called')
    this.audioPlaybackState = 'loading' // Will be set to 'playing' by loadAndPlayTrack
    this.wasmManager!.previous()
  }

  private async loadAndPlayTrack(track: QueueTrack): Promise<void> {
    try {
      console.log('[WasmPlaybackAdapter] loadAndPlayTrack:', track.title)
      await this.audioPlayer.loadTrack(track.path)
      this.audioPlayer.play()
      // Update our audio playback state - track is now playing!
      this.audioPlaybackState = 'playing'
      console.log('[WasmPlaybackAdapter] Track loaded and playing, audio state:', this.audioPlaybackState)

      // CRITICAL: Emit Playing state change since WASM won't do it
      // WASM state is stuck in Loading because it doesn't know about web audio
      console.log('[WasmPlaybackAdapter] Emitting Playing state to fix UI')
      this.emit('stateChange', PlaybackState.Playing)
    } catch (error) {
      console.error('[WasmPlaybackAdapter] Failed to load track:', error)
      this.audioPlaybackState = 'stopped'
      this.emit('error', error)
    }
  }

  private handleTrackFinished(): void {
    // Auto-advance to next track
    this.next()
  }

  // ===== Queue Management =====

  addToQueueNext(track: QueueTrack): void {
    this.ensureInitialized()
    // Use createWasmTrack for single track methods (they work with WASM objects)
    const wasmTrack = this.createWasmTrack(track)
    this.wasmManager!.addToQueueNext(wasmTrack)
  }

  addToQueueEnd(track: QueueTrack): void {
    this.ensureInitialized()
    // Use createWasmTrack for single track methods (they work with WASM objects)
    const wasmTrack = this.createWasmTrack(track)
    this.wasmManager!.addToQueueEnd(wasmTrack)
  }

  loadPlaylist(tracks: QueueTrack[]): void {
    this.ensureInitialized()
    console.log('[WasmPlaybackAdapter] loadPlaylist called with', tracks.length, 'tracks')
    console.log('[WasmPlaybackAdapter] First track:', tracks[0])

    // IMPORTANT: Pass plain JS objects, not WasmQueueTrack instances!
    // serde_wasm_bindgen expects plain objects with exact field names
    const plainTracks = tracks.map(t => ({
      id: t.id,
      path: t.path,
      title: t.title,
      artist: t.artist,
      album: t.album || null,
      duration_secs: t.duration_secs,  // Must match Rust struct field name
      track_number: t.track_number !== undefined ? t.track_number : null,
    }))

    console.log('[WasmPlaybackAdapter] Converted to plain tracks, first:', plainTracks[0])
    this.wasmManager!.loadPlaylist(plainTracks)
    console.log('[WasmPlaybackAdapter] loadPlaylist completed, queue length:', this.wasmManager!.queueLength())
  }

  async skipToQueueIndex(index: number): Promise<void> {
    this.ensureInitialized()
    console.log('[WasmPlaybackAdapter] skipToQueueIndex:', index)
    this.audioPlaybackState = 'loading' // Will be set to 'playing' by loadAndPlayTrack
    this.wasmManager!.skipToQueueIndex(index)
  }

  clearQueue(): void {
    this.ensureInitialized()
    this.wasmManager!.clearQueue()
    // Clearing queue doesn't necessarily stop playback, but we track it
  }

  getQueue(): QueueTrack[] {
    this.ensureInitialized()
    const wasmQueue = this.wasmManager!.getQueue()
    return wasmQueue.map((t: WasmTrackData) => this.mapWasmTrack(t))
  }

  queueLength(): number {
    this.ensureInitialized()
    return this.wasmManager!.queueLength()
  }

  // ===== Shuffle & Repeat =====

  setShuffle(mode: ShuffleMode): void {
    this.ensureInitialized()
    const modeStr = this.mapShuffleToWasm(mode)
    this.wasmManager!.setShuffle(modeStr)
    this.emit('shuffleChange', mode)
  }

  setRepeat(mode: RepeatMode): void {
    this.ensureInitialized()
    const modeStr = this.mapRepeatToWasm(mode)
    this.wasmManager!.setRepeat(modeStr)
    this.emit('repeatChange', mode)
  }

  getShuffle(): ShuffleMode {
    this.ensureInitialized()
    const modeStr = this.wasmManager!.getShuffle()
    return this.mapShuffleFromWasm(modeStr)
  }

  getRepeat(): RepeatMode {
    this.ensureInitialized()
    const modeStr = this.wasmManager!.getRepeat()
    return this.mapRepeatFromWasm(modeStr)
  }

  // ===== Volume =====

  setVolume(level: number): void {
    this.ensureInitialized()
    this.wasmManager!.setVolume(level)
    if (!this.wasmManager!.isMuted()) {
      this.audioPlayer.setVolume(level)
    }
    this.emit('volumeChange', level)
  }

  mute(): void {
    this.ensureInitialized()
    this.wasmManager!.mute()
    this.audioPlayer.setVolume(0)
    this.emit('muteChange', true)
  }

  unmute(): void {
    this.ensureInitialized()
    this.wasmManager!.unmute()
    const volume = this.wasmManager!.getVolume()
    this.audioPlayer.setVolume(volume)
    this.emit('muteChange', false)
  }

  toggleMute(): void {
    this.ensureInitialized()
    this.wasmManager!.toggleMute()
    if (this.wasmManager!.isMuted()) {
      this.audioPlayer.setVolume(0)
      this.emit('muteChange', true)
    } else {
      const volume = this.wasmManager!.getVolume()
      this.audioPlayer.setVolume(volume)
      this.emit('muteChange', false)
    }
  }

  getVolume(): number {
    this.ensureInitialized()
    return this.wasmManager!.getVolume()
  }

  getIsMuted(): boolean {
    this.ensureInitialized()
    return this.wasmManager!.isMuted()
  }

  // ===== Seek =====

  seek(position: number): void {
    this.ensureInitialized()
    this.audioPlayer.seek(position)
    // WASM doesn't need to know about seek position
  }

  seekPercent(percent: number): void {
    const position = (percent / 100) * this.audioPlayer.duration
    this.seek(position)
  }

  // ===== Getters =====

  getState(): PlaybackState {
    this.ensureInitialized()
    const stateStr = this.wasmManager!.getState()
    return this.mapWasmState(stateStr)
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

  hasNext(): boolean {
    this.ensureInitialized()
    return this.wasmManager!.hasNext()
  }

  hasPrevious(): boolean {
    this.ensureInitialized()
    return this.wasmManager!.hasPrevious()
  }

  getHistory(): QueueTrack[] {
    this.ensureInitialized()
    const wasmHistory = this.wasmManager!.getHistory()
    return wasmHistory.map((t: WasmTrackData) => this.mapWasmTrack(t))
  }

  // ===== Event Emitter =====

  on(event: string, callback: EventCallback): () => void {
    if (!this.eventListeners.has(event)) {
      this.eventListeners.set(event, [])
    }
    this.eventListeners.get(event)!.push(callback)

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

    if (this.wasmManager) {
      this.wasmManager.free()
      this.wasmManager = null
    }
  }

  // ===== Type Conversions =====

  private ensureInitialized(): void {
    if (!this.initialized || !this.wasmManager) {
      throw new Error('WASM not initialized. Call initialize() first.')
    }
  }

  private createWasmTrack(track: QueueTrack): WasmQueueTrack {
    const wasmTrack = new WasmQueueTrack(
      track.id,
      track.path,
      track.title,
      track.artist,
      track.duration_secs  // Use correct field name
    )
    if (track.album) {
      wasmTrack.album = track.album
    }
    if (track.track_number) {
      wasmTrack.trackNumber = track.track_number  // Use correct field name
    }
    return wasmTrack
  }

  private mapWasmTrack(wasmTrack: WasmTrackData): QueueTrack {
    // Note: serde_wasm_bindgen serializes with snake_case field names
    return {
      id: wasmTrack.id,
      path: wasmTrack.path,
      title: wasmTrack.title,
      artist: wasmTrack.artist || '',
      album: wasmTrack.album,
      duration_secs: wasmTrack.duration_secs || 0,  // serde uses snake_case
      track_number: wasmTrack.track_number,  // serde uses snake_case
      source: { type: 'single' }
    }
  }

  private mapWasmState(state: string): PlaybackState {
    switch (state) {
      case 'stopped': return 'stopped' as PlaybackState
      case 'playing': return 'playing' as PlaybackState
      case 'paused': return 'paused' as PlaybackState
      case 'loading': return 'loading' as PlaybackState
      default: return 'stopped' as PlaybackState
    }
  }

  private mapShuffleToWasm(mode: ShuffleMode): string {
    switch (mode) {
      case 'off': return 'off'
      case 'random': return 'random'
      case 'smart': return 'smart'
      default: return 'off'
    }
  }

  private mapShuffleFromWasm(mode: string): ShuffleMode {
    switch (mode) {
      case 'off': return 'off' as ShuffleMode
      case 'random': return 'random' as ShuffleMode
      case 'smart': return 'smart' as ShuffleMode
      default: return 'off' as ShuffleMode
    }
  }

  private mapRepeatToWasm(mode: RepeatMode): string {
    switch (mode) {
      case 'off': return 'off'
      case 'all': return 'all'
      case 'one': return 'one'
      default: return 'off'
    }
  }

  private mapRepeatFromWasm(mode: string): RepeatMode {
    switch (mode) {
      case 'off': return 'off' as RepeatMode
      case 'all': return 'all' as RepeatMode
      case 'one': return 'one' as RepeatMode
      default: return 'off' as RepeatMode
    }
  }

  // ===== Queue Manipulation =====

  removeFromQueue(index: number): QueueTrack | null {
    this.ensureInitialized()
    try {
      const removed = this.wasmManager!.removeFromQueue(index)
      return removed ? this.mapWasmTrack(removed) : null
    } catch (error) {
      console.error('[WasmPlaybackAdapter] Failed to remove from queue:', error)
      return null
    }
  }

  appendToQueue(tracks: QueueTrack[]): void {
    this.ensureInitialized()
    // IMPORTANT: Pass plain JS objects for array methods (serde deserialization)
    const plainTracks = tracks.map(t => ({
      id: t.id,
      path: t.path,
      title: t.title,
      artist: t.artist,
      album: t.album || null,
      duration_secs: t.duration_secs,
      track_number: t.track_number !== undefined ? t.track_number : null,
    }))
    this.wasmManager!.appendToQueue(plainTracks)
  }
}
