/**
 * Demo playback system exports
 * Provides JSON-based storage and Web Audio playback for marketing demo
 *
 * NOTE: Playback logic now uses WASM (soul-playback via @soul-player/playback-web)
 * TypeScript implementation (DemoPlaybackManager) has been removed to eliminate duplication
 *
 * NOTE: bridge.ts has been integrated into WebPlaybackProvider (@soul-player/shared)
 * Event wiring between WASM and store is now handled automatically by WebPlaybackProvider
 */

// Re-export playback components from the new library
export * from '@soul-player/playback-web'
