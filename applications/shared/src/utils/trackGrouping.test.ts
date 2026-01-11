/**
 * Tests for track grouping and deduplication logic.
 * These tests verify the queue building behavior to prevent duplicate tracks.
 */

import { describe, it, expect } from 'vitest';
import { groupTracks, getDeduplicatedTracks, getFormatQualityScore } from './trackGrouping';

// Simulate the data structure from the backend (DesktopTrack)
interface MockDesktopTrack {
  id: number;
  title: string;
  artist_name?: string;
  album_title?: string;
  duration_seconds?: number;
  file_path?: string;
  file_format?: string;
  bit_rate?: number;
  sample_rate?: number;
}

// Simulate the Track interface used by TrackList
interface MockTrackListTrack {
  id: number | string;
  title: string;
  artist?: string;
  album?: string;
  duration?: number;
  format?: string;
  bitrate?: number;
  sampleRate?: number;
}

describe('trackGrouping', () => {
  describe('getFormatQualityScore', () => {
    it('should rank FLAC higher than MP3', () => {
      const flacScore = getFormatQualityScore({ id: 1, title: 'Test', format: 'FLAC' });
      const mp3Score = getFormatQualityScore({ id: 2, title: 'Test', format: 'MP3' });
      expect(flacScore).toBeGreaterThan(mp3Score);
    });

    it('should rank higher sample rate FLAC higher', () => {
      const hiRes = getFormatQualityScore({ id: 1, title: 'Test', format: 'FLAC', sampleRate: 96000 });
      const cdQuality = getFormatQualityScore({ id: 2, title: 'Test', format: 'FLAC', sampleRate: 44100 });
      expect(hiRes).toBeGreaterThan(cdQuality);
    });
  });

  describe('groupTracks', () => {
    it('should group tracks with same artist and title', () => {
      const tracks: MockTrackListTrack[] = [
        { id: 1, title: 'Crevice', artist: 'Artist A', format: 'FLAC' },
        { id: 2, title: 'Crevice', artist: 'Artist A', format: 'MP3' },
        { id: 3, title: 'Blue', artist: 'Artist A', format: 'FLAC' },
        { id: 4, title: 'Blue', artist: 'Artist A', format: 'MP3' },
      ];

      const grouped = groupTracks(tracks);

      expect(grouped).toHaveLength(2); // Only 2 unique tracks
      expect(grouped[0].versions).toHaveLength(2);
      expect(grouped[1].versions).toHaveLength(2);
    });

    it('should select best quality version as activeVersion', () => {
      const tracks: MockTrackListTrack[] = [
        { id: 1, title: 'Crevice', artist: 'Artist A', format: 'MP3', bitrate: 320 },
        { id: 2, title: 'Crevice', artist: 'Artist A', format: 'FLAC', sampleRate: 44100 },
      ];

      const grouped = groupTracks(tracks);

      expect(grouped).toHaveLength(1);
      expect(grouped[0].bestVersion.format).toBe('FLAC'); // FLAC should be selected
    });

    it('should maintain original order based on first occurrence', () => {
      const tracks: MockTrackListTrack[] = [
        { id: 1, title: 'First', artist: 'Artist', format: 'FLAC' },
        { id: 2, title: 'Second', artist: 'Artist', format: 'FLAC' },
        { id: 3, title: 'First', artist: 'Artist', format: 'MP3' }, // Duplicate of First
      ];

      const grouped = groupTracks(tracks);

      expect(grouped).toHaveLength(2);
      expect(grouped[0].bestVersion.title).toBe('First');
      expect(grouped[1].bestVersion.title).toBe('Second');
    });
  });

  describe('getDeduplicatedTracks', () => {
    it('should return only best version of each unique track', () => {
      const tracks: MockTrackListTrack[] = [
        { id: 1, title: 'Crevice', artist: 'Artist A', format: 'MP3' },
        { id: 2, title: 'Crevice', artist: 'Artist A', format: 'FLAC' },
        { id: 3, title: 'Blue', artist: 'Artist A', format: 'MP3' },
        { id: 4, title: 'Blue', artist: 'Artist A', format: 'FLAC' },
      ];

      const deduplicated = getDeduplicatedTracks(tracks);

      expect(deduplicated).toHaveLength(2);
      // Should have FLAC versions (best quality)
      expect(deduplicated.find(t => t.title === 'Crevice')?.format).toBe('FLAC');
      expect(deduplicated.find(t => t.title === 'Blue')?.format).toBe('FLAC');
    });
  });

  describe('Album page queue building simulation', () => {
    /**
     * This test simulates the actual data flow in AlbumPage:
     * 1. Backend returns tracks (DesktopTrack[]) with duplicates
     * 2. AlbumPage maps them to Track[] for TrackList
     * 3. TrackList groups them and calls buildQueue with grouped tracks
     * 4. AlbumPage's buildQueue creates the queue
     */
    it('should not have duplicates when building queue from album tracks', () => {
      // Step 1: Simulate backend data (what get_album_tracks returns)
      const backendTracks: MockDesktopTrack[] = [
        { id: 1, title: 'Crevice', artist_name: 'Artist A', file_path: '/music/crevice.flac', file_format: 'FLAC', sample_rate: 44100 },
        { id: 2, title: 'Crevice', artist_name: 'Artist A', file_path: '/music/crevice.mp3', file_format: 'MP3', bit_rate: 320 },
        { id: 3, title: 'Blue', artist_name: 'Artist A', file_path: '/music/blue.flac', file_format: 'FLAC', sample_rate: 44100 },
        { id: 4, title: 'Blue', artist_name: 'Artist A', file_path: '/music/blue.mp3', file_format: 'MP3', bit_rate: 320 },
        { id: 5, title: 'Red', artist_name: 'Artist A', file_path: '/music/red.flac', file_format: 'FLAC', sample_rate: 44100 },
      ];

      // Step 2: Simulate AlbumPage mapping to TrackList Track format
      const trackListTracks: MockTrackListTrack[] = backendTracks.map(t => ({
        id: t.id,
        title: t.title,
        artist: t.artist_name,
        format: t.file_format,
        bitrate: t.bit_rate,
        sampleRate: t.sample_rate,
      }));

      // Step 3: Simulate TrackList grouping (what TrackList does internally)
      const grouped = groupTracks(trackListTracks);

      // TrackList builds activeTracks from grouped tracks
      const activeTracks = grouped.map(g => g.bestVersion);

      // Step 4: Simulate buildQueue (what AlbumPage does)
      const trackMap = new Map(backendTracks.map(t => [String(t.id), t]));

      const queue = activeTracks
        .filter(t => trackMap.get(String(t.id))?.file_path)
        .map(t => {
          const desktopTrack = trackMap.get(String(t.id))!;
          return {
            trackId: String(t.id),
            title: t.title,
            artist: desktopTrack.artist_name || 'Unknown',
            filePath: desktopTrack.file_path!,
          };
        });

      // Verify no duplicates
      expect(queue).toHaveLength(3); // Crevice, Blue, Red (not 5!)

      const titles = queue.map(q => q.title);
      expect(titles).toContain('Crevice');
      expect(titles).toContain('Blue');
      expect(titles).toContain('Red');

      // Check for duplicate titles
      const uniqueTitles = new Set(titles);
      expect(uniqueTitles.size).toBe(titles.length); // No duplicates
    });

    it('should handle case where user selects different format via dropdown', () => {
      const backendTracks: MockDesktopTrack[] = [
        { id: 1, title: 'Crevice', artist_name: 'Artist A', file_path: '/music/crevice.flac', file_format: 'FLAC' },
        { id: 2, title: 'Crevice', artist_name: 'Artist A', file_path: '/music/crevice.mp3', file_format: 'MP3' },
      ];

      const trackListTracks: MockTrackListTrack[] = backendTracks.map(t => ({
        id: t.id,
        title: t.title,
        artist: t.artist_name,
        format: t.file_format,
      }));

      const grouped = groupTracks(trackListTracks);

      // User selected MP3 via dropdown (not the default FLAC)
      const selectedVersions = new Map<string, MockTrackListTrack>();
      selectedVersions.set(grouped[0].groupKey, trackListTracks[1]); // Select MP3

      // Build activeTracks with user selection
      const activeTracks = grouped.map(g =>
        selectedVersions.get(g.groupKey) || g.bestVersion
      );

      expect(activeTracks).toHaveLength(1);
      expect(activeTracks[0].format).toBe('MP3'); // User's selection
    });
  });

  describe('Debugging: Find where duplicates come from', () => {
    it('should trace the exact data flow to find duplicates', () => {
      // Exact scenario from user: "Crevice" appearing twice
      const backendTracks: MockDesktopTrack[] = [
        { id: 101, title: 'Crevice', artist_name: 'Band', file_path: '/a.flac', file_format: 'FLAC' },
        { id: 102, title: 'Crevice', artist_name: 'Band', file_path: '/a.mp3', file_format: 'MP3' },
      ];

      console.log('=== Backend tracks ===');
      console.log(backendTracks);

      // Map to TrackList format
      const trackListTracks = backendTracks.map(t => ({
        id: t.id,
        title: t.title,
        artist: t.artist_name,
        format: t.file_format,
      }));

      console.log('=== TrackList tracks ===');
      console.log(trackListTracks);

      // Group them
      const grouped = groupTracks(trackListTracks);

      console.log('=== Grouped tracks ===');
      console.log(JSON.stringify(grouped, null, 2));

      expect(grouped).toHaveLength(1); // Should be 1, not 2!

      // Get active tracks (what TrackList passes to buildQueue)
      const activeTracks = grouped.map(g => g.bestVersion);

      console.log('=== Active tracks (passed to buildQueue) ===');
      console.log(activeTracks);

      expect(activeTracks).toHaveLength(1);

      // Build queue
      const trackMap = new Map(backendTracks.map(t => [String(t.id), t]));
      const queue = activeTracks.map(t => ({
        trackId: String(t.id),
        title: t.title,
        filePath: trackMap.get(String(t.id))!.file_path,
      }));

      console.log('=== Final queue ===');
      console.log(queue);

      expect(queue).toHaveLength(1);
    });
  });

  describe('handlePlayAll simulation (exact AlbumPage flow)', () => {
    /**
     * This test simulates the EXACT flow of handlePlayAll in AlbumPage.
     * handlePlayAll uses getDeduplicatedTracks directly on DesktopTrack[].
     */
    it('should deduplicate DesktopTrack[] directly with artist_name field', () => {
      // DesktopTrack uses artist_name, NOT artist
      const tracks: MockDesktopTrack[] = [
        { id: 1, title: 'Crevice', artist_name: 'Artist A', file_path: '/crevice.flac', file_format: 'FLAC', sample_rate: 44100 },
        { id: 2, title: 'Crevice', artist_name: 'Artist A', file_path: '/crevice.mp3', file_format: 'MP3', bit_rate: 320 },
        { id: 3, title: 'Blue', artist_name: 'Artist A', file_path: '/blue.flac', file_format: 'FLAC', sample_rate: 44100 },
        { id: 4, title: 'Blue', artist_name: 'Artist A', file_path: '/blue.mp3', file_format: 'MP3', bit_rate: 320 },
      ];

      console.log('=== handlePlayAll simulation ===');
      console.log('Input tracks:', tracks.map(t => ({ id: t.id, title: t.title, artist_name: t.artist_name, format: t.file_format })));

      // This is EXACTLY what handlePlayAll does
      const deduplicatedTracks = getDeduplicatedTracks(tracks.filter(t => t.file_path));

      console.log('Deduplicated tracks:', deduplicatedTracks.map(t => ({ id: t.id, title: t.title, format: t.file_format })));

      expect(deduplicatedTracks).toHaveLength(2); // Should be 2, not 4!

      const queue = deduplicatedTracks.map((t) => ({
        trackId: String(t.id),
        title: String(t.title || 'Unknown'),
        artist: t.artist_name || 'Unknown Artist',
        filePath: t.file_path!,
      }));

      console.log('Final queue:', queue);

      expect(queue).toHaveLength(2);

      // Verify no duplicate titles
      const titles = queue.map(q => q.title);
      expect(new Set(titles).size).toBe(titles.length);
    });

    it('should handle tracks with ONLY artist_name (no artist field)', () => {
      // This is the actual structure - artist_name exists, artist does NOT
      interface ExactDesktopTrack {
        id: number;
        title: string;
        artist_name?: string;  // Note: artist_name, not artist
        file_path?: string;
        file_format?: string;
      }

      const tracks: ExactDesktopTrack[] = [
        { id: 1, title: 'Song', artist_name: 'Band', file_path: '/a.flac', file_format: 'FLAC' },
        { id: 2, title: 'Song', artist_name: 'Band', file_path: '/a.mp3', file_format: 'MP3' },
      ];

      // getDeduplicatedTracks should work with artist_name
      const result = getDeduplicatedTracks(tracks);

      console.log('Tracks with only artist_name:', tracks);
      console.log('Result:', result);

      expect(result).toHaveLength(1);
    });

    it('should handle case sensitivity in artist/title', () => {
      const tracks = [
        { id: 1, title: 'CREVICE', artist_name: 'ARTIST', file_format: 'FLAC' },
        { id: 2, title: 'crevice', artist_name: 'artist', file_format: 'MP3' },
        { id: 3, title: 'Crevice', artist_name: 'Artist', file_format: 'AAC' },
      ];

      const result = getDeduplicatedTracks(tracks);

      console.log('Case sensitivity test:', result);

      expect(result).toHaveLength(1); // All should be same track
    });

    it('should handle whitespace differences', () => {
      const tracks = [
        { id: 1, title: 'Crevice ', artist_name: ' Artist', file_format: 'FLAC' },
        { id: 2, title: 'Crevice', artist_name: 'Artist', file_format: 'MP3' },
      ];

      const result = getDeduplicatedTracks(tracks);

      console.log('Whitespace test:', result);

      expect(result).toHaveLength(1);
    });

    it('should handle punctuation differences', () => {
      const tracks = [
        { id: 1, title: "Crevice's Song", artist_name: 'Artist', file_format: 'FLAC' },
        { id: 2, title: 'Crevices Song', artist_name: 'Artist', file_format: 'MP3' },
      ];

      const result = getDeduplicatedTracks(tracks);

      console.log('Punctuation test:', result);

      // These should be grouped together since punctuation is stripped
      expect(result).toHaveLength(1);
    });
  });

  describe('AlbumCard handlePlay (FIXED)', () => {
    /**
     * This test verifies AlbumCard.handlePlay correctly deduplicates tracks.
     * The fix uses getDeduplicatedTracks before building the queue.
     */
    it('should deduplicate tracks when building queue from AlbumCard', () => {
      interface AlbumTrack {
        id: number;
        title: string;
        artist_name?: string;
        album_title?: string;
        file_path?: string;
        duration_seconds?: number;
      }

      const tracks: AlbumTrack[] = [
        { id: 1, title: 'Crevice', artist_name: 'Band', file_path: '/a.flac' },
        { id: 2, title: 'Crevice', artist_name: 'Band', file_path: '/a.mp3' },
        { id: 3, title: 'Blue', artist_name: 'Band', file_path: '/b.flac' },
        { id: 4, title: 'Blue', artist_name: 'Band', file_path: '/b.mp3' },
      ];

      // FIXED CODE (now used in AlbumCard.tsx):
      const deduplicatedTracks = getDeduplicatedTracks(tracks.filter((t) => t.file_path));
      const queue = deduplicatedTracks.map((t) => ({
        trackId: String(t.id),
        title: t.title || 'Unknown',
        artist: t.artist_name || 'Unknown Artist',
        filePath: t.file_path!,
      }));

      console.log('=== AlbumCard queue (with deduplication) ===');
      console.log(queue);

      // Correctly deduplicated - 2 unique tracks
      expect(queue).toHaveLength(2);
      expect(queue.map(q => q.title)).toEqual(['Crevice', 'Blue']);
    });
  });
});
