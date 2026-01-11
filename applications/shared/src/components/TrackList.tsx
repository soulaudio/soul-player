'use client';

import { useState, useMemo, useRef, useCallback } from 'react';
import { Play, Pause, Music, AlertTriangle, ChevronDown } from 'lucide-react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { usePlayerStore } from '../stores/player';
import { usePlayerCommands } from '../contexts/PlayerCommandsContext';
import type { QueueTrack } from '../contexts/PlayerCommandsContext';
import { Tooltip } from './ui/Tooltip';

export type SourceType = 'local' | 'server' | 'cached';

export interface Track {
  id: number | string;
  title: string;
  artist?: string;
  album?: string;
  duration?: number;
  trackNumber?: number;
  /** Whether the track is available (file exists). Defaults to true. */
  isAvailable?: boolean;
  /** Audio file format (e.g., 'flac', 'mp3', 'aac') */
  format?: string;
  /** Bitrate in kbps (for lossy formats) */
  bitrate?: number;
  /** Sample rate in Hz */
  sampleRate?: number;
  /** Number of audio channels */
  channels?: number;
  /** Source type: local file, server stream, or cached from server */
  sourceType?: SourceType;
  /** Name of the source (e.g., server name or folder name) */
  sourceName?: string;
  /** Whether the source is currently online (for server sources) */
  sourceOnline?: boolean;
}

/** A grouped track with multiple format versions */
interface GroupedTrack {
  /** Unique group key (artist + title normalized) */
  groupKey: string;
  /** The selected/active version to play */
  activeVersion: Track;
  /** All available versions sorted by quality (best first) */
  versions: Track[];
  /** Display index in the list */
  displayIndex: number;
}

interface TrackListProps {
  tracks: Track[];
  /** Callback to build queue from tracks - platform-specific implementation */
  buildQueue: (tracks: Track[], clickedTrack: Track, clickedIndex: number) => QueueTrack[];
  onTrackAction?: (track: Track) => void;
  /** Optional menu component to render for each track */
  renderMenu?: (track: Track) => React.ReactNode;
  /** Whether to group tracks by artist+title (default: true) */
  groupByContent?: boolean;
  /** Enable virtualization for large lists (default: false) */
  virtualized?: boolean;
  /** Height of each row in pixels when virtualized (default: 56) */
  virtualItemSize?: number;
}

function formatDuration(seconds?: number): string {
  if (!seconds) return '--:--';
  const minutes = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${minutes}:${secs.toString().padStart(2, '0')}`;
}

/** Format quality score - higher is better */
function getFormatQualityScore(track: Track): number {
  const format = (track.format || '').toUpperCase();
  const sampleRate = track.sampleRate || 44100;
  const bitrate = track.bitrate || 0;

  // Base scores by format type
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

/** Get format badge styling */
function getFormatStyle(format: string): { bg: string; text: string } {
  const formatUpper = format.toUpperCase();

  // DSD formats - purple (premium)
  if (formatUpper.startsWith('DSD') || formatUpper === 'DSF' || formatUpper === 'DFF') {
    return { bg: 'bg-purple-500/15', text: 'text-purple-400' };
  }

  // Lossless formats - blue
  if (['FLAC', 'ALAC', 'WAV', 'AIFF', 'APE', 'WV'].includes(formatUpper)) {
    return { bg: 'bg-blue-500/15', text: 'text-blue-400' };
  }

  // High quality lossy - green
  if (['OPUS', 'AAC'].includes(formatUpper)) {
    return { bg: 'bg-emerald-500/15', text: 'text-emerald-400' };
  }

  // Standard lossy - neutral
  if (['MP3', 'OGG', 'M4A', 'WMA'].includes(formatUpper)) {
    return { bg: 'bg-zinc-500/15', text: 'text-zinc-400' };
  }

  // Default
  return { bg: 'bg-zinc-500/10', text: 'text-zinc-500' };
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
function createGroupKey(track: Track): string {
  const artist = normalizeForGrouping(track.artist);
  const title = normalizeForGrouping(track.title);
  return `${artist}::${title}`;
}

/** Group tracks by artist+title and select best quality */
function groupTracks(tracks: Track[]): GroupedTrack[] {
  const groups = new Map<string, Track[]>();

  // Group by normalized artist+title
  for (const track of tracks) {
    const key = createGroupKey(track);
    const existing = groups.get(key) || [];
    existing.push(track);
    groups.set(key, existing);
  }

  // Convert to GroupedTrack array, sorted by quality
  const result: GroupedTrack[] = [];
  let displayIndex = 0;

  // Maintain original order based on first occurrence
  const seenKeys = new Set<string>();
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
      activeVersion: sortedVersions[0], // Best quality by default
      versions: sortedVersions,
      displayIndex: displayIndex++,
    });
  }

  return result;
}

/** Format selector dropdown */
function FormatDropdown({
  versions,
  activeVersion,
  onSelect,
}: {
  versions: Track[];
  activeVersion: Track;
  onSelect: (track: Track) => void;
}) {
  const [isOpen, setIsOpen] = useState(false);
  const activeStyle = getFormatStyle(activeVersion.format || '');

  if (versions.length <= 1) {
    // Single format - just show badge
    return activeVersion.format ? (
      <span
        className={`inline-flex items-center text-[10px] font-medium px-1.5 py-0.5 rounded ${activeStyle.bg} ${activeStyle.text}`}
      >
        {activeVersion.format.toUpperCase()}
      </span>
    ) : null;
  }

  return (
    <div className="relative">
      <button
        onClick={(e) => {
          e.stopPropagation();
          setIsOpen(!isOpen);
        }}
        className={`inline-flex items-center gap-0.5 text-[10px] font-medium px-1.5 py-0.5 rounded transition-colors ${activeStyle.bg} ${activeStyle.text} hover:opacity-80`}
      >
        {activeVersion.format?.toUpperCase()}
        <ChevronDown className="w-3 h-3" />
      </button>

      {isOpen && (
        <>
          {/* Backdrop to close dropdown */}
          <div className="fixed inset-0 z-40" onClick={() => setIsOpen(false)} />

          {/* Dropdown menu */}
          <div className="absolute left-0 top-full mt-1 z-50 bg-popover border border-border rounded-md shadow-lg py-1 min-w-[120px]">
            {versions.map((version) => {
              const style = getFormatStyle(version.format || '');
              const isActive = version.id === activeVersion.id;
              const qualityInfo = [];
              if (version.sampleRate) {
                qualityInfo.push(`${Math.round(version.sampleRate / 1000)}kHz`);
              }
              if (version.bitrate) {
                qualityInfo.push(`${version.bitrate}kbps`);
              }

              return (
                <button
                  key={String(version.id)}
                  onClick={(e) => {
                    e.stopPropagation();
                    onSelect(version);
                    setIsOpen(false);
                  }}
                  className={`w-full px-3 py-1.5 text-left text-xs flex items-center justify-between gap-2 hover:bg-muted/50 ${
                    isActive ? 'bg-muted/30' : ''
                  }`}
                >
                  <span className={`font-medium ${style.text}`}>
                    {version.format?.toUpperCase()}
                  </span>
                  {qualityInfo.length > 0 && (
                    <span className="text-muted-foreground">{qualityInfo.join(' / ')}</span>
                  )}
                </button>
              );
            })}
          </div>
        </>
      )}
    </div>
  );
}

/** Single track row component - memoized for performance */
function TrackRow({
  group,
  getActiveVersion,
  onVersionSelect,
  onPlay,
  onPause,
  isCurrentTrack,
  isPlaying,
  isHovered,
  onMouseEnter,
  onMouseLeave,
  renderMenu,
}: {
  group: GroupedTrack;
  getActiveVersion: (group: GroupedTrack) => Track;
  onVersionSelect: (groupKey: string, track: Track) => void;
  onPlay: (group: GroupedTrack) => void;
  onPause: () => void;
  isCurrentTrack: boolean;
  isPlaying: boolean;
  isHovered: boolean;
  onMouseEnter: () => void;
  onMouseLeave: () => void;
  renderMenu?: (track: Track) => React.ReactNode;
}) {
  const activeVersion = getActiveVersion(group);
  const showPauseButton = isCurrentTrack && isPlaying;
  const isUnavailable = activeVersion.isAvailable === false;

  return (
    <div
      className={`grid grid-cols-[40px_minmax(200px,1fr)_minmax(120px,180px)_minmax(120px,180px)_70px_70px_40px] gap-4 px-4 py-3 hover:bg-muted/50 border-b last:border-b-0 transition-colors group ${
        isCurrentTrack ? 'bg-accent/20' : ''
      } ${isUnavailable ? 'opacity-60' : ''}`}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
      onDoubleClick={() => !isUnavailable && onPlay(group)}
    >
      {/* Track number / Play button */}
      <div className="flex items-center justify-center">
        {isUnavailable ? (
          <Tooltip content="File not found" position="right">
            <div className="w-8 h-8 flex items-center justify-center text-amber-500">
              <AlertTriangle className="w-4 h-4" />
            </div>
          </Tooltip>
        ) : isHovered || isCurrentTrack ? (
          <button
            onClick={() => (showPauseButton ? onPause() : onPlay(group))}
            className="w-8 h-8 flex items-center justify-center rounded hover:bg-primary/10 transition-colors"
            aria-label={showPauseButton ? 'Pause' : 'Play'}
          >
            {showPauseButton ? (
              <Pause className="w-4 h-4" fill="currentColor" />
            ) : (
              <Play className="w-4 h-4" fill="currentColor" />
            )}
          </button>
        ) : (
          <div className="w-8 h-8 flex items-center justify-center text-muted-foreground text-sm">
            {activeVersion.trackNumber || group.displayIndex + 1}
          </div>
        )}
      </div>

      {/* Title */}
      <div className="flex items-center min-w-0">
        <span
          className={`truncate ${isCurrentTrack ? 'text-primary font-medium' : ''} ${isUnavailable ? 'line-through' : ''}`}
        >
          {activeVersion.title}
        </span>
      </div>

      {/* Artist */}
      <div className="flex items-center text-sm text-muted-foreground truncate">
        {activeVersion.artist || 'Unknown Artist'}
      </div>

      {/* Album */}
      <div className="flex items-center text-sm text-muted-foreground truncate">
        {activeVersion.album || '—'}
      </div>

      {/* Format dropdown */}
      <div className="flex items-center">
        <FormatDropdown
          versions={group.versions}
          activeVersion={activeVersion}
          onSelect={(track) => onVersionSelect(group.groupKey, track)}
        />
      </div>

      {/* Duration */}
      <div className="flex items-center justify-end text-sm text-muted-foreground font-mono">
        {formatDuration(activeVersion.duration)}
      </div>

      {/* Menu */}
      <div className="flex items-center justify-end">{renderMenu?.(activeVersion)}</div>
    </div>
  );
}

export function TrackList({
  tracks,
  buildQueue,
  onTrackAction,
  renderMenu,
  groupByContent = true,
  virtualized = false,
  virtualItemSize = 56,
}: TrackListProps) {
  const [hoveredGroupKey, setHoveredGroupKey] = useState<string | null>(null);
  const [selectedVersions, setSelectedVersions] = useState<Map<string, Track>>(new Map());
  const { currentTrack, isPlaying } = usePlayerStore();
  const commands = usePlayerCommands();
  const parentRef = useRef<HTMLDivElement>(null);

  // Group tracks by content
  const groupedTracks = useMemo(() => {
    if (!groupByContent) {
      // No grouping - treat each track as its own group
      return tracks.map((track, index) => ({
        groupKey: String(track.id),
        activeVersion: track,
        versions: [track],
        displayIndex: index,
      }));
    }
    return groupTracks(tracks);
  }, [tracks, groupByContent]);

  // Virtualizer for large lists
  const rowVirtualizer = useVirtualizer({
    count: groupedTracks.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => virtualItemSize,
    overscan: 10,
    enabled: virtualized,
  });

  // Get the active version for a group (user-selected or best quality)
  const getActiveVersion = useCallback((group: GroupedTrack): Track => {
    return selectedVersions.get(group.groupKey) || group.activeVersion;
  }, [selectedVersions]);

  const handleVersionSelect = useCallback((groupKey: string, track: Track) => {
    setSelectedVersions((prev) => new Map(prev).set(groupKey, track));
  }, []);

  const handlePlay = useCallback(async (group: GroupedTrack) => {
    const activeVersion = getActiveVersion(group);

    // Build queue using all grouped tracks' active versions
    const activeTracks = groupedTracks.map((g) => getActiveVersion(g));
    const clickedIndex = groupedTracks.findIndex((g) => g.groupKey === group.groupKey);

    const queue = buildQueue(activeTracks, activeVersion, clickedIndex);

    console.log('[TrackList] Playing queue with', queue.length, 'tracks');

    try {
      await commands.playQueue(queue, 0);
      onTrackAction?.(activeVersion);
    } catch (error) {
      console.error('[TrackList] Failed to play track:', error);
    }
  }, [groupedTracks, getActiveVersion, buildQueue, commands, onTrackAction]);

  const handlePause = useCallback(async () => {
    try {
      await commands.pausePlayback();
    } catch (error) {
      console.error('[TrackList] Failed to pause:', error);
    }
  }, [commands]);

  if (tracks.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
        <Music className="w-12 h-12 mb-4 opacity-50" />
        <p>No tracks found</p>
        <p className="text-sm mt-1">Add music to get started</p>
      </div>
    );
  }

  // Virtualized rendering
  if (virtualized) {
    return (
      <div className="border rounded-lg overflow-hidden h-full flex flex-col">
        {/* Header */}
        <div className="bg-muted/50 flex-shrink-0">
          <div className="grid grid-cols-[40px_minmax(200px,1fr)_minmax(120px,180px)_minmax(120px,180px)_70px_70px_40px] gap-4 px-4 py-2 text-sm font-medium text-muted-foreground">
            <Tooltip content="Track number" position="top" delay={700}>
              <div>#</div>
            </Tooltip>
            <Tooltip content="Track title" position="top" delay={700}>
              <div>Title</div>
            </Tooltip>
            <Tooltip content="Artist name" position="top" delay={700}>
              <div>Artist</div>
            </Tooltip>
            <Tooltip content="Album title" position="top" delay={700}>
              <div>Album</div>
            </Tooltip>
            <Tooltip content="Audio format" position="top" delay={700}>
              <div>Format</div>
            </Tooltip>
            <Tooltip content="Track duration" position="top" delay={700}>
              <div className="text-right">Duration</div>
            </Tooltip>
            <div></div>
          </div>
        </div>

        {/* Virtualized track rows */}
        <div
          ref={parentRef}
          className="flex-1 overflow-auto"
        >
          <div
            style={{
              height: `${rowVirtualizer.getTotalSize()}px`,
              width: '100%',
              position: 'relative',
            }}
          >
            {rowVirtualizer.getVirtualItems().map((virtualRow) => {
              const group = groupedTracks[virtualRow.index];
              const activeVersion = getActiveVersion(group);
              const trackId = String(activeVersion.id);
              const isCurrentTrack = String(currentTrack?.id) === trackId;

              return (
                <div
                  key={virtualRow.key}
                  style={{
                    position: 'absolute',
                    top: 0,
                    left: 0,
                    width: '100%',
                    height: `${virtualRow.size}px`,
                    transform: `translateY(${virtualRow.start}px)`,
                  }}
                >
                  <TrackRow
                    group={group}
                    getActiveVersion={getActiveVersion}
                    onVersionSelect={handleVersionSelect}
                    onPlay={handlePlay}
                    onPause={handlePause}
                    isCurrentTrack={isCurrentTrack}
                    isPlaying={isPlaying}
                    isHovered={hoveredGroupKey === group.groupKey}
                    onMouseEnter={() => setHoveredGroupKey(group.groupKey)}
                    onMouseLeave={() => setHoveredGroupKey(null)}
                    renderMenu={renderMenu}
                  />
                </div>
              );
            })}
          </div>
        </div>
      </div>
    );
  }

  // Non-virtualized rendering (original behavior)
  return (
    <div className="border rounded-lg overflow-hidden">
      {/* Header */}
      <div className="bg-muted/50">
        <div className="grid grid-cols-[40px_minmax(200px,1fr)_minmax(120px,180px)_minmax(120px,180px)_70px_70px_40px] gap-4 px-4 py-2 text-sm font-medium text-muted-foreground">
          <Tooltip content="Track number" position="top" delay={700}>
            <div>#</div>
          </Tooltip>
          <Tooltip content="Track title" position="top" delay={700}>
            <div>Title</div>
          </Tooltip>
          <Tooltip content="Artist name" position="top" delay={700}>
            <div>Artist</div>
          </Tooltip>
          <Tooltip content="Album title" position="top" delay={700}>
            <div>Album</div>
          </Tooltip>
          <Tooltip content="Audio format" position="top" delay={700}>
            <div>Format</div>
          </Tooltip>
          <Tooltip content="Track duration" position="top" delay={700}>
            <div className="text-right">Duration</div>
          </Tooltip>
          <div></div>
        </div>
      </div>

      {/* Track rows */}
      <div>
        {groupedTracks.map((group) => {
          const activeVersion = getActiveVersion(group);
          const trackId = String(activeVersion.id);
          const isCurrentTrack = String(currentTrack?.id) === trackId;

          return (
            <TrackRow
              key={group.groupKey}
              group={group}
              getActiveVersion={getActiveVersion}
              onVersionSelect={handleVersionSelect}
              onPlay={handlePlay}
              onPause={handlePause}
              isCurrentTrack={isCurrentTrack}
              isPlaying={isPlaying}
              isHovered={hoveredGroupKey === group.groupKey}
              onMouseEnter={() => setHoveredGroupKey(group.groupKey)}
              onMouseLeave={() => setHoveredGroupKey(null)}
              renderMenu={renderMenu}
            />
          );
        })}
      </div>
    </div>
  );
}
