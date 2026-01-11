/**
 * Track grouping utilities for merging duplicate tracks with different formats.
 * Used across pages to ensure consistent behavior.
 */

export interface TrackForGrouping {
  id: number | string;
  title: string;
  artist?: string;
  artist_name?: string;
  format?: string;
  file_format?: string;
  bitrate?: number;
  bit_rate?: number;
  sampleRate?: number;
  sample_rate?: number;
  // Additional properties that may be present on tracks
  album_title?: string;
  album?: string;
  file_path?: string;
  path?: string;
  duration_seconds?: number;
  duration?: number;
  track_number?: number;
  trackNumber?: number;
}

export interface GroupedTrack<T extends TrackForGrouping> {
  groupKey: string;
  /** Best quality version (auto-selected) */
  bestVersion: T;
  /** All versions sorted by quality (best first) */
  versions: T[];
}

/** Format quality score - higher is better */
export function getFormatQualityScore(track: TrackForGrouping): number {
  const format = (track.format || track.file_format || '').toUpperCase();
  const sampleRate = track.sampleRate || track.sample_rate || 44100;
  const bitrate = track.bitrate || track.bit_rate || 0;

  let baseScore = 0;

  // DSD formats (highest quality)
  if (format.startsWith('DSD') || format === 'DSF' || format === 'DFF') {
    baseScore = 1000;
  }
  // Lossless formats
  else if (['FLAC', 'ALAC', 'WAV', 'AIFF', 'APE', 'WV'].includes(format)) {
    baseScore = 800;
  }
  // High-quality lossy
  else if (['OPUS'].includes(format)) {
    baseScore = 400;
  } else if (['AAC', 'M4A'].includes(format)) {
    baseScore = 350;
  }
  // Standard lossy
  else if (['OGG'].includes(format)) {
    baseScore = 300;
  } else if (['MP3'].includes(format)) {
    baseScore = 250;
  } else if (['WMA'].includes(format)) {
    baseScore = 200;
  }
  // Unknown
  else {
    baseScore = 100;
  }

  // Add sample rate bonus (normalized to 0-100)
  const sampleRateBonus = Math.min((sampleRate / 192000) * 100, 100);

  // Add bitrate bonus for lossy formats (normalized to 0-50)
  const bitrateBonus = baseScore < 800 ? Math.min((bitrate / 320) * 50, 50) : 0;

  return baseScore + sampleRateBonus + bitrateBonus;
}

/** Normalize string for grouping comparison */
function normalizeForGrouping(str: string | undefined): string {
  return (str || '')
    .toLowerCase()
    .trim()
    .replace(/[^\w\s]/g, '') // Remove punctuation
    .replace(/\s+/g, ' '); // Normalize whitespace
}

/** Create group key from track */
function createGroupKey(track: TrackForGrouping): string {
  const artist = normalizeForGrouping(track.artist || track.artist_name);
  const title = normalizeForGrouping(track.title);
  return `${artist}::${title}`;
}

/**
 * Group tracks by artist+title and select best quality version for each group.
 * Maintains original order based on first occurrence of each group.
 */
export function groupTracks<T extends TrackForGrouping>(tracks: T[]): GroupedTrack<T>[] {
  const groups = new Map<string, T[]>();

  // Group by normalized artist+title
  for (const track of tracks) {
    const key = createGroupKey(track);
    const existing = groups.get(key) || [];
    existing.push(track);
    groups.set(key, existing);
  }

  // Convert to GroupedTrack array
  const result: GroupedTrack<T>[] = [];
  const seenKeys = new Set<string>();

  // Maintain original order based on first occurrence
  for (const track of tracks) {
    const key = createGroupKey(track);
    if (seenKeys.has(key)) continue;
    seenKeys.add(key);

    const versions = groups.get(key) || [track];
    // Sort versions by quality (best first)
    const sortedVersions = [...versions].sort(
      (a, b) => getFormatQualityScore(b) - getFormatQualityScore(a)
    );

    result.push({
      groupKey: key,
      bestVersion: sortedVersions[0],
      versions: sortedVersions,
    });
  }

  return result;
}

/**
 * Get deduplicated tracks with best quality version for each unique track.
 * Use this for building queues.
 */
export function getDeduplicatedTracks<T extends TrackForGrouping>(tracks: T[]): T[] {
  return groupTracks(tracks).map(g => g.bestVersion);
}
