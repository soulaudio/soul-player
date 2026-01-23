/**
 * Utility functions for HomePage album filtering and categorization
 */

import { BackendAlbum } from '../contexts/BackendContext'

/**
 * Fisher-Yates shuffle algorithm for uniform distribution
 * O(n) time complexity, better than Array.sort() based shuffles
 * @param array - Array to shuffle (not mutated)
 * @returns Shuffled copy of the array
 */
export function shuffle<T>(array: T[]): T[] {
  const result = [...array]
  for (let i = result.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [result[i], result[j]] = [result[j], result[i]]
  }
  return result
}

export interface PlaybackContext {
  contextType: string
  contextId: string | null
  playedAt?: string
}

export interface AlbumCategories {
  recentAlbumIds: Set<number>
  timeCapsuleAlbumIds: Set<number>
  onRepeatAlbumIds: number[] // Sorted by play count (descending)
}

/**
 * Categorizes albums based on playback history
 * @param contexts - All playback contexts with timestamps
 * @param now - Current timestamp (for testing)
 * @returns Album ID sets for different categories
 */
export function categorizeAlbumsByPlayback(
  contexts: PlaybackContext[],
  now: number = Date.now()
): AlbumCategories {
  const twoMonthsAgo = now - (60 * 24 * 60 * 60 * 1000) // 60 days
  const sixMonthsAgo = now - (180 * 24 * 60 * 60 * 1000) // 180 days
  const oneMonthAgo = now - (30 * 24 * 60 * 60 * 1000) // 30 days
  const twoWeeksAgo = now - (14 * 24 * 60 * 60 * 1000) // 14 days

  const recentAlbumIds = new Set<number>()
  const timeCapsuleIds = new Set<number>()
  const recentlyPlayedIds = new Set<number>()
  const albumPlayCounts = new Map<number, number>() // Track play counts in last 2 weeks

  // Filter for album contexts with timestamps
  const albumContexts = contexts.filter(ctx => ctx.contextType === 'album' && ctx.playedAt)

  // First pass: identify recent albums (last 30 days) and count plays in last 2 weeks
  for (const ctx of albumContexts) {
    if (!ctx.playedAt) continue
    const playedAt = new Date(ctx.playedAt).getTime()
    const albumId = Number(ctx.contextId)

    if (playedAt >= oneMonthAgo) {
      recentlyPlayedIds.add(albumId)
      recentAlbumIds.add(albumId)
    }

    // Count plays in last 2 weeks
    if (playedAt >= twoWeeksAgo) {
      albumPlayCounts.set(albumId, (albumPlayCounts.get(albumId) || 0) + 1)
    }
  }

  // Second pass: identify time capsule albums (2-6 months ago, not recently)
  for (const ctx of albumContexts) {
    if (!ctx.playedAt) continue
    const playedAt = new Date(ctx.playedAt).getTime()
    const albumId = Number(ctx.contextId)

    // Album was played 2-6 months ago
    if (playedAt >= sixMonthsAgo && playedAt <= twoMonthsAgo) {
      // And not played in the last month
      if (!recentlyPlayedIds.has(albumId)) {
        timeCapsuleIds.add(albumId)
      }
    }
  }

  // Identify albums "on repeat" (played 3+ times in last 2 weeks)
  // Sort by play count descending
  const onRepeatAlbums = Array.from(albumPlayCounts.entries())
    .filter(([_, count]) => count >= 3) // Minimum 3 plays to be "on repeat"
    .sort((a, b) => b[1] - a[1]) // Sort by play count descending
    .map(([albumId, _]) => albumId)

  return {
    recentAlbumIds,
    timeCapsuleAlbumIds: timeCapsuleIds,
    onRepeatAlbumIds: onRepeatAlbums
  }
}

/**
 * Selects albums from a set of IDs, avoiding duplicates
 * @param allAlbums - All available albums
 * @param albumIds - IDs to select from
 * @param maxCount - Maximum number of albums to return
 * @param usedIds - Set of already used album IDs (will be mutated)
 * @returns Selected albums
 */
export function selectAlbumsFromIds(
  allAlbums: BackendAlbum[],
  albumIds: Set<number>,
  maxCount: number,
  usedIds?: Set<number>
): BackendAlbum[] {
  if (allAlbums.length === 0 || albumIds.size === 0) return []

  // Filter albums by IDs and exclude already used
  let filteredAlbums = allAlbums.filter(album => albumIds.has(album.id))
  if (usedIds) {
    filteredAlbums = filteredAlbums.filter(album => !usedIds.has(album.id))
  }

  // Shuffle and take up to maxCount
  const shuffled = shuffle(filteredAlbums)
  const selected = shuffled.slice(0, Math.min(maxCount, filteredAlbums.length))

  // Mark as used
  if (usedIds) {
    selected.forEach(album => usedIds.add(album.id))
  }

  return selected
}

/**
 * Selects albums from an ordered array of IDs (maintains order), avoiding duplicates
 * Used for "on repeat" where order matters (sorted by play count)
 * @param allAlbums - All available albums
 * @param albumIds - Ordered array of IDs to select from
 * @param maxCount - Maximum number of albums to return
 * @param usedIds - Set of already used album IDs (will be mutated)
 * @returns Selected albums in the same order
 */
export function selectAlbumsFromOrderedIds(
  allAlbums: BackendAlbum[],
  albumIds: number[],
  maxCount: number,
  usedIds?: Set<number>
): BackendAlbum[] {
  if (allAlbums.length === 0 || albumIds.length === 0) return []

  // Create a Map for O(1) album lookups instead of O(n) find()
  const albumMap = new Map(allAlbums.map(a => [a.id, a]))
  const selected: BackendAlbum[] = []

  for (const albumId of albumIds) {
    if (selected.length >= maxCount) break

    // Skip if already used
    if (usedIds && usedIds.has(albumId)) continue

    const album = albumMap.get(albumId)
    if (album) {
      selected.push(album)
      if (usedIds) {
        usedIds.add(album.id)
      }
    }
  }

  return selected
}
