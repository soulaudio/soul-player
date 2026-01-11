/**
 * Tauri Backend Provider - implements BackendInterface using Tauri invoke
 * Used for desktop app
 */

import { ReactNode, useMemo } from 'react'
import { invoke } from '@tauri-apps/api/core'
import {
  BackendProvider,
  type BackendInterface,
  type BackendTrack,
  type BackendAlbum,
  type BackendArtist,
  type BackendPlaylist,
  type BackendGenre,
  type DatabaseHealth,
  type BackendPlaybackContext,
} from '@soul-player/shared'

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

    async getPlaylistTracks(playlistId: string) {
      return invoke<BackendTrack[]>('get_playlist_tracks', { playlistId })
    },

    async getGenreTracks(genreId: number) {
      return invoke<BackendTrack[]>('get_genre_tracks', { genreId })
    },

    // Health check
    async checkDatabaseHealth() {
      return invoke<DatabaseHealth>('check_database_health')
    },

    // Playback context
    async getRecentContexts(limit: number) {
      return invoke<BackendPlaybackContext[]>('get_recent_playback_contexts', { limit })
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

    // Track operations
    async deleteTrack(id: number) {
      await invoke('delete_track', { id })
    },

    // Queue/playback
    async playQueue(queue, startIndex) {
      console.log('[TauriBackendProvider] playQueue called:', { queueLength: queue.length, startIndex, firstTrack: queue[0] })
      try {
        await invoke('play_queue', { queue, startIndex })
        console.log('[TauriBackendProvider] playQueue invoke completed')
      } catch (err) {
        console.error('[TauriBackendProvider] playQueue invoke failed:', err)
        throw err
      }
    },

    // Onboarding
    async checkOnboardingNeeded() {
      return invoke<boolean>('check_onboarding_needed')
    },
  }), [])

  return (
    <BackendProvider value={backend}>
      {children}
    </BackendProvider>
  )
}
