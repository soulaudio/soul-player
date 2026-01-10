/**
 * Demo playback system exports
 * Provides JSON-based storage and Web Audio playback for marketing demo
 *
 * NOTE: Playback logic now uses WASM (soul-playback via wasm-playback-adapter.ts)
 * TypeScript implementation (DemoPlaybackManager) has been removed to eliminate duplication
 */

export * from './types'
export * from './storage'
export * from './audio-player'
export * from './wasm-playback-adapter'
export * from './bridge'
