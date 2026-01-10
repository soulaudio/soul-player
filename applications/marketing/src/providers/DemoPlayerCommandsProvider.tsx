/**
 * Demo implementation of PlayerCommands context
 * Bridges web audio playback to shared PlayerCommands interface
 */

import { ReactNode, useMemo } from 'react';
import {
  PlayerCommandsProvider,
  type PlayerContextValue,
  type PlayerCommandsInterface,
  type PlaybackEventsInterface,
  type PlaybackCapabilities,
} from '@soul-player/shared';
import { getManager, getManagerSync } from '@/lib/demo/bridge';
import { PlaybackState } from '@/lib/demo/types';

export function DemoPlayerCommandsProvider({ children }: { children: ReactNode }) {
  const value = useMemo<PlayerContextValue>(() => {
    // Ensure bridge initialization is started (async)
    getManager().catch(console.error);

    // Helper to get manager, throws if not initialized yet
    const getManagerOrThrow = () => {
      const manager = getManagerSync();
      if (!manager) {
        throw new Error('WASM playback manager not initialized yet');
      }
      return manager;
    };

    // Commands implementation using demo playback manager
    const commands: PlayerCommandsInterface = {
      async playTrack(trackId: string | number) {
        const storage = (await import('@/lib/demo/storage')).getDemoStorage();
        const track = storage.getTrackById(String(trackId));
        if (!track) throw new Error(`Track ${trackId} not found`);

        const queueTrack = storage.toQueueTrack(track);
        getManagerOrThrow().clearQueue();
        getManagerOrThrow().addToQueueNext(queueTrack);
        await getManagerOrThrow().play();
      },

      async pausePlayback() {
        getManagerOrThrow().pause();
      },

      async resumePlayback() {
        await getManagerOrThrow().play();
      },

      async stopPlayback() {
        getManagerOrThrow().stop();
      },

      async skipNext() {
        await getManagerOrThrow().next();
      },

      async skipPrevious() {
        await getManagerOrThrow().previous();
      },

      async seek(position: number) {
        getManagerOrThrow().seek(position);
      },

      async setVolume(volume: number) {
        // Demo manager expects 0-100, shared interface uses 0-1
        const volumePercent = Math.max(0, Math.min(100, Math.round(volume * 100)));
        console.log('[DemoPlayerCommandsProvider] Setting volume:', { volume, volumePercent });
        getManagerOrThrow().setVolume(volumePercent);
      },

      async setShuffle(enabled: boolean) {
        const ShuffleMode = (await import('@/lib/demo/types')).ShuffleMode;
        getManagerOrThrow().setShuffle(enabled ? ShuffleMode.Random : ShuffleMode.Off);
      },

      async setRepeatMode(mode: 'off' | 'all' | 'one') {
        const RepeatMode = (await import('@/lib/demo/types')).RepeatMode;
        const modeMap = {
          off: RepeatMode.Off,
          all: RepeatMode.All,
          one: RepeatMode.One,
        };
        getManagerOrThrow().setRepeat(modeMap[mode]);
      },

      async getPlaybackCapabilities(): Promise<PlaybackCapabilities> {
        return {
          hasNext: getManagerOrThrow().hasNext(),
          hasPrevious: getManagerOrThrow().hasPrevious(),
        };
      },

      async getQueue() {
        // Demo: Return current queue from manager with cover art
        // Note: coverUrl is not stored in WASM, so we look it up from demo storage
        const storage = (await import('@/lib/demo/storage')).getDemoStorage();

        return getManagerOrThrow().getQueue().map((track) => {
          // Look up demo track to get coverUrl
          const demoTrack = storage.getTrackById(track.id);

          return {
            trackId: track.id,
            title: track.title,
            artist: track.artist,
            album: track.album || null,
            filePath: track.path,
            durationSeconds: track.duration_secs || null,  // Use correct field name
            trackNumber: track.track_number || null,  // Use correct field name
            coverArtPath: demoTrack?.coverUrl || null,  // Look up from storage
          };
        });
      },

      async playQueue(queue, startIndex = 0) {
        console.log('[DemoPlayerCommandsProvider] playQueue called:', {
          queueLength: queue.length,
          startIndex,
          firstTrack: queue[0]?.title
        });

        // Convert QueueTrack[] to demo QueueTrack format
        const storage = (await import('@/lib/demo/storage')).getDemoStorage();
        const demoQueue = queue.map(track => {
          const demoTrack = storage.getTrackById(track.trackId);
          if (!demoTrack) {
            console.error('[DemoPlayerCommandsProvider] Demo track not found:', track.trackId);
          }
          return {
            id: track.trackId,
            title: track.title,
            artist: track.artist,
            album: track.album || undefined,
            path: track.filePath,
            duration_secs: track.durationSeconds || 0,  // WASM expects duration_secs (underscore)
            track_number: track.trackNumber || undefined,
            coverUrl: demoTrack?.coverUrl,
          };
        });

        console.log('[DemoPlayerCommandsProvider] Converted to demo queue:', {
          length: demoQueue.length,
          firstPath: demoQueue[0]?.path,
          allHavePaths: demoQueue.every(t => t.path)
        });

        // Load the queue starting from the specified index
        const reorderedQueue = [
          ...demoQueue.slice(startIndex),
          ...demoQueue.slice(0, startIndex),
        ];

        console.log('[DemoPlayerCommandsProvider] Loading playlist to WASM, starting track:', reorderedQueue[0]?.title);

        // IMPORTANT: Stop current playback first (Spotify behavior)
        // This ensures clicking play starts fresh, doesn't append
        try {
          getManagerOrThrow().stop();
          getManagerOrThrow().loadPlaylist(reorderedQueue);
          await getManagerOrThrow().play();
          console.log('[DemoPlayerCommandsProvider] Playback started successfully');
        } catch (error) {
          console.error('[DemoPlayerCommandsProvider] Failed to start playback:', error);
          throw error;
        }
      },

      async skipToQueueIndex(index: number) {
        // Use the manager's built-in method that maintains history
        await getManagerOrThrow().skipToQueueIndex(index);
      },

      async getAllSources() {
        // Demo: Return mock sources
        return [
          {
            id: 1,
            name: 'Demo Library',
            sourceType: 'local',
            isActive: true,
            isOnline: true,
          },
        ];
      },
    };

    // Events implementation using demo manager event emitter
    const events: PlaybackEventsInterface = {
      onStateChange(callback) {
        const handler = (state: PlaybackState) => {
          callback(state === PlaybackState.Playing);
        };
        return getManagerOrThrow().on('stateChange', handler);
      },

      onTrackChange(callback) {
        return getManagerOrThrow().on('trackChange', callback);
      },

      onPositionUpdate(callback) {
        return getManagerOrThrow().on('positionUpdate', callback);
      },

      onVolumeChange(callback) {
        return getManagerOrThrow().on('volumeChange', callback);
      },

      onQueueUpdate(callback) {
        return getManagerOrThrow().on('queueChange', callback);
      },

      onError(callback) {
        return getManagerOrThrow().on('error', callback);
      },
    };

    return { commands, events };
  }, []);

  return <PlayerCommandsProvider value={value}>{children}</PlayerCommandsProvider>;
}
