/**
 * Tests for HomePage utility functions
 */

import { describe, it, expect } from 'vitest'
import {
  categorizeAlbumsByPlayback,
  selectAlbumsFromIds,
  selectAlbumsFromOrderedIds,
  PlaybackContext
} from './homePageUtils'
import { BackendAlbum } from '../contexts/BackendContext'

describe('categorizeAlbumsByPlayback', () => {
  const now = new Date('2024-06-01T00:00:00Z').getTime()

  it('should identify recent albums (last 30 days)', () => {
    const contexts: PlaybackContext[] = [
      { contextType: 'album', contextId: '1', playedAt: '2024-05-15T00:00:00Z' }, // 17 days ago
      { contextType: 'album', contextId: '2', playedAt: '2024-05-25T00:00:00Z' }, // 7 days ago
    ]

    const result = categorizeAlbumsByPlayback(contexts, now)

    expect(result.recentAlbumIds.has(1)).toBe(true)
    expect(result.recentAlbumIds.has(2)).toBe(true)
    expect(result.recentAlbumIds.size).toBe(2)
  })

  it('should identify time capsule albums (2-6 months ago, not recently)', () => {
    const contexts: PlaybackContext[] = [
      { contextType: 'album', contextId: '1', playedAt: '2024-03-01T00:00:00Z' }, // ~3 months ago
      { contextType: 'album', contextId: '2', playedAt: '2024-02-01T00:00:00Z' }, // ~4 months ago
      { contextType: 'album', contextId: '3', playedAt: '2024-01-01T00:00:00Z' }, // ~5 months ago
    ]

    const result = categorizeAlbumsByPlayback(contexts, now)

    expect(result.timeCapsuleAlbumIds.has(1)).toBe(true)
    expect(result.timeCapsuleAlbumIds.has(2)).toBe(true)
    expect(result.timeCapsuleAlbumIds.has(3)).toBe(true)
    expect(result.timeCapsuleAlbumIds.size).toBe(3)
  })

  it('should exclude time capsule albums that were played recently', () => {
    const contexts: PlaybackContext[] = [
      { contextType: 'album', contextId: '1', playedAt: '2024-03-01T00:00:00Z' }, // 3 months ago
      { contextType: 'album', contextId: '1', playedAt: '2024-05-25T00:00:00Z' }, // 7 days ago (same album!)
    ]

    const result = categorizeAlbumsByPlayback(contexts, now)

    // Album 1 should be in recent, NOT in time capsule
    expect(result.recentAlbumIds.has(1)).toBe(true)
    expect(result.timeCapsuleAlbumIds.has(1)).toBe(false)
  })

  it('should exclude albums played more than 6 months ago', () => {
    const contexts: PlaybackContext[] = [
      { contextType: 'album', contextId: '1', playedAt: '2023-11-01T00:00:00Z' }, // ~7 months ago
      { contextType: 'album', contextId: '2', playedAt: '2023-06-01T00:00:00Z' }, // ~12 months ago
    ]

    const result = categorizeAlbumsByPlayback(contexts, now)

    expect(result.timeCapsuleAlbumIds.has(1)).toBe(false)
    expect(result.timeCapsuleAlbumIds.has(2)).toBe(false)
    expect(result.timeCapsuleAlbumIds.size).toBe(0)
  })

  it('should exclude albums played less than 2 months ago from time capsule', () => {
    const contexts: PlaybackContext[] = [
      { contextType: 'album', contextId: '1', playedAt: '2024-05-01T00:00:00Z' }, // ~1 month ago
      { contextType: 'album', contextId: '2', playedAt: '2024-04-15T00:00:00Z' }, // ~1.5 months ago
    ]

    const result = categorizeAlbumsByPlayback(contexts, now)

    // These are too recent for time capsule but too old for recent
    expect(result.timeCapsuleAlbumIds.size).toBe(0)
    expect(result.recentAlbumIds.size).toBe(0)
  })

  it('should ignore contexts without playedAt timestamp', () => {
    const contexts: PlaybackContext[] = [
      { contextType: 'album', contextId: '1' }, // No timestamp
      { contextType: 'album', contextId: '2', playedAt: '2024-05-25T00:00:00Z' },
    ]

    const result = categorizeAlbumsByPlayback(contexts, now)

    expect(result.recentAlbumIds.has(1)).toBe(false)
    expect(result.recentAlbumIds.has(2)).toBe(true)
    expect(result.recentAlbumIds.size).toBe(1)
  })

  it('should ignore non-album contexts', () => {
    const contexts: PlaybackContext[] = [
      { contextType: 'artist', contextId: '1', playedAt: '2024-05-25T00:00:00Z' },
      { contextType: 'playlist', contextId: '2', playedAt: '2024-05-25T00:00:00Z' },
      { contextType: 'album', contextId: '3', playedAt: '2024-05-25T00:00:00Z' },
    ]

    const result = categorizeAlbumsByPlayback(contexts, now)

    expect(result.recentAlbumIds.has(1)).toBe(false)
    expect(result.recentAlbumIds.has(2)).toBe(false)
    expect(result.recentAlbumIds.has(3)).toBe(true)
    expect(result.recentAlbumIds.size).toBe(1)
  })

  it('should handle same album played multiple times in time window', () => {
    const contexts: PlaybackContext[] = [
      { contextType: 'album', contextId: '1', playedAt: '2024-03-01T00:00:00Z' }, // 3 months ago
      { contextType: 'album', contextId: '1', playedAt: '2024-03-15T00:00:00Z' }, // ~2.5 months ago
      { contextType: 'album', contextId: '1', playedAt: '2024-04-01T00:00:00Z' }, // ~2 months ago
    ]

    const result = categorizeAlbumsByPlayback(contexts, now)

    // Should appear in time capsule only once
    expect(result.timeCapsuleAlbumIds.has(1)).toBe(true)
    expect(result.timeCapsuleAlbumIds.size).toBe(1)
  })

  it('should identify albums on repeat (3+ plays in last 2 weeks)', () => {
    const contexts: PlaybackContext[] = [
      { contextType: 'album', contextId: '1', playedAt: '2024-05-25T00:00:00Z' }, // 7 days ago
      { contextType: 'album', contextId: '1', playedAt: '2024-05-26T00:00:00Z' }, // 6 days ago
      { contextType: 'album', contextId: '1', playedAt: '2024-05-27T00:00:00Z' }, // 5 days ago
      { contextType: 'album', contextId: '2', playedAt: '2024-05-28T00:00:00Z' }, // 4 days ago
      { contextType: 'album', contextId: '2', playedAt: '2024-05-29T00:00:00Z' }, // 3 days ago
    ]

    const result = categorizeAlbumsByPlayback(contexts, now)

    // Album 1 has 3 plays (on repeat)
    expect(result.onRepeatAlbumIds).toContain(1)
    // Album 2 has only 2 plays (not on repeat)
    expect(result.onRepeatAlbumIds).not.toContain(2)
  })

  it('should sort on-repeat albums by play count (descending)', () => {
    const contexts: PlaybackContext[] = [
      // Album 1: 3 plays
      { contextType: 'album', contextId: '1', playedAt: '2024-05-25T00:00:00Z' },
      { contextType: 'album', contextId: '1', playedAt: '2024-05-26T00:00:00Z' },
      { contextType: 'album', contextId: '1', playedAt: '2024-05-27T00:00:00Z' },
      // Album 2: 5 plays (most played)
      { contextType: 'album', contextId: '2', playedAt: '2024-05-25T00:00:00Z' },
      { contextType: 'album', contextId: '2', playedAt: '2024-05-26T00:00:00Z' },
      { contextType: 'album', contextId: '2', playedAt: '2024-05-27T00:00:00Z' },
      { contextType: 'album', contextId: '2', playedAt: '2024-05-28T00:00:00Z' },
      { contextType: 'album', contextId: '2', playedAt: '2024-05-29T00:00:00Z' },
      // Album 3: 4 plays
      { contextType: 'album', contextId: '3', playedAt: '2024-05-26T00:00:00Z' },
      { contextType: 'album', contextId: '3', playedAt: '2024-05-27T00:00:00Z' },
      { contextType: 'album', contextId: '3', playedAt: '2024-05-28T00:00:00Z' },
      { contextType: 'album', contextId: '3', playedAt: '2024-05-29T00:00:00Z' },
    ]

    const result = categorizeAlbumsByPlayback(contexts, now)

    // Should be sorted: [2 (5 plays), 3 (4 plays), 1 (3 plays)]
    expect(result.onRepeatAlbumIds).toEqual([2, 3, 1])
  })

  it('should exclude albums played before 2 weeks from on-repeat', () => {
    const contexts: PlaybackContext[] = [
      { contextType: 'album', contextId: '1', playedAt: '2024-05-10T00:00:00Z' }, // >2 weeks ago
      { contextType: 'album', contextId: '1', playedAt: '2024-05-11T00:00:00Z' }, // >2 weeks ago
      { contextType: 'album', contextId: '1', playedAt: '2024-05-12T00:00:00Z' }, // >2 weeks ago
    ]

    const result = categorizeAlbumsByPlayback(contexts, now)

    // Album played 3 times but all more than 2 weeks ago
    expect(result.onRepeatAlbumIds).not.toContain(1)
    expect(result.onRepeatAlbumIds.length).toBe(0)
  })

  it('should only count plays in last 2 weeks for on-repeat', () => {
    const contexts: PlaybackContext[] = [
      // Album 1: 2 plays in last 2 weeks + 2 older plays = should NOT be on repeat
      { contextType: 'album', contextId: '1', playedAt: '2024-05-29T00:00:00Z' }, // 3 days ago
      { contextType: 'album', contextId: '1', playedAt: '2024-05-28T00:00:00Z' }, // 4 days ago
      { contextType: 'album', contextId: '1', playedAt: '2024-05-10T00:00:00Z' }, // >2 weeks ago
      { contextType: 'album', contextId: '1', playedAt: '2024-05-09T00:00:00Z' }, // >2 weeks ago
    ]

    const result = categorizeAlbumsByPlayback(contexts, now)

    // Only 2 plays in window, not enough for "on repeat"
    expect(result.onRepeatAlbumIds).not.toContain(1)
  })
})

describe('selectAlbumsFromIds', () => {
  const mockAlbums: BackendAlbum[] = [
    { id: 1, title: 'Album 1', artist_name: 'Artist 1' },
    { id: 2, title: 'Album 2', artist_name: 'Artist 2' },
    { id: 3, title: 'Album 3', artist_name: 'Artist 3' },
    { id: 4, title: 'Album 4', artist_name: 'Artist 4' },
    { id: 5, title: 'Album 5', artist_name: 'Artist 5' },
  ]

  it('should select albums from provided IDs', () => {
    const albumIds = new Set([1, 3, 5])
    const result = selectAlbumsFromIds(mockAlbums, albumIds, 10)

    expect(result.length).toBe(3)
    expect(result.every(album => albumIds.has(album.id))).toBe(true)
  })

  it('should respect maxCount parameter', () => {
    const albumIds = new Set([1, 2, 3, 4, 5])
    const result = selectAlbumsFromIds(mockAlbums, albumIds, 3)

    expect(result.length).toBe(3)
  })

  it('should exclude already used album IDs', () => {
    const albumIds = new Set([1, 2, 3])
    const usedIds = new Set([2])
    const result = selectAlbumsFromIds(mockAlbums, albumIds, 10, usedIds)

    expect(result.length).toBe(2)
    expect(result.find(a => a.id === 2)).toBeUndefined()
    expect(result.find(a => a.id === 1)).toBeDefined()
    expect(result.find(a => a.id === 3)).toBeDefined()
  })

  it('should mark selected albums as used', () => {
    const albumIds = new Set([1, 2, 3])
    const usedIds = new Set<number>()
    const result = selectAlbumsFromIds(mockAlbums, albumIds, 2, usedIds)

    expect(result.length).toBe(2)
    expect(usedIds.size).toBe(2)
    result.forEach(album => {
      expect(usedIds.has(album.id)).toBe(true)
    })
  })

  it('should return empty array if no albums available', () => {
    const albumIds = new Set([1, 2])
    const result = selectAlbumsFromIds([], albumIds, 10)

    expect(result.length).toBe(0)
  })

  it('should return empty array if no album IDs provided', () => {
    const albumIds = new Set<number>()
    const result = selectAlbumsFromIds(mockAlbums, albumIds, 10)

    expect(result.length).toBe(0)
  })

  it('should handle case where all albums are already used', () => {
    const albumIds = new Set([1, 2, 3])
    const usedIds = new Set([1, 2, 3])
    const result = selectAlbumsFromIds(mockAlbums, albumIds, 10, usedIds)

    expect(result.length).toBe(0)
  })

  it('should not mutate usedIds if not provided', () => {
    const albumIds = new Set([1, 2, 3])
    const result = selectAlbumsFromIds(mockAlbums, albumIds, 2)

    // Should not throw error
    expect(result.length).toBe(2)
  })
})

describe('selectAlbumsFromOrderedIds', () => {
  const mockAlbums: BackendAlbum[] = [
    { id: 1, title: 'Album 1', artist_name: 'Artist 1' },
    { id: 2, title: 'Album 2', artist_name: 'Artist 2' },
    { id: 3, title: 'Album 3', artist_name: 'Artist 3' },
    { id: 4, title: 'Album 4', artist_name: 'Artist 4' },
    { id: 5, title: 'Album 5', artist_name: 'Artist 5' },
  ]

  it('should maintain order of album IDs', () => {
    const albumIds = [3, 1, 5, 2]
    const result = selectAlbumsFromOrderedIds(mockAlbums, albumIds, 10)

    expect(result.length).toBe(4)
    expect(result[0].id).toBe(3)
    expect(result[1].id).toBe(1)
    expect(result[2].id).toBe(5)
    expect(result[3].id).toBe(2)
  })

  it('should respect maxCount parameter', () => {
    const albumIds = [1, 2, 3, 4, 5]
    const result = selectAlbumsFromOrderedIds(mockAlbums, albumIds, 3)

    expect(result.length).toBe(3)
    expect(result[0].id).toBe(1)
    expect(result[1].id).toBe(2)
    expect(result[2].id).toBe(3)
  })

  it('should exclude already used album IDs', () => {
    const albumIds = [1, 2, 3, 4]
    const usedIds = new Set([2])
    const result = selectAlbumsFromOrderedIds(mockAlbums, albumIds, 10, usedIds)

    expect(result.length).toBe(3)
    expect(result.find(a => a.id === 2)).toBeUndefined()
    expect(result[0].id).toBe(1)
    expect(result[1].id).toBe(3)
    expect(result[2].id).toBe(4)
  })

  it('should mark selected albums as used', () => {
    const albumIds = [1, 2, 3]
    const usedIds = new Set<number>()
    const result = selectAlbumsFromOrderedIds(mockAlbums, albumIds, 2, usedIds)

    expect(result.length).toBe(2)
    expect(usedIds.size).toBe(2)
    expect(usedIds.has(1)).toBe(true)
    expect(usedIds.has(2)).toBe(true)
  })

  it('should return empty array if no albums available', () => {
    const albumIds = [1, 2]
    const result = selectAlbumsFromOrderedIds([], albumIds, 10)

    expect(result.length).toBe(0)
  })

  it('should return empty array if no album IDs provided', () => {
    const albumIds: number[] = []
    const result = selectAlbumsFromOrderedIds(mockAlbums, albumIds, 10)

    expect(result.length).toBe(0)
  })

  it('should skip album IDs not found in allAlbums', () => {
    const albumIds = [1, 999, 3, 888]
    const result = selectAlbumsFromOrderedIds(mockAlbums, albumIds, 10)

    expect(result.length).toBe(2)
    expect(result[0].id).toBe(1)
    expect(result[1].id).toBe(3)
  })

  it('should stop at maxCount even with more albums available', () => {
    const albumIds = [1, 2, 3, 4, 5]
    const usedIds = new Set([2]) // Skip 2
    const result = selectAlbumsFromOrderedIds(mockAlbums, albumIds, 2, usedIds)

    // Should return [1, 3] and stop, not continue to 4
    expect(result.length).toBe(2)
    expect(result[0].id).toBe(1)
    expect(result[1].id).toBe(3)
  })
})
