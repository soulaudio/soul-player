/**
 * @soul-player/playback-web
 *
 * Web-based audio playback library using WASM and Web Audio API.
 *
 * This library provides a reusable playback engine for browser-based environments,
 * used by both the marketing demo and future web player applications.
 *
 * @module @soul-player/playback-web
 */

// Core playback adapter (bridges WASM and Web Audio)
export { WasmPlaybackAdapter } from './wasm-adapter'

// Web Audio player
export { WebAudioPlayer } from './audio-player'

// Type definitions
export * from './types'
export type {
  AudioEventCallback,
  AudioPositionCallback,
  AudioErrorCallback
} from './audio-player'

// Conversion utilities
export { toQueueTrack, tracksToQueue } from './converters'

// WASM bindings (for advanced usage)
export { default as init, WasmPlaybackManager, WasmQueueTrack } from './wasm/soul_playback'

export const VERSION = '0.1.3'
