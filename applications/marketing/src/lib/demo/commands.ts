/**
 * Demo commands that replace Tauri invoke()
 * Same interface as desktop playerCommands
 */

import { getManager } from './bridge'
import { getDemoStorage } from './storage'
import { RepeatMode, ShuffleMode } from './types'

export const playerCommands = {
  async playTrack(trackId: string) {
    const manager = getManager()
    const storage = getDemoStorage()

    const track = storage.getTrackById(trackId)
    if (!track) throw new Error(`Track ${trackId} not found`)

    const queueTrack = storage.toQueueTrack(track)

    // Clear and play just this track
    manager.clearQueue()
    manager.addToQueueNext(queueTrack)
    await manager.play()
  },

  async playTracks(trackIds: string[], startIndex: number = 0) {
    const manager = getManager()
    const storage = getDemoStorage()

    const tracks = trackIds
      .map(id => storage.getTrackById(id))
      .filter((t): t is NonNullable<typeof t> => t !== null)
      .map(t => storage.toQueueTrack(t))

    if (tracks.length === 0) throw new Error('No valid tracks')

    // Reorder so startIndex track plays first
    const reordered = [...tracks.slice(startIndex), ...tracks.slice(0, startIndex)]

    manager.clearQueue()
    manager.loadPlaylist(reordered)
    await manager.play()
  },

  async pausePlayback() {
    getManager().pause()
  },

  async resumePlayback() {
    await getManager().play()
  },

  async skipNext() {
    await getManager().next()
  },

  async skipPrevious() {
    await getManager().previous()
  },

  async seekTo(position: number) {
    getManager().seek(position)
  },

  async setVolume(volume: number) {
    getManager().setVolume(Math.round(volume * 100)) // 0-1 to 0-100
  },

  async setRepeatMode(mode: 'off' | 'all' | 'one') {
    const modeMap: Record<string, RepeatMode> = {
      off: RepeatMode.Off,
      all: RepeatMode.All,
      one: RepeatMode.One,
    }
    getManager().setRepeat(modeMap[mode])
  },

  async setShuffle(enabled: boolean) {
    getManager().setShuffle(enabled ? ShuffleMode.Random : ShuffleMode.Off)
  },
}

// For compatibility with desktop patterns
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export async function invoke(command: string, args?: any): Promise<any> {
  switch (command) {
    case 'play_track':
      return playerCommands.playTrack(args.trackId)
    case 'pause_playback':
      return playerCommands.pausePlayback()
    case 'resume_playback':
      return playerCommands.resumePlayback()
    case 'skip_next':
      return playerCommands.skipNext()
    case 'skip_previous':
      return playerCommands.skipPrevious()
    case 'seek_to':
      return playerCommands.seekTo(args.position)
    case 'set_volume':
      return playerCommands.setVolume(args.volume)
    case 'set_repeat_mode':
      return playerCommands.setRepeatMode(args.mode)
    case 'set_shuffle':
      return playerCommands.setShuffle(args.enabled)
    case 'get_playback_capabilities':
      return {
        hasNext: getManager().hasNext(),
        hasPrevious: getManager().hasPrevious(),
      }
    default:
      console.warn(`[Demo] Unhandled command: ${command}`)
      return null
  }
}
