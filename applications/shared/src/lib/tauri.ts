// Type-safe Tauri command wrappers
import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import type { Track, Album, Artist, Playlist } from '../types';

// Player commands
export const playerCommands = {
  playTrack: (trackId: number) =>
    tauriInvoke<void>('play_track', { trackId }),

  pausePlayback: () =>
    tauriInvoke<void>('pause_playback'),

  resumePlayback: () =>
    tauriInvoke<void>('resume_playback'),

  stopPlayback: () =>
    tauriInvoke<void>('stop_playback'),

  setVolume: (volume: number) =>
    tauriInvoke<void>('set_volume', { volume }),

  seek: (position: number) =>
    tauriInvoke<void>('seek_to', { position }),

  skipNext: () =>
    tauriInvoke<void>('next_track'),

  skipPrevious: () =>
    tauriInvoke<void>('previous_track'),
};

// Library commands
export const libraryCommands = {
  getAllTracks: () =>
    tauriInvoke<Track[]>('get_all_tracks'),

  getAllAlbums: () =>
    tauriInvoke<Album[]>('get_all_albums'),

  getAllArtists: () =>
    tauriInvoke<Artist[]>('get_all_artists'),

  getTrackById: (id: number) =>
    tauriInvoke<Track>('get_track_by_id', { id }),

  scanLibrary: (paths: string[]) =>
    tauriInvoke<void>('scan_library', { paths }),

  searchLibrary: (query: string) =>
    tauriInvoke<Track[]>('search_library', { query }),
};

// Playlist commands
export const playlistCommands = {
  getAllPlaylists: () =>
    tauriInvoke<Playlist[]>('get_all_playlists'),

  getPlaylistById: (id: number) =>
    tauriInvoke<Playlist>('get_playlist_by_id', { id }),

  createPlaylist: (name: string, description?: string) =>
    tauriInvoke<Playlist>('create_playlist', { name, description }),

  updatePlaylist: (id: number, name: string, description?: string) =>
    tauriInvoke<Playlist>('update_playlist', { id, name, description }),

  deletePlaylist: (id: number) =>
    tauriInvoke<void>('delete_playlist', { id }),

  addTracksToPlaylist: (playlistId: number, trackIds: number[]) =>
    tauriInvoke<void>('add_tracks_to_playlist', { playlistId, trackIds }),

  removeTrackFromPlaylist: (playlistId: number, trackId: number) =>
    tauriInvoke<void>('remove_track_from_playlist', { playlistId, trackId }),

  getPlaylistTracks: (playlistId: number) =>
    tauriInvoke<Track[]>('get_playlist_tracks', { playlistId }),
};

// Combined export
export const commands = {
  ...playerCommands,
  ...libraryCommands,
  ...playlistCommands,
};
