/**
 * Web Audio API player
 * Replaces CPAL + LocalAudioSource from desktop implementation
 */

export type AudioEventCallback = () => void
export type AudioPositionCallback = (position: number) => void
export type AudioErrorCallback = (error: Error) => void

export class WebAudioPlayer {
  private audioContext: AudioContext
  private audioElement: HTMLAudioElement
  private gainNode: GainNode
  private sourceNode?: MediaElementAudioSourceNode

  constructor() {
    this.audioContext = new AudioContext()
    this.audioElement = new Audio()
    this.audioElement.preload = 'auto'

    // Create gain node for volume control
    this.gainNode = this.audioContext.createGain()
    this.gainNode.connect(this.audioContext.destination)
  }

  async loadTrack(url: string): Promise<void> {
    console.log('[WebAudioPlayer] Loading track:', url)

    // Stop current playback
    this.audioElement.pause()
    this.audioElement.currentTime = 0
    this.audioElement.src = url

    // Connect to Web Audio API (for effects support later)
    if (!this.sourceNode) {
      this.sourceNode = this.audioContext.createMediaElementSource(
        this.audioElement
      )
      this.sourceNode.connect(this.gainNode)
    }

    // Resume audio context if suspended (browser autoplay policy)
    if (this.audioContext.state === 'suspended') {
      console.log('[WebAudioPlayer] Resuming suspended audio context')
      await this.audioContext.resume()
    }

    // Load the audio
    return new Promise((resolve, reject) => {
      const onCanPlay = () => {
        console.log('[WebAudioPlayer] Track loaded and ready')
        resolve()
      }

      const onError = (e: Event) => {
        const error = this.audioElement.error
        console.error('[WebAudioPlayer] Failed to load audio:', {
          url,
          error: error?.message,
          code: error?.code,
          event: e
        })
        reject(new Error(`Failed to load audio: ${error?.message || 'Unknown error'}`))
      }

      this.audioElement.addEventListener('canplay', onCanPlay, { once: true })
      this.audioElement.addEventListener('error', onError, { once: true })
      this.audioElement.load()
    })
  }

  async play(): Promise<void> {
    // Resume audio context if suspended (browser autoplay policy)
    if (this.audioContext.state === 'suspended') {
      console.log('[WebAudioPlayer] Resuming audio context before play')
      await this.audioContext.resume()
    }

    try {
      await this.audioElement.play()
      console.log('[WebAudioPlayer] Playback started')
    } catch (err) {
      console.error('[WebAudioPlayer] Playback failed:', err)
      throw err
    }
  }

  pause(): void {
    this.audioElement.pause()
  }

  stop(): void {
    this.audioElement.pause()
    this.audioElement.currentTime = 0
  }

  seek(position: number): void {
    this.audioElement.currentTime = position
  }

  setVolume(level: number): void {
    // Logarithmic volume scaling (0-100 -> 0.0-1.0)
    // Using quadratic curve for more natural volume feel
    const normalized = Math.max(0, Math.min(100, level)) / 100
    this.gainNode.gain.value = normalized * normalized
  }

  get position(): number {
    return this.audioElement.currentTime
  }

  get duration(): number {
    return this.audioElement.duration || 0
  }

  get isFinished(): boolean {
    return this.audioElement.ended
  }

  get isPaused(): boolean {
    return this.audioElement.paused
  }

  // Event listeners
  onTimeUpdate(callback: AudioPositionCallback): () => void {
    const handler = () => callback(this.position)
    this.audioElement.addEventListener('timeupdate', handler)
    return () => this.audioElement.removeEventListener('timeupdate', handler)
  }

  onEnded(callback: AudioEventCallback): () => void {
    this.audioElement.addEventListener('ended', callback)
    return () => this.audioElement.removeEventListener('ended', callback)
  }

  onError(callback: AudioErrorCallback): () => void {
    const handler = () => callback(new Error('Audio playback error'))
    this.audioElement.addEventListener('error', handler)
    return () => this.audioElement.removeEventListener('error', handler)
  }

  onPlay(callback: AudioEventCallback): () => void {
    this.audioElement.addEventListener('play', callback)
    return () => this.audioElement.removeEventListener('play', callback)
  }

  onPause(callback: AudioEventCallback): () => void {
    this.audioElement.addEventListener('pause', callback)
    return () => this.audioElement.removeEventListener('pause', callback)
  }

  onLoadStart(callback: AudioEventCallback): () => void {
    this.audioElement.addEventListener('loadstart', callback)
    return () => this.audioElement.removeEventListener('loadstart', callback)
  }

  // Cleanup
  destroy(): void {
    this.audioElement.pause()
    this.audioElement.src = ''
    if (this.sourceNode) {
      this.sourceNode.disconnect()
    }
    this.gainNode.disconnect()
  }
}
