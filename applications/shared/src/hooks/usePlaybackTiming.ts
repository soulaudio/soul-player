import { useState, useEffect } from 'react';
import { DEFAULT_TIMING_CONFIG, type PlaybackTimingConfig } from '../types/playback-timing';
import { debug } from '../utils/debug';

/**
 * Hook to fetch playback timing configuration from backend
 *
 * This ensures frontend timing (e.g., ignore window after seek) is synchronized
 * with backend timing (position update interval).
 *
 * The configuration is fetched once on mount and cached for the component lifetime.
 * Falls back to DEFAULT_TIMING_CONFIG if fetch fails or on non-desktop platforms.
 */
export function usePlaybackTiming(): PlaybackTimingConfig {
  const [config, setConfig] = useState<PlaybackTimingConfig>(DEFAULT_TIMING_CONFIG);

  useEffect(() => {
    // Only fetch on desktop platform
    if (typeof window !== 'undefined' && '__TAURI__' in window) {
      const tauri = (window as any).__TAURI__;
      if (tauri?.core?.invoke) {
        tauri.core.invoke('get_playback_timing_config')
          .then((backendConfig: PlaybackTimingConfig) => {
            debug.log('[usePlaybackTiming] Loaded timing config from backend:', backendConfig);
            setConfig(backendConfig);
          })
          .catch((error: Error) => {
            debug.warn('[usePlaybackTiming] Failed to fetch timing config, using defaults:', error);
            // Keep using DEFAULT_TIMING_CONFIG
          });
      }
    } else {
      debug.log('[usePlaybackTiming] Not on desktop platform, using default timing config');
    }
  }, []);

  return config;
}
