/* tslint:disable */
/* eslint-disable */

export class WasmPlaybackManager {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Get current repeat mode
   */
  getRepeat(): string;
  /**
   * Get current volume (0-100)
   */
  getVolume(): number;
  /**
   * Set repeat mode ("off" | "all" | "one")
   */
  setRepeat(mode: string): void;
  /**
   * Set volume (0-100)
   */
  setVolume(level: number): void;
  /**
   * Clear entire queue
   */
  clearQueue(): void;
  /**
   * Get playback history
   */
  getHistory(): any;
  /**
   * Get current shuffle mode
   */
  getShuffle(): string;
  /**
   * Set shuffle mode ("off" | "random" | "smart")
   */
  setShuffle(mode: string): void;
  /**
   * Toggle mute
   */
  toggleMute(): void;
  /**
   * Get duration of current track in seconds
   */
  getDuration(): number | undefined;
  /**
   * Get current position in seconds
   */
  getPosition(): number;
  /**
   * Check if there is a previous track
   */
  hasPrevious(): boolean;
  /**
   * Get queue length
   */
  queueLength(): number;
  /**
   * Load playlist as source queue
   */
  loadPlaylist(tracks: any): void;
  /**
   * Append tracks to existing queue
   */
  appendToQueue(tracks: any): void;
  /**
   * Register queue change callback
   */
  onQueueChange(callback: Function): void;
  /**
   * Register state change callback
   */
  onStateChange(callback: Function): void;
  /**
   * Register track change callback
   */
  onTrackChange(callback: Function): void;
  /**
   * Seek to position by percentage (0.0 - 1.0)
   */
  seekToPercent(percent: number): void;
  /**
   * Add track to end of queue (explicit queue)
   */
  addToQueueEnd(track: WasmQueueTrack): void;
  /**
   * Add track to play next (explicit queue)
   */
  addToQueueNext(track: WasmQueueTrack): void;
  /**
   * Remove track from queue by index
   */
  removeFromQueue(index: number): any;
  /**
   * Skip to track at queue index
   */
  skipToQueueIndex(index: number): void;
  /**
   * Create a new playback manager
   */
  constructor();
  /**
   * Mute audio
   */
  mute(): void;
  /**
   * Skip to next track
   */
  next(): void;
  /**
   * Start or resume playback
   */
  play(): void;
  /**
   * Stop playback
   */
  stop(): void;
  /**
   * Pause playback
   */
  pause(): void;
  /**
   * Unmute audio
   */
  unmute(): void;
  /**
   * Seek to position in seconds
   */
  seekTo(position_secs: number): void;
  /**
   * Check if there is a next track
   */
  hasNext(): boolean;
  /**
   * Check if muted
   */
  isMuted(): boolean;
  /**
   * Register error callback
   */
  onError(callback: Function): void;
  /**
   * Go to previous track
   */
  previous(): void;
  /**
   * Get all tracks in queue as JSON
   */
  getQueue(): any;
  /**
   * Get current playback state as string
   */
  getState(): string;
}

export class WasmQueueTrack {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a new queue track
   */
  constructor(id: string, path: string, title: string, artist: string, duration_secs: number);
  get trackNumber(): number | undefined;
  set trackNumber(value: number | null | undefined);
  readonly durationSecs: number;
  readonly id: string;
  readonly path: string;
  get album(): string | undefined;
  set album(value: string | null | undefined);
  readonly title: string;
  readonly artist: string;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_wasmplaybackmanager_free: (a: number, b: number) => void;
  readonly __wbg_wasmqueuetrack_free: (a: number, b: number) => void;
  readonly wasmplaybackmanager_addToQueueEnd: (a: number, b: number) => void;
  readonly wasmplaybackmanager_addToQueueNext: (a: number, b: number) => void;
  readonly wasmplaybackmanager_appendToQueue: (a: number, b: number, c: number) => void;
  readonly wasmplaybackmanager_clearQueue: (a: number) => void;
  readonly wasmplaybackmanager_getDuration: (a: number, b: number) => void;
  readonly wasmplaybackmanager_getHistory: (a: number) => number;
  readonly wasmplaybackmanager_getPosition: (a: number) => number;
  readonly wasmplaybackmanager_getQueue: (a: number) => number;
  readonly wasmplaybackmanager_getRepeat: (a: number, b: number) => void;
  readonly wasmplaybackmanager_getShuffle: (a: number, b: number) => void;
  readonly wasmplaybackmanager_getState: (a: number, b: number) => void;
  readonly wasmplaybackmanager_getVolume: (a: number) => number;
  readonly wasmplaybackmanager_hasNext: (a: number) => number;
  readonly wasmplaybackmanager_hasPrevious: (a: number) => number;
  readonly wasmplaybackmanager_isMuted: (a: number) => number;
  readonly wasmplaybackmanager_loadPlaylist: (a: number, b: number, c: number) => void;
  readonly wasmplaybackmanager_mute: (a: number) => void;
  readonly wasmplaybackmanager_new: () => number;
  readonly wasmplaybackmanager_next: (a: number, b: number) => void;
  readonly wasmplaybackmanager_onError: (a: number, b: number) => void;
  readonly wasmplaybackmanager_onQueueChange: (a: number, b: number) => void;
  readonly wasmplaybackmanager_onStateChange: (a: number, b: number) => void;
  readonly wasmplaybackmanager_onTrackChange: (a: number, b: number) => void;
  readonly wasmplaybackmanager_pause: (a: number) => void;
  readonly wasmplaybackmanager_play: (a: number, b: number) => void;
  readonly wasmplaybackmanager_previous: (a: number, b: number) => void;
  readonly wasmplaybackmanager_queueLength: (a: number) => number;
  readonly wasmplaybackmanager_removeFromQueue: (a: number, b: number, c: number) => void;
  readonly wasmplaybackmanager_seekTo: (a: number, b: number, c: number) => void;
  readonly wasmplaybackmanager_seekToPercent: (a: number, b: number, c: number) => void;
  readonly wasmplaybackmanager_setRepeat: (a: number, b: number, c: number, d: number) => void;
  readonly wasmplaybackmanager_setShuffle: (a: number, b: number, c: number, d: number) => void;
  readonly wasmplaybackmanager_setVolume: (a: number, b: number) => void;
  readonly wasmplaybackmanager_skipToQueueIndex: (a: number, b: number, c: number) => void;
  readonly wasmplaybackmanager_stop: (a: number) => void;
  readonly wasmplaybackmanager_toggleMute: (a: number) => void;
  readonly wasmplaybackmanager_unmute: (a: number) => void;
  readonly wasmqueuetrack_album: (a: number, b: number) => void;
  readonly wasmqueuetrack_artist: (a: number, b: number) => void;
  readonly wasmqueuetrack_durationSecs: (a: number) => number;
  readonly wasmqueuetrack_id: (a: number, b: number) => void;
  readonly wasmqueuetrack_new: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => number;
  readonly wasmqueuetrack_path: (a: number, b: number) => void;
  readonly wasmqueuetrack_set_album: (a: number, b: number, c: number) => void;
  readonly wasmqueuetrack_set_trackNumber: (a: number, b: number) => void;
  readonly wasmqueuetrack_title: (a: number, b: number) => void;
  readonly wasmqueuetrack_trackNumber: (a: number) => number;
  readonly __wbindgen_export: (a: number, b: number) => number;
  readonly __wbindgen_export2: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_export3: (a: number) => void;
  readonly __wbindgen_export4: (a: number, b: number, c: number) => void;
  readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
