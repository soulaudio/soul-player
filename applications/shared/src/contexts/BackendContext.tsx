/**
 * BackendContext - abstracts all backend operations for platform-agnostic pages
 * Desktop: Uses Tauri invoke()
 * Marketing: Uses demo data and mock implementations
 */

import { createContext, useContext, ReactNode } from 'react'

// =============================================================================
// Types - shared data structures
// =============================================================================

export interface BackendTrack {
  id: number
  title: string
  artist_name?: string
  album_title?: string
  album_id?: number
  artist_id?: number
  duration_seconds?: number
  file_path?: string
  track_number?: number
  year?: number
  file_format?: string
  bit_rate?: number
  sample_rate?: number
  channels?: number
  source_type?: string
  source_name?: string
  source_online?: boolean
  cover_art_path?: string
  // Whether the track is in the managed library (vs watched folder)
  is_in_managed_library?: boolean
}

export interface BackendAlbum {
  id: number
  title: string
  artist_id?: number
  artist_name?: string
  year?: number
  track_count?: number
  cover_art_path?: string
}

export interface BackendArtist {
  id: number
  name: string
  sort_name?: string
  track_count: number
  album_count: number
  cover_art_path?: string
}

export interface BackendPlaylist {
  id: string
  name: string
  description?: string
  owner_id: number
  is_public: boolean
  is_favorite: boolean
  track_count: number
  created_at: string
  updated_at: string
  cover_art_path?: string
}

export interface BackendGenre {
  id: number
  name: string
  track_count: number
}

export interface DatabaseHealth {
  total_tracks: number
  tracks_with_availability: number
  tracks_with_local_files: number
  issues: string[]
}

export interface PlaybackContext {
  id?: number
  contextType: 'album' | 'artist' | 'playlist' | 'genre' | 'tracks'
  contextId: string | null
  contextName: string | null
  contextArtworkPath: string | null
  playedAt?: string
}

export interface QueueTrack {
  trackId: string
  title: string
  artist: string
  album: string | null
  filePath: string
  durationSeconds: number | null
  trackNumber: number | null
}

export interface SetArtworkParams {
  entityType: 'album' | 'artist' | 'playlist'
  entityId: string
  artworkBase64: string
  mimeType: string
  writeToFiles?: boolean // Only for albums - embed in track files
  useSoulStorage?: boolean // Only for albums - use Soul Player storage instead of album folder
}

// =============================================================================
// Audio Settings Types
// =============================================================================

export interface EffectSlot {
  index: number
  effect: EffectType | null
  enabled: boolean
}

export type EffectType =
  | { type: 'eq'; bands: EqBand[] }
  | { type: 'compressor'; settings: CompressorSettings }
  | { type: 'limiter'; settings: LimiterSettings }
  | { type: 'crossfeed'; settings: CrossfeedSettings }
  | { type: 'stereo'; settings: StereoSettings }
  | { type: 'graphic_eq'; settings: GraphicEqSettings }
  | { type: 'convolution'; settings: ConvolutionSettings }

export interface EqBand {
  frequency: number
  gain: number
  q: number
}

export interface CompressorSettings {
  thresholdDb: number
  ratio: number
  attackMs: number
  releaseMs: number
  kneeDb: number
  makeupGainDb: number
}

export interface LimiterSettings {
  thresholdDb: number
  releaseMs: number
}

export interface CrossfeedSettings {
  preset: string
  levelDb: number
  cutoffHz: number
}

export interface StereoSettings {
  width: number
  midGainDb: number
  sideGainDb: number
  balance: number
}

export interface GraphicEqSettings {
  preset: string
  bandCount: number
  gains: number[]
}

export interface ConvolutionSettings {
  irFilePath: string
  wetDryMix: number
  preDelayMs: number
  decay: number
}

export interface HeadroomSettings {
  enabled: boolean
  mode: {
    mode: string // 'auto' | 'manual' | 'disabled'
    manualDb: number | null
  }
  totalGainDb: number
  attenuationDb: number
}

export interface LatencyInfo {
  bufferSamples: number
  bufferMs: number
  totalMs: number
  exclusive: boolean
}

export interface ExclusiveConfig {
  sampleRate: number
  bitDepth: string
  bufferFrames: number | null
  exclusiveMode: boolean
  deviceName: string | null
  backend: string
}

export interface AnalysisQueueStats {
  total: number
  pending: number
  processing: number
  completed: number
  failed: number
}

export interface AnalysisWorkerStatus {
  isRunning: boolean
  tracksAnalyzed: number
}

export interface AudioBackend {
  backend: 'default' | 'asio' | 'jack'
  name: string
  description: string
  available: boolean
  isDefault: boolean
  deviceCount: number
}

export interface AudioDevice {
  name: string
  backend: string
  isDefault: boolean
  sampleRate: number
  channels: number
  sampleRateRange?: [number, number]
}

// =============================================================================
// Backend Interface
// =============================================================================

export interface BackendInterface {
  // Library data
  getAllTracks: () => Promise<BackendTrack[]>
  getAllAlbums: () => Promise<BackendAlbum[]>
  getAllArtists: () => Promise<BackendArtist[]>
  getAllPlaylists: () => Promise<BackendPlaylist[]>
  getAllGenres: () => Promise<BackendGenre[]>
  getRandomAlbums: (limit: number) => Promise<BackendAlbum[]>

  // Discovery & recommendations
  getRecentlyAddedAlbums: (limit: number) => Promise<BackendAlbum[]>
  getRecentlyAddedAlbumsWithinDays: (days: number, limit: number) => Promise<BackendAlbum[]>
  getLeastPlayedAlbums: (limit: number) => Promise<BackendAlbum[]>
  getTimeCapsuleAlbums: (limit: number) => Promise<BackendAlbum[]>
  getGenreAlbums: (genreId: number, limit: number) => Promise<BackendAlbum[]>

  // Single item lookups
  getAlbumById: (id: number) => Promise<BackendAlbum | null>
  getArtistById: (id: number) => Promise<BackendArtist | null>
  getPlaylistById: (id: string) => Promise<BackendPlaylist | null>
  getGenreById: (id: number) => Promise<BackendGenre | null>

  // Related data
  getAlbumTracks: (albumId: number) => Promise<BackendTrack[]>
  getArtistTracks: (artistId: number) => Promise<BackendTrack[]>
  getArtistAlbums: (artistId: number) => Promise<BackendAlbum[]>
  getArtistTopTracks: (artistId: number, limit?: number) => Promise<BackendTrack[]>
  getPlaylistTracks: (playlistId: string) => Promise<BackendTrack[]>
  getGenreTracks: (genreId: number) => Promise<BackendTrack[]>

  /**
   * Get multiple tracks by their IDs
   * Returns null for missing tracks
   */
  getTracksByIds: (trackIds: number[]) => Promise<(BackendTrack | null)[]>

  // Health check
  checkDatabaseHealth: () => Promise<DatabaseHealth>

  // Playback context (for "Jump back into" feature)
  getRecentContexts: (limit: number) => Promise<PlaybackContext[]>
  recordContext: (context: Omit<PlaybackContext, 'id' | 'playedAt'>) => Promise<void>

  // Playlist operations
  createPlaylist: (name: string, description?: string) => Promise<BackendPlaylist>
  deletePlaylist: (id: string) => Promise<void>
  getPlaylistsContainingTrack: (trackId: number) => Promise<string[]>
  addTrackToPlaylist: (playlistId: string, trackId: number) => Promise<void>
  removeTrackFromPlaylist: (playlistId: string, trackId: number) => Promise<void>

  // Track operations
  deleteTrack: (id: number) => Promise<void>
  showInFileExplorer: (path: string) => Promise<void>

  // Onboarding (desktop only, can be no-op for web)
  checkOnboardingNeeded: () => Promise<boolean>

  // Settings
  getUserSetting: (key: string) => Promise<any>
  setUserSetting: (key: string, value: any) => Promise<void>

  // Artwork editing
  setArtwork: (params: SetArtworkParams) => Promise<void>
  removeArtwork: (entityType: 'album' | 'artist' | 'playlist', entityId: string) => Promise<void>
  getArtistArtwork: (artistId: number) => Promise<string | null>
  getPlaylistArtwork: (playlistId: string) => Promise<string | null>

  // App metadata
  getVersion: () => Promise<string>

  // Audio Settings - DSP Chain
  getDspChain: () => Promise<EffectSlot[]>
  addEffectToChain: (slotIndex: number, effect: EffectType) => Promise<void>
  removeEffectFromChain: (slotIndex: number) => Promise<void>
  toggleEffect: (slotIndex: number, enabled: boolean) => Promise<void>
  clearDspChain: () => Promise<void>
  updateEffectParameters: (slotIndex: number, effect: EffectType) => Promise<void>

  // Audio Settings - Headroom Management
  getHeadroomSettings: () => Promise<HeadroomSettings>
  setHeadroomMode: (mode: string, manualDb?: number) => Promise<void>
  setHeadroomEnabled: (enabled: boolean) => Promise<void>

  // Audio Settings - Latency & Exclusive Mode
  getLatencyInfo: () => Promise<LatencyInfo>
  isExclusiveMode: () => Promise<boolean>
  disableExclusiveMode: () => Promise<void>
  setExclusiveMode: (config: ExclusiveConfig) => Promise<LatencyInfo>

  // Audio Settings - Volume Leveling Analysis
  getAnalysisQueueStats: () => Promise<AnalysisQueueStats>
  getAnalysisWorkerStatus: () => Promise<AnalysisWorkerStatus>
  startAnalysisWorker: () => Promise<void>
  stopAnalysisWorker: () => Promise<void>
  queueAllUnanalyzed: () => Promise<number>
  clearCompletedAnalysis: () => Promise<void>

  // Audio Settings - Volume Leveling Runtime
  setVolumeLevelingMode: (mode: string) => Promise<void>
  setVolumeLevelingPreamp: (preampDb: number) => Promise<void>
  setVolumeLevelingPreventClipping: (prevent: boolean) => Promise<void>

  // Audio Settings - Resampling
  setResamplingQuality: (quality: string) => Promise<void>
  setResamplingTargetRate: (rate: number) => Promise<void>
  setResamplingBackend: (backend: string) => Promise<void>
  isR8brainAvailable: () => Promise<boolean>

  // Audio Settings - Crossfade
  setCrossfadeSettings: (enabled: boolean, durationMs: number, curve: string) => Promise<void>

  // Audio Settings - Device Selection
  getAudioBackends: () => Promise<AudioBackend[]>
  getAudioDevices: (backendStr: string) => Promise<AudioDevice[]>
  setAudioDevice: (backendStr: string, deviceName: string) => Promise<void>
  getCurrentAudioDevice: () => Promise<AudioDevice | null>

  // Audio Settings - File Dialog (for convolution IR selection)
  openFileDialog: (multiple: boolean, filters: Array<{ name: string; extensions: string[] }>) => Promise<string[] | null>
}

// =============================================================================
// Context
// =============================================================================

export const BackendContext = createContext<BackendInterface | null>(null)

export function useBackend(): BackendInterface {
  const context = useContext(BackendContext)
  if (!context) {
    throw new Error('useBackend must be used within BackendProvider')
  }
  return context
}

interface BackendProviderProps {
  children: ReactNode
  value: BackendInterface
}

export function BackendProvider({ children, value }: BackendProviderProps) {
  return (
    <BackendContext.Provider value={value}>
      {children}
    </BackendContext.Provider>
  )
}
