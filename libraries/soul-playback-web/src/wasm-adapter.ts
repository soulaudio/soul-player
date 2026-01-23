/**
 * WASM Playback Adapter
 *
 * Bridges the WASM PlaybackManager (queue/state logic) with WebAudioPlayer (audio output).
 * This adapter is required because WASM can't directly access Web Audio API.
 *
 * Architecture:
 * - WASM: Queue management, shuffle, repeat, state tracking
 * - TypeScript: Audio playback, seeking, volume control via Web Audio API
 *
 * @module @soul-player/playback-web
 * @example
 * ```typescript
 * const adapter = new WasmPlaybackAdapter();
 * await adapter.initialize();
 *
 * // Load and play a playlist
 * adapter.loadPlaylist(tracks);
 * await adapter.play();
 *
 * // Listen for events
 * adapter.on('trackChange', (track) => {
 *   console.log('Now playing:', track.title);
 * });
 * ```
 */

import init, { WasmPlaybackManager, WasmQueueTrack } from './wasm/soul_playback'

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

/**
 * WASM Playback Adapter
 *
 * Core class that bridges WASM queue logic with Web Audio API for browser-based playback.
 *
 * @class WasmPlaybackAdapter
 */
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

  // State synchronization
  private stateSyncIntervalId: number | null = null
  private readonly STATE_SYNC_INTERVAL_MS = 5000 // 5 seconds

  /**
   * Create a new WASM playback adapter
   *
   * @param _config - Optional playback configuration (currently unused, reserved for future use)
   */
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
   * Initialize WASM module
   *
   * Must be called before using any WASM functions. This loads the WASM binary,
   * instantiates the PlaybackManager, and sets up event listeners.
   *
   * @returns Promise that resolves when WASM is fully initialized
   * @throws Error if WASM initialization fails
   *
   * @example
   * ```typescript
   * const adapter = new WasmPlaybackAdapter();
   * await adapter.initialize(); // Must call before use
   * ```
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

        // Automatically emit queue change when track changes
        // This ensures UI stays in sync with queue position changes
        console.log('[WasmPlaybackAdapter] Emitting queueChange (track changed, queue position updated, deferred)')
        this.deferredEmit('queueChange')
      })

      this.wasmManager.onQueueChange(() => {
        console.log('[WasmPlaybackAdapter] Emitting queueChange (WASM onQueueChange event, deferred)')
        this.deferredEmit('queueChange')
      })

      this.wasmManager.onError((error: string) => {
        console.error('[WasmPlaybackAdapter] WASM error:', error)
        this.emit('error', error)
      })

      // Sync initial volume
      const volume = this.wasmManager.getVolume()
      this.audioPlayer.setVolume(volume)

      // Start periodic state synchronization
      this.startStateSyncInterval()

      this.initialized = true
      console.log('[WasmPlaybackAdapter] WASM initialized successfully')
    } catch (error) {
      console.error('[WasmPlaybackAdapter] Failed to initialize WASM:', error)
      throw error
    }
  }

  // ===== Playback Control =====

  /**
   * Start or resume playback
   *
   * If paused, resumes from current position. Otherwise, starts playing from queue beginning.
   * Automatically loads the current track and starts audio output.
   *
   * @returns Promise that resolves when playback starts
   * @throws Error if queue is empty or track loading fails
   *
   * @example
   * ```typescript
   * await adapter.play(); // Start playing
   * ```
   */
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
        // Check queue length before calling play
        if (this.wasmManager!.queueLength() === 0) {
          console.warn('[WasmPlaybackAdapter] Cannot resume - queue is empty')
          this.emit('error', 'Cannot resume - queue is empty')
          return
        }
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

    // Check if queue has tracks before calling play
    const queueLength = this.wasmManager!.queueLength()
    if (queueLength === 0) {
      console.warn('[WasmPlaybackAdapter] Cannot play - queue is empty')
      this.emit('error', 'Cannot play - queue is empty')
      return
    }

    try {
      this.wasmManager!.play()
      const newState = this.wasmManager!.getState()
      console.log('[WasmPlaybackAdapter] WASM play() returned, new state:', newState)

      // If WASM is in Loading state, a new track is being loaded
      // The onTrackChange event will handle loading and playing the track
      if (newState === 'loading') {
        console.log('[WasmPlaybackAdapter] Loading new track, waiting for track change event...')
        this.audioPlaybackState = 'loading'
        await new Promise(resolve => setTimeout(resolve, 200)) // Wait 200ms for event

        // Check if callback fired
        if (!this.currentTrack) {
          console.warn('[WasmPlaybackAdapter] Track change event did not fire within timeout')
          console.warn('[WasmPlaybackAdapter] This can happen during hot reload - try playing again')

          // Reset state and emit error
          this.audioPlaybackState = 'stopped'
          this.emit('error', 'Playback initialization timed out - please try again')
          return
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

  /**
   * Pause playback
   *
   * Pauses audio at current position. Call play() to resume.
   *
   * @example
   * ```typescript
   * adapter.pause();
   * ```
   */
  pause(): void {
    this.ensureInitialized()
    console.log('[WasmPlaybackAdapter] pause() called, audio state:', this.audioPlaybackState)

    try {
      this.wasmManager!.pause()
      this.audioPlayer.pause()
      this.audioPlaybackState = 'paused'
      console.log('[WasmPlaybackAdapter] Paused, new audio state:', this.audioPlaybackState)
      // Emit Paused state to update UI
      this.emit('stateChange', PlaybackState.Paused)
    } catch (error) {
      console.error('[WasmPlaybackAdapter] Failed to pause:', error)
      this.emit('error', 'Failed to pause playback')
      throw error
    }
  }

  stop(): void {
    // If not initialized, nothing to stop
    if (!this.initialized || !this.wasmManager) {
      console.log('[WasmPlaybackAdapter] stop() called but not initialized, nothing to stop')
      return
    }

    try {
      this.wasmManager.stop()
      this.audioPlayer.stop()
      this.audioPlaybackState = 'stopped'
      // Emit Stopped state to update UI
      this.emit('stateChange', PlaybackState.Stopped)
    } catch (error) {
      console.error('[WasmPlaybackAdapter] Failed to stop:', error)
      this.emit('error', 'Failed to stop playback')
      throw error
    }
  }

  async next(): Promise<void> {
    this.ensureInitialized()
    console.log('[WasmPlaybackAdapter] next() called')

    try {
      // Validate that there's a next track before advancing
      if (!this.wasmManager!.hasNext()) {
        console.warn('[WasmPlaybackAdapter] No next track available')
        this.emit('error', 'No more tracks in queue')
        return
      }

      this.audioPlaybackState = 'loading' // Will be set to 'playing' by loadAndPlayTrack
      this.wasmManager!.next()
      // Note: queueChange is automatically emitted by onTrackChange callback
    } catch (error) {
      console.error('[WasmPlaybackAdapter] Failed to skip to next track:', error)
      this.emit('error', 'Failed to skip to next track')
      this.audioPlaybackState = 'stopped'
      throw error
    }
  }

  async previous(): Promise<void> {
    this.ensureInitialized()
    console.log('[WasmPlaybackAdapter] previous() called')

    try {
      // Validate that there's a previous track before going back
      if (!this.wasmManager!.hasPrevious()) {
        console.warn('[WasmPlaybackAdapter] No previous track available')
        this.emit('error', 'No previous tracks in queue')
        return
      }

      this.audioPlaybackState = 'loading' // Will be set to 'playing' by loadAndPlayTrack
      this.wasmManager!.previous()
      // Note: queueChange is automatically emitted by onTrackChange callback
    } catch (error) {
      console.error('[WasmPlaybackAdapter] Failed to skip to previous track:', error)
      this.emit('error', 'Failed to skip to previous track')
      this.audioPlaybackState = 'stopped'
      throw error
    }
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
    console.log('[WasmPlaybackAdapter] Track finished, checking if we can advance')

    // CRITICAL: Check if there's a next track before trying to advance
    // This prevents "Queue is empty" errors when the last track finishes
    // and repeat mode is off. Without this check, calling next() would fail
    // because there's no next track to advance to.
    if (!this.hasNext()) {
      console.log('[WasmPlaybackAdapter] No next track available, stopping playback')
      this.stop()
      return
    }

    console.log('[WasmPlaybackAdapter] Advancing to next track')
    this.next()
  }

  // ===== Queue Management =====

  /**
   * Add track to play next (after current track)
   *
   * Track will play immediately after the current track finishes.
   * Part of the three-tier queue system: [History] → Current → Play Next → Added to Queue.
   *
   * @param track - Track to add (must have id, path, title, artist)
   * @throws Error if track is missing required fields
   *
   * @example
   * ```typescript
   * adapter.addToQueueNext({
   *   id: '123',
   *   path: '/audio/track.mp3',
   *   title: 'Song Title',
   *   artist: 'Artist Name',
   *   duration_secs: 180
   * });
   * ```
   */
  addToQueueNext(track: QueueTrack): void {
    this.ensureInitialized()

    try {
      // Validate track has required fields
      if (!track.id || !track.path || !track.title || !track.artist) {
        const errorMsg = 'Cannot add track to queue - missing required fields'
        console.error('[WasmPlaybackAdapter]', errorMsg, track)
        this.emit('error', errorMsg)
        throw new Error(errorMsg)
      }

      // Use createWasmTrack for single track methods (they work with WASM objects)
      const wasmTrack = this.createWasmTrack(track)
      this.wasmManager!.addToQueueNext(wasmTrack)
      this.deferredEmit('queueChange')
    } catch (error) {
      console.error('[WasmPlaybackAdapter] Failed to add track to queue next:', error)
      if (error instanceof Error) {
        throw error
      }
      const wrappedError = new Error('Failed to add track to queue')
      this.emit('error', wrappedError.message)
      throw wrappedError
    }
  }

  addToQueueEnd(track: QueueTrack): void {
    this.ensureInitialized()

    try {
      // Validate track has required fields
      if (!track.id || !track.path || !track.title || !track.artist) {
        const errorMsg = 'Cannot add track to queue - missing required fields'
        console.error('[WasmPlaybackAdapter]', errorMsg, track)
        this.emit('error', errorMsg)
        throw new Error(errorMsg)
      }

      // Use createWasmTrack for single track methods (they work with WASM objects)
      const wasmTrack = this.createWasmTrack(track)
      this.wasmManager!.addToQueueEnd(wasmTrack)
      this.deferredEmit('queueChange')
    } catch (error) {
      console.error('[WasmPlaybackAdapter] Failed to add track to queue end:', error)
      if (error instanceof Error) {
        throw error
      }
      const wrappedError = new Error('Failed to add track to queue')
      this.emit('error', wrappedError.message)
      throw wrappedError
    }
  }

  /**
   * Load a playlist of tracks into the queue
   *
   * Clears current queue and loads new tracks. Call play() to start playback.
   *
   * IMPORTANT: All tracks must have required fields (id, path, title, artist).
   * Use snake_case field names (duration_secs, track_number) for WASM compatibility.
   *
   * @param tracks - Array of tracks to load
   * @throws Error if any track is missing required fields or queue remains empty after loading
   *
   * @example
   * ```typescript
   * adapter.loadPlaylist([
   *   {
   *     id: '1',
   *     path: '/audio/track1.mp3',
   *     title: 'Song 1',
   *     artist: 'Artist',
   *     duration_secs: 180,
   *     track_number: 1
   *   },
   *   // ... more tracks
   * ]);
   * await adapter.play();
   * ```
   */
  loadPlaylist(tracks: QueueTrack[]): void {
    this.ensureInitialized()
    console.log('[WasmPlaybackAdapter] loadPlaylist called with', tracks.length, 'tracks')
    console.log('[WasmPlaybackAdapter] First track:', tracks[0])

    // Validate tracks have required fields
    const invalidTracks = tracks.filter(t => !t.id || !t.path || !t.title || !t.artist)
    if (invalidTracks.length > 0) {
      console.error('[WasmPlaybackAdapter] Invalid tracks received:', invalidTracks)
      throw new Error(`loadPlaylist: ${invalidTracks.length} track(s) missing required fields`)
    }

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
    console.log('[WasmPlaybackAdapter] Calling WASM loadPlaylist with', plainTracks.length, 'tracks')

    try {
      this.wasmManager!.loadPlaylist(plainTracks)
      const queueLength = this.wasmManager!.queueLength()
      console.log('[WasmPlaybackAdapter] loadPlaylist completed, queue length:', queueLength)

      if (queueLength === 0) {
        console.error('[WasmPlaybackAdapter] CRITICAL: Queue is empty after loadPlaylist!')
        console.error('[WasmPlaybackAdapter] Tracks that were passed:', plainTracks)
        throw new Error('Queue is empty after loading playlist - WASM rejected the tracks')
      }

      // Emit queue change event to update UI
      console.log('[WasmPlaybackAdapter] Emitting queueChange event (deferred)')
      this.deferredEmit('queueChange')
    } catch (error) {
      console.error('[WasmPlaybackAdapter] loadPlaylist failed:', error)
      throw error
    }
  }

  async skipToQueueIndex(index: number): Promise<void> {
    this.ensureInitialized()
    console.log('[WasmPlaybackAdapter] skipToQueueIndex:', index)

    try {
      // Validate index before trying to skip
      const queueLength = this.wasmManager!.queueLength()
      if (queueLength === 0) {
        const errorMsg = 'Cannot skip to track - queue is empty'
        console.error('[WasmPlaybackAdapter]', errorMsg)
        this.emit('error', errorMsg)
        throw new Error(errorMsg)
      }
      if (index < 0 || index >= queueLength) {
        const errorMsg = `Invalid queue index ${index} (queue has ${queueLength} tracks)`
        console.error('[WasmPlaybackAdapter]', errorMsg)
        this.emit('error', errorMsg)
        throw new Error(errorMsg)
      }

      this.audioPlaybackState = 'loading' // Will be set to 'playing' by loadAndPlayTrack
      this.wasmManager!.skipToQueueIndex(index)
      // Note: queueChange is automatically emitted by onTrackChange callback
    } catch (error) {
      console.error('[WasmPlaybackAdapter] Failed to skip to queue index:', error)
      if (error instanceof Error) {
        // Re-throw errors we already handled above
        throw error
      }
      // Wrap unexpected errors
      const wrappedError = new Error('Failed to skip to queue index')
      this.emit('error', wrappedError.message)
      this.audioPlaybackState = 'stopped'
      throw wrappedError
    }
  }

  clearQueue(): void {
    this.ensureInitialized()

    try {
      this.wasmManager!.clearQueue()
      this.deferredEmit('queueChange')
      console.log('[WasmPlaybackAdapter] Queue cleared')
    } catch (error) {
      console.error('[WasmPlaybackAdapter] Failed to clear queue:', error)
      this.emit('error', 'Failed to clear queue')
      throw error
    }
  }

  getQueue(): QueueTrack[] {
    if (!this.initialized || !this.wasmManager) {
      console.warn('[WasmPlaybackAdapter] getQueue called before initialization, returning empty array')
      return []
    }

    try {
      const wasmQueue = this.wasmManager.getQueue()
      return wasmQueue.map((t: WasmTrackData) => this.mapWasmTrack(t))
    } catch (error) {
      console.error('[WasmPlaybackAdapter] Failed to get queue:', error)
      this.emit('error', 'Failed to retrieve queue')
      return []
    }
  }

  queueLength(): number {
    if (!this.initialized || !this.wasmManager) {
      console.warn('[WasmPlaybackAdapter] queueLength called before initialization, returning 0')
      return 0
    }

    try {
      return this.wasmManager.queueLength()
    } catch (error) {
      console.error('[WasmPlaybackAdapter] Failed to get queue length:', error)
      return 0
    }
  }

  // ===== Shuffle & Repeat =====

  setShuffle(mode: ShuffleMode): void {
    this.ensureInitialized()

    try {
      const modeStr = this.mapShuffleToWasm(mode)
      console.log('[WasmPlaybackAdapter] Setting shuffle mode:', modeStr)
      this.wasmManager!.setShuffle(modeStr)
      this.emit('shuffleChange', mode)
      // Emit queue change as shuffle reorders the queue
      console.log('[WasmPlaybackAdapter] Emitting queueChange (shuffle reordered queue, deferred)')
      this.deferredEmit('queueChange')
    } catch (error) {
      console.error('[WasmPlaybackAdapter] Failed to set shuffle mode:', error)
      this.emit('error', 'Failed to change shuffle mode')
      throw error
    }
  }

  setRepeat(mode: RepeatMode): void {
    this.ensureInitialized()

    try {
      const modeStr = this.mapRepeatToWasm(mode)
      console.log('[WasmPlaybackAdapter] Setting repeat mode:', modeStr)
      this.wasmManager!.setRepeat(modeStr)
      this.emit('repeatChange', mode)
      // Emit queue change as repeat mode affects queue behavior (hasNext/hasPrevious)
      console.log('[WasmPlaybackAdapter] Emitting queueChange (repeat mode affects queue behavior, deferred)')
      this.deferredEmit('queueChange')
    } catch (error) {
      console.error('[WasmPlaybackAdapter] Failed to set repeat mode:', error)
      this.emit('error', 'Failed to change repeat mode')
      throw error
    }
  }

  getShuffle(): ShuffleMode {
    if (!this.initialized || !this.wasmManager) {
      console.warn('[WasmPlaybackAdapter] getShuffle called before initialization, returning off')
      return 'off' as ShuffleMode
    }

    try {
      const modeStr = this.wasmManager.getShuffle()
      return this.mapShuffleFromWasm(modeStr)
    } catch (error) {
      console.error('[WasmPlaybackAdapter] Failed to get shuffle mode:', error)
      // Return safe default
      return 'off' as ShuffleMode
    }
  }

  getRepeat(): RepeatMode {
    if (!this.initialized || !this.wasmManager) {
      console.warn('[WasmPlaybackAdapter] getRepeat called before initialization, returning off')
      return 'off' as RepeatMode
    }

    try {
      const modeStr = this.wasmManager.getRepeat()
      return this.mapRepeatFromWasm(modeStr)
    } catch (error) {
      console.error('[WasmPlaybackAdapter] Failed to get repeat mode:', error)
      // Return safe default
      return 'off' as RepeatMode
    }
  }

  // ===== Volume =====

  setVolume(level: number): void {
    this.ensureInitialized()

    try {
      // Validate volume level
      if (level < 0 || level > 100) {
        const errorMsg = `Invalid volume level: ${level} (must be 0-100)`
        console.error('[WasmPlaybackAdapter]', errorMsg)
        this.emit('error', errorMsg)
        throw new Error(errorMsg)
      }

      this.wasmManager!.setVolume(level)
      if (!this.wasmManager!.isMuted()) {
        this.audioPlayer.setVolume(level)
      }
      this.emit('volumeChange', level)
    } catch (error) {
      console.error('[WasmPlaybackAdapter] Failed to set volume:', error)
      if (error instanceof Error) {
        throw error
      }
      const wrappedError = new Error('Failed to set volume')
      this.emit('error', wrappedError.message)
      throw wrappedError
    }
  }

  mute(): void {
    this.ensureInitialized()

    try {
      this.wasmManager!.mute()
      this.audioPlayer.setVolume(0)
      this.emit('muteChange', true)
    } catch (error) {
      console.error('[WasmPlaybackAdapter] Failed to mute:', error)
      this.emit('error', 'Failed to mute audio')
      throw error
    }
  }

  unmute(): void {
    this.ensureInitialized()

    try {
      this.wasmManager!.unmute()
      const volume = this.wasmManager!.getVolume()
      this.audioPlayer.setVolume(volume)
      this.emit('muteChange', false)
    } catch (error) {
      console.error('[WasmPlaybackAdapter] Failed to unmute:', error)
      this.emit('error', 'Failed to unmute audio')
      throw error
    }
  }

  toggleMute(): void {
    this.ensureInitialized()

    try {
      this.wasmManager!.toggleMute()
      if (this.wasmManager!.isMuted()) {
        this.audioPlayer.setVolume(0)
        this.emit('muteChange', true)
      } else {
        const volume = this.wasmManager!.getVolume()
        this.audioPlayer.setVolume(volume)
        this.emit('muteChange', false)
      }
    } catch (error) {
      console.error('[WasmPlaybackAdapter] Failed to toggle mute:', error)
      this.emit('error', 'Failed to toggle mute')
      throw error
    }
  }

  getVolume(): number {
    if (!this.initialized || !this.wasmManager) {
      console.warn('[WasmPlaybackAdapter] getVolume called before initialization, returning default')
      return 100
    }

    try {
      return this.wasmManager.getVolume()
    } catch (error) {
      console.error('[WasmPlaybackAdapter] Failed to get volume:', error)
      // Return safe default
      return 100
    }
  }

  getIsMuted(): boolean {
    if (!this.initialized || !this.wasmManager) {
      console.warn('[WasmPlaybackAdapter] getIsMuted called before initialization, returning false')
      return false
    }

    try {
      return this.wasmManager.isMuted()
    } catch (error) {
      console.error('[WasmPlaybackAdapter] Failed to get mute state:', error)
      // Return safe default
      return false
    }
  }

  // ===== Seek =====

  seek(position: number): void {
    this.ensureInitialized()

    try {
      // Validate position
      // Clamp position to valid range instead of throwing error
      // This handles race conditions where UI calculates position with stale duration
      const duration = this.audioPlayer.duration
      let clampedPosition = position

      if (clampedPosition < 0) {
        console.warn('[WasmPlaybackAdapter] Seek position < 0, clamping to 0:', position)
        clampedPosition = 0
      }

      if (duration > 0 && clampedPosition > duration) {
        console.warn('[WasmPlaybackAdapter] Seek position exceeds duration, clamping:', {
          requested: position,
          duration,
          clamped: duration - 0.1
        })
        // Leave 0.1s buffer to avoid seeking exactly to end
        clampedPosition = Math.max(0, duration - 0.1)
      }

      this.audioPlayer.seek(clampedPosition)
      // WASM doesn't need to know about seek position
    } catch (error) {
      console.error('[WasmPlaybackAdapter] Failed to seek:', error)
      if (error instanceof Error) {
        throw error
      }
      const wrappedError = new Error('Failed to seek')
      this.emit('error', wrappedError.message)
      throw wrappedError
    }
  }

  seekPercent(percent: number): void {
    try {
      // Validate percent
      if (percent < 0 || percent > 100) {
        const errorMsg = `Invalid seek percent: ${percent} (must be 0-100)`
        console.error('[WasmPlaybackAdapter]', errorMsg)
        this.emit('error', errorMsg)
        throw new Error(errorMsg)
      }

      const position = (percent / 100) * this.audioPlayer.duration
      this.seek(position)
    } catch (error) {
      console.error('[WasmPlaybackAdapter] Failed to seek by percent:', error)
      if (error instanceof Error) {
        throw error
      }
      const wrappedError = new Error('Failed to seek')
      this.emit('error', wrappedError.message)
      throw wrappedError
    }
  }

  // ===== Getters =====

  getState(): PlaybackState {
    this.ensureInitialized()

    try {
      const stateStr = this.wasmManager!.getState()
      return this.mapWasmState(stateStr)
    } catch (error) {
      console.error('[WasmPlaybackAdapter] Failed to get state:', error)
      // Return safe default
      return 'stopped' as PlaybackState
    }
  }

  getCurrentTrack(): QueueTrack | null {
    return this.currentTrack
  }

  getPosition(): number {
    try {
      return this.audioPlayer.position
    } catch (error) {
      console.error('[WasmPlaybackAdapter] Failed to get position:', error)
      return 0
    }
  }

  getDuration(): number {
    try {
      return this.audioPlayer.duration
    } catch (error) {
      console.error('[WasmPlaybackAdapter] Failed to get duration:', error)
      return 0
    }
  }

  hasNext(): boolean {
    if (!this.initialized || !this.wasmManager) {
      console.warn('[WasmPlaybackAdapter] hasNext called before initialization, returning false')
      return false
    }

    try {
      return this.wasmManager.hasNext()
    } catch (error) {
      console.error('[WasmPlaybackAdapter] Failed to check hasNext:', error)
      return false
    }
  }

  hasPrevious(): boolean {
    if (!this.initialized || !this.wasmManager) {
      console.warn('[WasmPlaybackAdapter] hasPrevious called before initialization, returning false')
      return false
    }

    try {
      return this.wasmManager.hasPrevious()
    } catch (error) {
      console.error('[WasmPlaybackAdapter] Failed to check hasPrevious:', error)
      return false
    }
  }

  getHistory(): QueueTrack[] {
    if (!this.initialized || !this.wasmManager) {
      console.warn('[WasmPlaybackAdapter] getHistory called before initialization, returning empty array')
      return []
    }

    try {
      const wasmHistory = this.wasmManager.getHistory()
      return wasmHistory.map((t: WasmTrackData) => this.mapWasmTrack(t))
    } catch (error) {
      console.error('[WasmPlaybackAdapter] Failed to get history:', error)
      return []
    }
  }

  // ===== Event Emitter =====

  /**
   * Subscribe to playback events
   *
   * Events are emitted automatically when state changes. Never manually emit events
   * from application code - this creates duplicate events and desync.
   *
   * Available events:
   * - 'stateChange': (state: PlaybackState) => void - Playback state changed
   * - 'trackChange': (track: QueueTrack | null) => void - Current track changed
   * - 'positionUpdate': (position: number) => void - Playback position updated (every ~100ms)
   * - 'volumeChange': (volume: number) => void - Volume changed (0-100)
   * - 'queueChange': () => void - Queue modified (track added/removed/reordered)
   * - 'shuffleChange': (mode: ShuffleMode) => void - Shuffle mode changed
   * - 'repeatChange': (mode: RepeatMode) => void - Repeat mode changed
   * - 'muteChange': (muted: boolean) => void - Mute state changed
   * - 'error': (error: string | Error) => void - Error occurred
   *
   * @param event - Event name
   * @param callback - Function to call when event fires
   * @returns Cleanup function to unsubscribe
   *
   * @example
   * ```typescript
   * const cleanup = adapter.on('trackChange', (track) => {
   *   console.log('Now playing:', track?.title);
   * });
   *
   * // Later: unsubscribe
   * cleanup();
   * ```
   */
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

  /**
   * Emit event asynchronously to avoid Rust RefCell borrow conflicts.
   * Use this when emitting from within WASM method calls to prevent
   * callbacks from recursively calling back into WASM.
   */
  private deferredEmit(event: string, ...args: any[]): void {
    setTimeout(() => {
      this.emit(event, ...args)
    }, 0)
  }

  // ===== State Synchronization =====

  /**
   * Start periodic state synchronization to detect desync between
   * TypeScript queue state and WASM queue state
   */
  private startStateSyncInterval(): void {
    if (this.stateSyncIntervalId !== null) {
      return // Already running
    }

    console.log('[WasmPlaybackAdapter] Starting state sync interval (every', this.STATE_SYNC_INTERVAL_MS, 'ms)')
    this.stateSyncIntervalId = window.setInterval(() => {
      this.syncQueueState()
    }, this.STATE_SYNC_INTERVAL_MS)
  }

  /**
   * Stop periodic state synchronization
   */
  private stopStateSyncInterval(): void {
    if (this.stateSyncIntervalId !== null) {
      clearInterval(this.stateSyncIntervalId)
      this.stateSyncIntervalId = null
      console.log('[WasmPlaybackAdapter] Stopped state sync interval')
    }
  }

  /**
   * Synchronize queue state between TypeScript and WASM
   * Detects and logs any inconsistencies
   */
  private syncQueueState(): void {
    if (!this.initialized || !this.wasmManager) {
      return // Not initialized yet, skip sync
    }

    try {
      // Get current queue state from WASM
      const wasmQueueLength = this.wasmManager.queueLength()
      const wasmCurrentTrack = this.currentTrack

      // Log sync check (debug level - only in dev)
      if (process.env.NODE_ENV === 'development') {
        console.debug('[WasmPlaybackAdapter] State sync check:', {
          queueLength: wasmQueueLength,
          currentTrack: wasmCurrentTrack?.title || 'None',
          audioState: this.audioPlaybackState
        })
      }

      // Detect queue length mismatch
      // Note: We don't track queue length in TypeScript, so we just validate WASM state
      if (wasmQueueLength < 0) {
        console.warn('[WasmPlaybackAdapter] WARNING: WASM queue length is negative!', wasmQueueLength)
      }

      // Detect current track mismatch with audio player
      if (wasmCurrentTrack && this.audioPlaybackState === 'playing') {
        const audioPosition = this.audioPlayer.position
        const audioDuration = this.audioPlayer.duration

        // Check if audio finished but we didn't advance (rare case)
        if (audioDuration > 0 && audioPosition >= audioDuration - 0.1) {
          console.warn('[WasmPlaybackAdapter] WARNING: Audio finished but WASM still shows playing track')
        }
      }

    } catch (error) {
      console.error('[WasmPlaybackAdapter] State sync failed:', error)
    }
  }

  /**
   * Manually trigger state synchronization
   * Can be called by external code to force a sync check
   */
  forceSyncQueueState(): void {
    console.log('[WasmPlaybackAdapter] Manual state sync triggered')
    this.syncQueueState()
  }

  // ===== Cleanup =====

  destroy(): void {
    this.stop()
    this.stopStateSyncInterval()
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
      if (removed) {
        const mappedTrack = this.mapWasmTrack(removed)
        // Emit queue change when track is successfully removed
        console.log('[WasmPlaybackAdapter] Emitting queueChange (removed track at index', index, ':', mappedTrack.title, ', deferred)')
        this.deferredEmit('queueChange')
        return mappedTrack
      }
      return null
    } catch (error) {
      console.error('[WasmPlaybackAdapter] Failed to remove from queue:', error)
      this.emit('error', 'Failed to remove track from queue')
      return null
    }
  }

  appendToQueue(tracks: QueueTrack[]): void {
    this.ensureInitialized()

    try {
      // Validate tracks array
      if (!tracks || tracks.length === 0) {
        const errorMsg = 'Cannot append to queue - no tracks provided'
        console.error('[WasmPlaybackAdapter]', errorMsg)
        this.emit('error', errorMsg)
        throw new Error(errorMsg)
      }

      // Validate tracks have required fields
      const invalidTracks = tracks.filter(t => !t.id || !t.path || !t.title || !t.artist)
      if (invalidTracks.length > 0) {
        const errorMsg = `Cannot append to queue - ${invalidTracks.length} track(s) missing required fields`
        console.error('[WasmPlaybackAdapter]', errorMsg, invalidTracks)
        this.emit('error', errorMsg)
        throw new Error(errorMsg)
      }

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
      this.deferredEmit('queueChange')
      console.log('[WasmPlaybackAdapter] Appended', tracks.length, 'tracks to queue')
    } catch (error) {
      console.error('[WasmPlaybackAdapter] Failed to append to queue:', error)
      if (error instanceof Error) {
        throw error
      }
      const wrappedError = new Error('Failed to append tracks to queue')
      this.emit('error', wrappedError.message)
      throw wrappedError
    }
  }

  clearPlayNext(): void {
    this.ensureInitialized()
    // Note: WASM manager may not have this method yet, stub for now
    // TODO: Implement in WASM if needed
    console.warn('[WasmPlaybackAdapter] clearPlayNext() not implemented in WASM')
  }

  clearAddToQueue(): void {
    this.ensureInitialized()
    // Note: WASM manager may not have this method yet, stub for now
    // TODO: Implement in WASM if needed
    console.warn('[WasmPlaybackAdapter] clearAddToQueue() not implemented in WASM')
  }
}
