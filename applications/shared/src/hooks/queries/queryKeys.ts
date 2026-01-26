/**
 * Query Key Factories - 2026 Best Practices
 * Organized by domain for type-safe cache invalidation
 *
 * @see https://tanstack.com/query/v5/docs/react/guides/query-keys
 * @see https://tkdodo.eu/blog/effective-react-query-keys
 */

/**
 * Albums query keys
 */
export const albumKeys = {
  all: () => ['albums'] as const,
  lists: () => [...albumKeys.all(), 'list'] as const,
  list: (filters?: Record<string, unknown>) => [...albumKeys.lists(), filters] as const,
  details: () => [...albumKeys.all(), 'detail'] as const,
  detail: (id: number) => [...albumKeys.details(), id] as const,
  tracks: (id: number) => [...albumKeys.detail(id), 'tracks'] as const,
  random: (limit: number) => [...albumKeys.lists(), 'random', limit] as const,
  recentlyAdded: (limit: number) => [...albumKeys.lists(), 'recently-added', limit] as const,
  recentlyAddedWithinDays: (days: number, limit: number) =>
    [...albumKeys.lists(), 'recently-added-days', { days, limit }] as const,
  leastPlayed: (limit: number) => [...albumKeys.lists(), 'least-played', limit] as const,
  timeCapsule: (limit: number) => [...albumKeys.lists(), 'time-capsule', limit] as const,
}

/**
 * Artists query keys
 */
export const artistKeys = {
  all: () => ['artists'] as const,
  lists: () => [...artistKeys.all(), 'list'] as const,
  list: (filters?: Record<string, unknown>) => [...artistKeys.lists(), filters] as const,
  details: () => [...artistKeys.all(), 'detail'] as const,
  detail: (id: number) => [...artistKeys.details(), id] as const,
  tracks: (id: number) => [...artistKeys.detail(id), 'tracks'] as const,
  albums: (id: number) => [...artistKeys.detail(id), 'albums'] as const,
  topTracks: (id: number, limit: number) =>
    [...artistKeys.detail(id), 'top-tracks', limit] as const,
  artwork: (id: number) => [...artistKeys.detail(id), 'artwork'] as const,
}

/**
 * Tracks query keys
 */
export const trackKeys = {
  all: () => ['tracks'] as const,
  lists: () => [...trackKeys.all(), 'list'] as const,
  list: (filters?: Record<string, unknown>) => [...trackKeys.lists(), filters] as const,
}

/**
 * Playlists query keys
 */
export const playlistKeys = {
  all: () => ['playlists'] as const,
  lists: () => [...playlistKeys.all(), 'list'] as const,
  list: (filters?: Record<string, unknown>) => [...playlistKeys.lists(), filters] as const,
  details: () => [...playlistKeys.all(), 'detail'] as const,
  detail: (id: string) => [...playlistKeys.details(), id] as const,
  tracks: (id: string) => [...playlistKeys.detail(id), 'tracks'] as const,
  artwork: (id: string) => [...playlistKeys.detail(id), 'artwork'] as const,
  containingTrack: (trackId: number) =>
    [...playlistKeys.all(), 'containing-track', trackId] as const,
}

/**
 * Genres query keys
 */
export const genreKeys = {
  all: () => ['genres'] as const,
  lists: () => [...genreKeys.all(), 'list'] as const,
  list: (filters?: Record<string, unknown>) => [...genreKeys.lists(), filters] as const,
  details: () => [...genreKeys.all(), 'detail'] as const,
  detail: (id: number) => [...genreKeys.details(), id] as const,
  tracks: (id: number) => [...genreKeys.detail(id), 'tracks'] as const,
  albums: (id: number, limit: number) => [...genreKeys.detail(id), 'albums', limit] as const,
}

/**
 * Library/system query keys
 */
export const libraryKeys = {
  health: () => ['library', 'health'] as const,
}

/**
 * Playback context query keys
 */
export const contextKeys = {
  all: () => ['contexts'] as const,
  recent: (limit: number) => [...contextKeys.all(), 'recent', limit] as const,
}

/**
 * Settings query keys
 */
export const settingsKeys = {
  all: () => ['settings'] as const,
  detail: (key: string) => [...settingsKeys.all(), key] as const,
}
