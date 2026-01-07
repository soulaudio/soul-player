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
import { getManager } from '@/lib/demo/bridge';
import { PlaybackState } from '@/lib/demo/types';

export function DemoPlayerCommandsProvider({ children }: { children: ReactNode }) {
  const value = useMemo<PlayerContextValue>(() => {
    // Ensure bridge is initialized
    const manager = getManager();
    console.log('[DemoPlayerCommandsProvider] Manager initialized:', manager);

    // Commands implementation using demo playback manager
    const commands: PlayerCommandsInterface = {
      async playTrack(trackId: string | number) {
        const storage = (await import('@/lib/demo/storage')).getDemoStorage();
        const track = storage.getTrackById(String(trackId));
        if (!track) throw new Error(`Track ${trackId} not found`);

        const queueTrack = storage.toQueueTrack(track);
        manager.clearQueue();
        manager.addToQueueNext(queueTrack);
        await manager.play();
      },

      async pausePlayback() {
        manager.pause();
      },

      async resumePlayback() {
        await manager.play();
      },

      async stopPlayback() {
        manager.stop();
      },

      async skipNext() {
        await manager.next();
      },

      async skipPrevious() {
        await manager.previous();
      },

      async seek(position: number) {
        manager.seek(position);
      },

      async setVolume(volume: number) {
        // Demo manager expects 0-100, shared interface uses 0-1
        const volumePercent = Math.max(0, Math.min(100, Math.round(volume * 100)));
        console.log('[DemoPlayerCommandsProvider] Setting volume:', { volume, volumePercent });
        manager.setVolume(volumePercent);
      },

      async setShuffle(enabled: boolean) {
        const ShuffleMode = (await import('@/lib/demo/types')).ShuffleMode;
        manager.setShuffle(enabled ? ShuffleMode.Random : ShuffleMode.Off);
      },

      async setRepeatMode(mode: 'off' | 'all' | 'one') {
        const RepeatMode = (await import('@/lib/demo/types')).RepeatMode;
        const modeMap = {
          off: RepeatMode.Off,
          all: RepeatMode.All,
          one: RepeatMode.One,
        };
        manager.setRepeat(modeMap[mode]);
      },

      async getPlaybackCapabilities(): Promise<PlaybackCapabilities> {
        return {
          hasNext: manager.hasNext(),
          hasPrevious: manager.hasPrevious(),
        };
      },

      async getQueue() {
        // Demo: Return current queue from manager
        return manager.getQueue().map((track, index) => ({
          trackId: track.id,
          title: track.title,
          artist: track.artist,
          album: track.album || null,
          filePath: track.path,
          durationSeconds: track.duration || null,
          trackNumber: null,
        }));
      },

      async playQueue(queue, startIndex = 0) {
        // Convert QueueTrack[] to demo QueueTrack format
        const demoQueue = queue.map(track => ({
          id: track.trackId,
          title: track.title,
          artist: track.artist,
          album: track.album || undefined,
          path: track.filePath,
          duration: track.durationSeconds || 0,
          source: { type: 'single' as const },
        }));

        // Load the queue starting from the specified index
        const reorderedQueue = [
          ...demoQueue.slice(startIndex),
          ...demoQueue.slice(0, startIndex),
        ];

        // IMPORTANT: Stop current playback first (Spotify behavior)
        // This ensures clicking play starts fresh, doesn't append
        manager.stop();
        manager.loadPlaylist(reorderedQueue);
        await manager.play();
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
        return manager.on('stateChange', handler);
      },

      onTrackChange(callback) {
        return manager.on('trackChange', callback);
      },

      onPositionUpdate(callback) {
        return manager.on('positionUpdate', callback);
      },

      onVolumeChange(callback) {
        return manager.on('volumeChange', callback);
      },

      onQueueUpdate(callback) {
        return manager.on('queueChange', callback);
      },

      onError(callback) {
        return manager.on('error', callback);
      },
    };

    return { commands, events };
  }, []);

  return <PlayerCommandsProvider value={value}>{children}</PlayerCommandsProvider>;
}
