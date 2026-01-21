/**
 * Server Backend Provider - implements BackendInterface using REST API
 * Reusable for web app, mobile app, and any server-backed client
 */

import { ReactNode, useMemo } from 'react'
import { BackendProvider } from '../contexts/BackendContext'
import type {
import { debug } from '../utils/debug';
  BackendInterface,
  BackendTrack,
  BackendAlbum,
  BackendArtist,
  BackendPlaylist,
  BackendGenre,
  DatabaseHealth,
  PlaybackContext,
  SetArtworkParams,
} from '../contexts/BackendContext'

interface ServerBackendProviderProps {
  apiBase: string
  authToken?: string | null
  children: ReactNode
}

/**
 * Fetch helper with authentication and error handling
 */
async function apiFetch<T>(
  url: string,
  options: RequestInit = {},
  authToken?: string | null
): Promise<T> {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...(options.headers as Record<string, string>),
  }

  if (authToken) {
    headers['Authorization'] = `Bearer ${authToken}`
  }

  const response = await fetch(url, {
    ...options,
    headers,
  })

  if (!response.ok) {
    const error = await response.text().catch(() => response.statusText)
    throw new Error(`API error (${response.status}): ${error}`)
  }

  // Handle no-content responses
  if (response.status === 204 || response.headers.get('Content-Length') === '0') {
    return undefined as T
  }

  return response.json()
}

export function ServerBackendProvider({ apiBase, authToken, children }: ServerBackendProviderProps) {
  const backend = useMemo<BackendInterface>(() => ({
    // Library data
    async getAllTracks() {
      return apiFetch<BackendTrack[]>(`${apiBase}/tracks`, {}, authToken)
    },

    async getAllAlbums() {
      return apiFetch<BackendAlbum[]>(`${apiBase}/albums`, {}, authToken)
    },

    async getAllArtists() {
      return apiFetch<BackendArtist[]>(`${apiBase}/artists`, {}, authToken)
    },

    async getAllPlaylists() {
      return apiFetch<BackendPlaylist[]>(`${apiBase}/playlists`, {}, authToken)
    },

    async getAllGenres() {
      return apiFetch<BackendGenre[]>(`${apiBase}/genres`, {}, authToken)
    },

    async getRandomAlbums(limit: number) {
      return apiFetch<BackendAlbum[]>(`${apiBase}/albums/random?limit=${limit}`, {}, authToken)
    },

    async getRecentlyAddedAlbums(limit: number) {
      return apiFetch<BackendAlbum[]>(`${apiBase}/albums/recently-added?limit=${limit}`, {}, authToken)
    },

    async getRecentlyAddedAlbumsWithinDays(days: number, limit: number) {
      return apiFetch<BackendAlbum[]>(`${apiBase}/albums/recently-added?days=${days}&limit=${limit}`, {}, authToken)
    },

    async getLeastPlayedAlbums(limit: number) {
      return apiFetch<BackendAlbum[]>(`${apiBase}/albums/least-played?limit=${limit}`, {}, authToken)
    },

    async getTimeCapsuleAlbums(limit: number) {
      return apiFetch<BackendAlbum[]>(`${apiBase}/albums/time-capsule?limit=${limit}`, {}, authToken)
    },

    async getGenreAlbums(genreId: number, limit: number) {
      return apiFetch<BackendAlbum[]>(`${apiBase}/genres/${genreId}/albums?limit=${limit}`, {}, authToken)
    },

    // Single item lookups
    async getAlbumById(id: number) {
      return apiFetch<BackendAlbum | null>(`${apiBase}/albums/${id}`, {}, authToken)
    },

    async getArtistById(id: number) {
      return apiFetch<BackendArtist | null>(`${apiBase}/artists/${id}`, {}, authToken)
    },

    async getPlaylistById(id: string) {
      return apiFetch<BackendPlaylist | null>(`${apiBase}/playlists/${id}`, {}, authToken)
    },

    async getGenreById(id: number) {
      return apiFetch<BackendGenre | null>(`${apiBase}/genres/${id}`, {}, authToken)
    },

    // Related data
    async getAlbumTracks(albumId: number) {
      return apiFetch<BackendTrack[]>(`${apiBase}/albums/${albumId}/tracks`, {}, authToken)
    },

    async getArtistTracks(artistId: number) {
      return apiFetch<BackendTrack[]>(`${apiBase}/artists/${artistId}/tracks`, {}, authToken)
    },

    async getArtistAlbums(artistId: number) {
      return apiFetch<BackendAlbum[]>(`${apiBase}/artists/${artistId}/albums`, {}, authToken)
    },

    async getArtistTopTracks(artistId: number, limit = 10) {
      return apiFetch<BackendTrack[]>(`${apiBase}/artists/${artistId}/tracks/top?limit=${limit}`, {}, authToken)
    },

    async getPlaylistTracks(playlistId: string) {
      return apiFetch<BackendTrack[]>(`${apiBase}/playlists/${playlistId}/tracks`, {}, authToken)
    },

    async getGenreTracks(genreId: number) {
      return apiFetch<BackendTrack[]>(`${apiBase}/genres/${genreId}/tracks`, {}, authToken)
    },

    // Health check
    async checkDatabaseHealth() {
      return apiFetch<DatabaseHealth>(`${apiBase}/health/database`, {}, authToken)
    },

    // Playback context
    async getRecentContexts(limit: number) {
      return apiFetch<PlaybackContext[]>(
        `${apiBase}/playback/contexts?limit=${limit}`,
        {},
        authToken
      )
    },

    async recordContext(context) {
      await apiFetch(
        `${apiBase}/playback/contexts`,
        {
          method: 'POST',
          body: JSON.stringify(context),
        },
        authToken
      )
    },

    // Playlist operations
    async createPlaylist(name: string, description?: string) {
      return apiFetch<BackendPlaylist>(
        `${apiBase}/playlists`,
        {
          method: 'POST',
          body: JSON.stringify({ name, description }),
        },
        authToken
      )
    },

    async deletePlaylist(id: string) {
      await apiFetch(
        `${apiBase}/playlists/${id}`,
        { method: 'DELETE' },
        authToken
      )
    },

    async getPlaylistsContainingTrack(trackId: number) {
      return apiFetch<string[]>(
        `${apiBase}/tracks/${trackId}/playlists`,
        {},
        authToken
      )
    },

    async addTrackToPlaylist(playlistId: string, trackId: number) {
      await apiFetch(
        `${apiBase}/playlists/${playlistId}/tracks`,
        {
          method: 'POST',
          body: JSON.stringify({ trackId }),
        },
        authToken
      )
    },

    async removeTrackFromPlaylist(playlistId: string, trackId: number) {
      await apiFetch(
        `${apiBase}/playlists/${playlistId}/tracks/${trackId}`,
        { method: 'DELETE' },
        authToken
      )
    },

    // Track operations
    async deleteTrack(id: number) {
      await apiFetch(
        `${apiBase}/tracks/${id}`,
        { method: 'DELETE' },
        authToken
      )
    },

    async showInFileExplorer(_path: string) {
      // Server-backed web apps can't show file explorer
      debug.log('[ServerBackend] showInFileExplorer not supported in web')
    },

    // Onboarding
    async checkOnboardingNeeded() {
      return apiFetch<boolean>(`${apiBase}/onboarding/needed`, {}, authToken)
    },

    async getUserSetting(key: string) {
      return apiFetch<any>(`${apiBase}/settings/${encodeURIComponent(key)}`, {}, authToken)
    },

    async setUserSetting(key: string, value: any) {
      await apiFetch(`${apiBase}/settings/${encodeURIComponent(key)}`, {
        method: 'PUT',
        body: JSON.stringify({ value }),
        headers: { 'Content-Type': 'application/json' },
      }, authToken)
    },

    // Artwork editing
    async setArtwork(params: SetArtworkParams) {
      await apiFetch(
        `${apiBase}/artwork`,
        {
          method: 'POST',
          body: JSON.stringify(params),
        },
        authToken
      )
    },

    async removeArtwork(entityType: 'album' | 'artist' | 'playlist', entityId: string) {
      await apiFetch(
        `${apiBase}/artwork/${entityType}/${entityId}`,
        { method: 'DELETE' },
        authToken
      )
    },

    async getArtistArtwork(artistId: number) {
      return apiFetch<string | null>(
        `${apiBase}/artists/${artistId}/artwork`,
        {},
        authToken
      )
    },

    async getPlaylistArtwork(playlistId: string) {
      return apiFetch<string | null>(
        `${apiBase}/playlists/${playlistId}/artwork`,
        {},
        authToken
      )
    },

    async getVersion() {
      return apiFetch<string>(`${apiBase}/version`, {}, authToken)
    },
  }), [apiBase, authToken])

  return (
    <BackendProvider value={backend}>
      {children}
    </BackendProvider>
  )
}
