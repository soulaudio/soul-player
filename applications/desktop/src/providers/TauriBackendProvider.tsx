/**
 * Tauri Backend Provider - implements BackendInterface using Tauri invoke
 * Used for desktop app
 */

import { ReactNode, useMemo } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { getVersion } from '@tauri-apps/api/app'
import {
  BackendProvider,
  type BackendInterface,
  type BackendTrack,
  type BackendAlbum,
  type BackendArtist,
  type BackendPlaylist,
  type BackendGenre,
  type DatabaseHealth,
  type PlaybackContext,
  type SetArtworkParams,
  type EffectSlot,
  type EffectType,
  type LatencyInfo,
  type ExclusiveConfig,
  type AnalysisQueueStats,
  type AnalysisWorkerStatus,
} from '@soul-player/shared'
import {
  invokeValidatedArray,
  BackendTrackSchema,
} from '../types/validation'

interface TauriBackendProviderProps {
  children: ReactNode
}

export function TauriBackendProvider({ children }: TauriBackendProviderProps) {
  const backend = useMemo<BackendInterface>(() => ({
    // Library data
    async getAllTracks() {
      return invoke<BackendTrack[]>('get_all_tracks')
    },

    async getAllAlbums() {
      return invoke<BackendAlbum[]>('get_all_albums')
    },

    async getAllArtists() {
      return invoke<BackendArtist[]>('get_all_artists')
    },

    async getAllPlaylists() {
      return invoke<BackendPlaylist[]>('get_all_playlists')
    },

    async getAllGenres() {
      return invoke<BackendGenre[]>('get_all_genres')
    },

    async getRandomAlbums(limit: number) {
      return invoke<BackendAlbum[]>('get_random_albums', { limit })
    },

    async getRecentlyAddedAlbums(limit: number) {
      return invoke<BackendAlbum[]>('get_recently_added_albums', { limit })
    },

    async getRecentlyAddedAlbumsWithinDays(days: number, limit: number) {
      return invoke<BackendAlbum[]>('get_recently_added_albums_within_days', { days, limit })
    },

    async getLeastPlayedAlbums(limit: number) {
      return invoke<BackendAlbum[]>('get_least_played_albums', { limit })
    },

    async getTimeCapsuleAlbums(limit: number) {
      return invoke<BackendAlbum[]>('get_time_capsule_albums', { limit })
    },

    async getGenreAlbums(genreId: number, limit: number) {
      return invoke<BackendAlbum[]>('get_genre_albums', { genreId, limit })
    },

    // Single item lookups
    async getAlbumById(id: number) {
      return invoke<BackendAlbum | null>('get_album_by_id', { id })
    },

    async getArtistById(id: number) {
      return invoke<BackendArtist | null>('get_artist_by_id', { id })
    },

    async getPlaylistById(id: string) {
      return invoke<BackendPlaylist | null>('get_playlist_by_id', { id })
    },

    async getGenreById(id: number) {
      return invoke<BackendGenre | null>('get_genre_by_id', { id })
    },

    // Related data
    async getAlbumTracks(albumId: number) {
      return invoke<BackendTrack[]>('get_album_tracks', { albumId })
    },

    async getArtistTracks(artistId: number) {
      return invoke<BackendTrack[]>('get_artist_tracks', { artistId })
    },

    async getArtistAlbums(artistId: number) {
      return invoke<BackendAlbum[]>('get_artist_albums', { artistId })
    },

    async getArtistTopTracks(artistId: number, limit = 10) {
      return invoke<BackendTrack[]>('get_artist_top_tracks', { artistId, limit })
    },

    async getPlaylistTracks(playlistId: string) {
      return invoke<BackendTrack[]>('get_playlist_tracks', { id: playlistId })
    },

    async getGenreTracks(genreId: number) {
      return invoke<BackendTrack[]>('get_genre_tracks', { genreId })
    },

    async getTracksByIds(trackIds: number[]) {
      return invokeValidatedArray('get_tracks_by_ids', BackendTrackSchema.nullable(), { trackIds })
    },

    // Health check
    async checkDatabaseHealth() {
      return invoke<DatabaseHealth>('check_database_health')
    },

    // Playback context
    async getRecentContexts(limit: number) {
      return invoke<PlaybackContext[]>('get_recent_playback_contexts', { limit })
    },

    async recordContext(context) {
      await invoke('record_playback_context', { input: context })
    },

    // Playlist operations
    async createPlaylist(name: string, description?: string) {
      return invoke<BackendPlaylist>('create_playlist', { name, description })
    },

    async deletePlaylist(id: string) {
      await invoke('delete_playlist', { id })
    },

    async getPlaylistsContainingTrack(trackId: number) {
      return invoke<string[]>('get_playlists_containing_track', { trackId: String(trackId) })
    },

    async addTrackToPlaylist(playlistId: string, trackId: number) {
      await invoke('add_track_to_playlist', { playlistId, trackId: String(trackId) })
    },

    async removeTrackFromPlaylist(playlistId: string, trackId: number) {
      await invoke('remove_track_from_playlist', { playlistId, trackId: String(trackId) })
    },

    // Track operations
    async deleteTrack(id: number) {
      await invoke('delete_track', { id })
    },

    async showInFileExplorer(path: string) {
      await invoke('show_in_file_explorer', { path })
    },

    // Onboarding
    async checkOnboardingNeeded() {
      return invoke<boolean>('check_onboarding_needed')
    },

    async getUserSetting(key: string) {
      // Settings can be any JSON value, but we use string | null as the return type
      // since settings are stored as JSON strings
      return invoke<string | null>('get_user_setting', { key })
    },

    async setUserSetting(key: string, value: unknown) {
      // Value can be any JSON-serializable value
      await invoke('set_user_setting', { key, value })
    },

    // Artwork editing
    async setArtwork(params: SetArtworkParams) {
      await invoke('set_artwork', { request: params })
    },

    async removeArtwork(entityType: 'album' | 'artist' | 'playlist', entityId: string) {
      await invoke('remove_artwork', { entityType, entityId })
    },

    async getArtistArtwork(artistId: number) {
      return invoke<string | null>('get_artist_artwork', { artistId })
    },

    async getPlaylistArtwork(playlistId: string) {
      return invoke<string | null>('get_playlist_artwork', { playlistId })
    },

    // App metadata
    async getVersion() {
      return getVersion()
    },

    // Audio Settings - DSP Chain
    async getDspChain() {
      return invoke<EffectSlot[]>('get_dsp_chain')
    },

    async addEffectToChain(slotIndex: number, effect: EffectType) {
      await invoke('add_effect_to_chain', { slotIndex, effect })
    },

    async removeEffectFromChain(slotIndex: number) {
      await invoke('remove_effect_from_chain', { slotIndex })
    },

    async toggleEffect(slotIndex: number, enabled: boolean) {
      await invoke('toggle_effect', { slotIndex, enabled })
    },

    async clearDspChain() {
      await invoke('clear_dsp_chain')
    },

    async updateEffectParameters(slotIndex: number, effect: EffectType) {
      await invoke('update_effect_parameters', { slotIndex, effect })
    },

    // Audio Settings - Latency & Exclusive Mode
    async getLatencyInfo() {
      return invoke<LatencyInfo>('get_latency_info')
    },

    async isExclusiveMode() {
      return invoke<boolean>('is_exclusive_mode')
    },

    async disableExclusiveMode() {
      await invoke('disable_exclusive_mode')
    },

    async setExclusiveMode(config: ExclusiveConfig) {
      return invoke<LatencyInfo>('set_exclusive_mode', { config })
    },

    // Audio Settings - Volume Leveling Analysis
    async getAnalysisQueueStats() {
      return invoke<AnalysisQueueStats>('get_analysis_queue_stats')
    },

    async getAnalysisWorkerStatus() {
      return invoke<AnalysisWorkerStatus>('get_analysis_worker_status')
    },

    async startAnalysisWorker() {
      await invoke('start_analysis_worker')
    },

    async stopAnalysisWorker() {
      await invoke('stop_analysis_worker')
    },

    async queueAllUnanalyzed() {
      return invoke<number>('queue_all_unanalyzed')
    },

    async clearCompletedAnalysis() {
      await invoke('clear_completed_analysis')
    },

    // Audio Settings - Volume Leveling Runtime
    async setVolumeLevelingMode(mode: string) {
      await invoke('set_volume_leveling_mode', { mode })
    },

    async setVolumeLevelingPreamp(preampDb: number) {
      await invoke('set_volume_leveling_preamp', { preampDb })
    },

    async setVolumeLevelingPreventClipping(prevent: boolean) {
      await invoke('set_volume_leveling_prevent_clipping', { prevent })
    },

    // Audio Settings - Resampling
    async setResamplingQuality(quality: string) {
      await invoke('set_resampling_quality', { quality })
    },

    async setResamplingTargetRate(rate: number) {
      await invoke('set_resampling_target_rate', { rate })
    },

    async setResamplingBackend(backend: string) {
      await invoke('set_resampling_backend', { backend })
    },

    async isR8brainAvailable() {
      return invoke<boolean>('is_r8brain_available')
    },

    // Audio Settings - Crossfade
    async setCrossfadeSettings(enabled: boolean, durationMs: number, curve: string) {
      await invoke('set_crossfade_settings', { enabled, durationMs, curve })
    },

    // Audio Settings - Device Selection
    async getAudioBackends() {
      return invoke('get_audio_backends')
    },

    async getAudioDevices(backendStr: string) {
      return invoke('get_audio_devices', { backendStr })
    },

    async setAudioDevice(backendStr: string, deviceName: string) {
      await invoke('set_audio_device', { backendStr, deviceName })
    },

    async getCurrentAudioDevice() {
      try {
        return await invoke('get_current_audio_device')
      } catch {
        return null
      }
    },

    // Audio Settings - File Dialog (for convolution IR selection)
    async openFileDialog(multiple: boolean, filters: Array<{ name: string; extensions: string[] }>) {
      return invoke<string[] | null>('open_file_dialog', { multiple, filters })
    },
  }), [])

  return (
    <BackendProvider value={backend}>
      {children}
    </BackendProvider>
  )
}
