/**
 * Queue utility functions
 */

/**
 * Remove consecutive duplicate tracks from a queue
 * Prevents the same track from playing twice in a row
 *
 * @param queue Array of tracks with trackId/id property
 * @param idKey The key to use for identifying duplicates (default: 'trackId')
 * @returns Deduplicated queue with consecutive duplicates removed
 *
 * @example
 * // Input: [Track A, Track A, Track B, Track A]
 * // Output: [Track A, Track B, Track A]
 */
export function removeConsecutiveDuplicates<T extends { [key: string]: any }>(
  queue: T[],
  idKey: string = 'trackId'
): T[] {
  if (queue.length === 0) return queue;

  const result: T[] = [queue[0]];

  for (let i = 1; i < queue.length; i++) {
    const currentId = queue[i][idKey];
    const previousId = queue[i - 1][idKey];

    // Only add if different from the previous track
    if (currentId !== previousId) {
      result.push(queue[i]);
    }
  }

  return result;
}

/**
 * Remove all duplicate tracks from a queue (keeps first occurrence)
 *
 * @param queue Array of tracks with trackId/id property
 * @param idKey The key to use for identifying duplicates (default: 'trackId')
 * @returns Deduplicated queue with all duplicates removed
 */
export function removeAllDuplicates<T extends { [key: string]: any }>(
  queue: T[],
  idKey: string = 'trackId'
): T[] {
  const seen = new Set<any>();
  const result: T[] = [];

  for (const track of queue) {
    const id = track[idKey];
    if (!seen.has(id)) {
      seen.add(id);
      result.push(track);
    }
  }

  return result;
}
