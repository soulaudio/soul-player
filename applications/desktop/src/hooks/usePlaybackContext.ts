/**
 * Hook for managing playback context tracking
 *
 * Tracks what context (album, playlist, artist, etc.) the user is playing from
 * for "Jump Back Into" and "Now Playing" context display.
 */

import { useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';

export type ContextType = 'album' | 'playlist' | 'artist' | 'genre' | 'tracks';

export interface PlaybackContext {
  id: number;
  contextType: ContextType;
  contextId: string | null;
  contextName: string | null;
  contextArtworkPath: string | null;
  lastPlayedAt: number;
}

export interface RecordContextInput {
  contextType: ContextType;
  contextId?: string | null;
  contextName?: string | null;
  contextArtworkPath?: string | null;
}

/**
 * Hook for recording and retrieving playback contexts
 */
export function usePlaybackContext() {
  /**
   * Record that playback started from a specific context
   */
  const recordContext = useCallback(async (input: RecordContextInput) => {
    try {
      await invoke('record_playback_context', {
        input: {
          contextType: input.contextType,
          contextId: input.contextId ?? null,
          contextName: input.contextName ?? null,
          contextArtworkPath: input.contextArtworkPath ?? null,
        },
      });
    } catch (error) {
      console.error('[usePlaybackContext] Failed to record context:', error);
    }
  }, []);

  /**
   * Get recent playback contexts for "Jump Back Into" section
   */
  const getRecentContexts = useCallback(async (limit: number = 10): Promise<PlaybackContext[]> => {
    try {
      const contexts = await invoke<PlaybackContext[]>('get_recent_playback_contexts', { limit });
      return contexts;
    } catch (error) {
      console.error('[usePlaybackContext] Failed to get recent contexts:', error);
      return [];
    }
  }, []);

  /**
   * Get the current (most recent) playback context
   */
  const getCurrentContext = useCallback(async (): Promise<PlaybackContext | null> => {
    try {
      const context = await invoke<PlaybackContext | null>('get_current_playback_context');
      return context;
    } catch (error) {
      console.error('[usePlaybackContext] Failed to get current context:', error);
      return null;
    }
  }, []);

  /**
   * Clear all playback context history
   */
  const clearHistory = useCallback(async (): Promise<void> => {
    try {
      await invoke('clear_playback_context_history');
    } catch (error) {
      console.error('[usePlaybackContext] Failed to clear history:', error);
    }
  }, []);

  return {
    recordContext,
    getRecentContexts,
    getCurrentContext,
    clearHistory,
  };
}
