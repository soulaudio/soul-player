import { useEffect, useState, useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { usePlayerStore } from '@soul-player/shared/stores/player';
import { ArtworkImage, usePlayerCommands, usePlaybackEvents } from '@soul-player/shared';
import { Music, Library, Disc3, ListMusic, Users, Guitar, ChevronDown } from 'lucide-react';
import { usePlaybackContext, type PlaybackContext, type ContextType } from '../hooks/usePlaybackContext';

/** Track with full info from backend */
interface FullTrack {
  id: number;
  title: string;
  artist_name?: string;
  album_title?: string;
  track_number?: number;
  duration_seconds?: number;
  file_path?: string;
  file_format?: string;
  bit_rate?: number;
  sample_rate?: number;
}

/** Grouped track with multiple format versions */
interface GroupedTrack {
  groupKey: string;
  activeVersion: FullTrack;
  versions: FullTrack[];
  displayIndex: number;
}

/** Format quality score - higher is better */
function getFormatQualityScore(track: FullTrack): number {
  const format = (track.file_format || '').toUpperCase();
  const sampleRate = track.sample_rate || 44100;
  const bitrate = track.bit_rate || 0;

  let baseScore = 0;

  if (format.startsWith('DSD') || format === 'DSF' || format === 'DFF') {
    baseScore = 1000;
  } else if (['FLAC', 'ALAC', 'WAV', 'AIFF', 'APE', 'WV'].includes(format)) {
    baseScore = 800;
  } else if (['OPUS'].includes(format)) {
    baseScore = 400;
  } else if (['AAC', 'M4A'].includes(format)) {
    baseScore = 350;
  } else if (['OGG'].includes(format)) {
    baseScore = 300;
  } else if (['MP3'].includes(format)) {
    baseScore = 250;
  } else if (['WMA'].includes(format)) {
    baseScore = 200;
  } else {
    baseScore = 100;
  }

  const sampleRateBonus = Math.min((sampleRate / 192000) * 100, 100);
  const bitrateBonus = baseScore < 800 ? Math.min((bitrate / 320) * 50, 50) : 0;

  return baseScore + sampleRateBonus + bitrateBonus;
}

/** Get format badge styling */
function getFormatStyle(format: string): { bg: string; text: string } {
  const formatUpper = format.toUpperCase();

  if (formatUpper.startsWith('DSD') || formatUpper === 'DSF' || formatUpper === 'DFF') {
    return { bg: 'bg-purple-500/15', text: 'text-purple-400' };
  }
  if (['FLAC', 'ALAC', 'WAV', 'AIFF', 'APE', 'WV'].includes(formatUpper)) {
    return { bg: 'bg-blue-500/15', text: 'text-blue-400' };
  }
  if (['OPUS', 'AAC'].includes(formatUpper)) {
    return { bg: 'bg-emerald-500/15', text: 'text-emerald-400' };
  }
  if (['MP3', 'OGG', 'M4A', 'WMA'].includes(formatUpper)) {
    return { bg: 'bg-zinc-500/15', text: 'text-zinc-400' };
  }
  return { bg: 'bg-zinc-500/10', text: 'text-zinc-500' };
}

/** Normalize string for grouping */
function normalizeForGrouping(str: string | undefined): string {
  return (str || '')
    .toLowerCase()
    .trim()
    .replace(/[^\w\s]/g, '')
    .replace(/\s+/g, ' ');
}

/** Create group key from track */
function createGroupKey(track: FullTrack): string {
  const artist = normalizeForGrouping(track.artist_name);
  const title = normalizeForGrouping(track.title);
  return `${artist}::${title}`;
}

/** Group tracks by artist+title and select best quality */
function groupTracks(tracks: FullTrack[]): GroupedTrack[] {
  const groups = new Map<string, FullTrack[]>();

  for (const track of tracks) {
    const key = createGroupKey(track);
    const existing = groups.get(key) || [];
    existing.push(track);
    groups.set(key, existing);
  }

  const result: GroupedTrack[] = [];
  let displayIndex = 0;

  const seenKeys = new Set<string>();
  for (const track of tracks) {
    const key = createGroupKey(track);
    if (seenKeys.has(key)) continue;
    seenKeys.add(key);

    const versions = groups.get(key) || [track];
    const sortedVersions = [...versions].sort(
      (a, b) => getFormatQualityScore(b) - getFormatQualityScore(a)
    );

    result.push({
      groupKey: key,
      activeVersion: sortedVersions[0],
      versions: sortedVersions,
      displayIndex: displayIndex++,
    });
  }

  return result;
}

/** Format dropdown component */
function FormatDropdown({
  versions,
  activeVersion,
  onSelect,
}: {
  versions: FullTrack[];
  activeVersion: FullTrack;
  onSelect: (track: FullTrack) => void;
}) {
  const [isOpen, setIsOpen] = useState(false);
  const style = getFormatStyle(activeVersion.file_format || '');

  if (versions.length <= 1) {
    return activeVersion.file_format ? (
      <span className={`text-[10px] font-medium px-1.5 py-0.5 rounded ${style.bg} ${style.text}`}>
        {activeVersion.file_format.toUpperCase()}
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
        className={`inline-flex items-center gap-0.5 text-[10px] font-medium px-1.5 py-0.5 rounded transition-colors ${style.bg} ${style.text} hover:opacity-80`}
      >
        {activeVersion.file_format?.toUpperCase()}
        <ChevronDown className="w-3 h-3" />
      </button>

      {isOpen && (
        <>
          <div className="fixed inset-0 z-40" onClick={() => setIsOpen(false)} />
          <div className="absolute right-0 top-full mt-1 z-50 bg-popover border border-border rounded-md shadow-lg py-1 min-w-[140px]">
            {versions.map((version) => {
              const vStyle = getFormatStyle(version.file_format || '');
              const isActive = version.id === activeVersion.id;
              const qualityInfo = [];
              if (version.sample_rate) {
                qualityInfo.push(`${Math.round(version.sample_rate / 1000)}kHz`);
              }
              if (version.bit_rate) {
                qualityInfo.push(`${version.bit_rate}kbps`);
              }

              return (
                <button
                  key={version.id}
                  onClick={(e) => {
                    e.stopPropagation();
                    onSelect(version);
                    setIsOpen(false);
                  }}
                  className={`w-full px-3 py-1.5 text-left text-xs flex items-center justify-between gap-2 hover:bg-muted/50 ${
                    isActive ? 'bg-muted/30' : ''
                  }`}
                >
                  <span className={`font-medium ${vStyle.text}`}>
                    {version.file_format?.toUpperCase()}
                  </span>
                  {qualityInfo.length > 0 && (
                    <span className="text-muted-foreground text-[10px]">{qualityInfo.join(' ')}</span>
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

export function NowPlayingPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { currentTrack, isPlaying } = usePlayerStore();
  const commands = usePlayerCommands();
  const events = usePlaybackEvents();
  const { getCurrentContext } = usePlaybackContext();

  const [tracks, setTracks] = useState<FullTrack[]>([]);
  const [selectedVersions, setSelectedVersions] = useState<Map<string, FullTrack>>(new Map());
  const [playbackContext, setPlaybackContext] = useState<PlaybackContext | null>(null);
  const [loading, setLoading] = useState(false);

  // Fetch current playback context
  useEffect(() => {
    getCurrentContext()
      .then(setPlaybackContext)
      .catch((err) => {
        console.error('Failed to fetch playback context:', err);
        setPlaybackContext(null);
      });
  }, [currentTrack?.id, getCurrentContext]);

  // Fetch tracks based on context
  useEffect(() => {
    const loadTracks = async () => {
      if (!currentTrack) {
        setTracks([]);
        return;
      }

      setLoading(true);
      try {
        let fetchedTracks: FullTrack[] = [];

        // Try to get tracks based on context
        if (playbackContext?.contextType === 'album' && playbackContext.contextId) {
          fetchedTracks = await invoke<FullTrack[]>('get_album_tracks', {
            albumId: parseInt(playbackContext.contextId)
          });
        } else if (playbackContext?.contextType === 'artist' && playbackContext.contextId) {
          fetchedTracks = await invoke<FullTrack[]>('get_artist_tracks', {
            artistId: parseInt(playbackContext.contextId)
          });
        } else if (playbackContext?.contextType === 'genre' && playbackContext.contextId) {
          fetchedTracks = await invoke<FullTrack[]>('get_genre_tracks', {
            genreId: parseInt(playbackContext.contextId)
          });
        } else if (currentTrack.albumId) {
          // Fallback to album if available
          fetchedTracks = await invoke<FullTrack[]>('get_album_tracks', {
            albumId: currentTrack.albumId
          });
        } else {
          // Get queue and fetch full track info
          const queue = await commands.getQueue();
          if (queue.length > 0) {
            // For now, just show queue items without full format info
            fetchedTracks = queue.map((q, idx) => ({
              id: typeof q.trackId === 'string' ? parseInt(q.trackId) : q.trackId as number,
              title: q.title,
              artist_name: q.artist,
              album_title: q.album || undefined,
              duration_seconds: q.durationSeconds || undefined,
              track_number: idx + 1,
              file_path: q.filePath,
            }));
          }
        }

        setTracks(fetchedTracks);
      } catch (err) {
        console.error('Failed to load tracks:', err);
        setTracks([]);
      } finally {
        setLoading(false);
      }
    };

    loadTracks();

    const unsubscribe = events.onQueueUpdate(() => {
      loadTracks();
    });
    return unsubscribe;
  }, [currentTrack?.id, currentTrack?.albumId, playbackContext, commands, events]);

  // Group tracks
  const groupedTracks = useMemo(() => groupTracks(tracks), [tracks]);

  // Get active version for a group
  const getActiveVersion = (group: GroupedTrack): FullTrack => {
    return selectedVersions.get(group.groupKey) || group.activeVersion;
  };

  // Handle format selection - plays the selected format
  const handleFormatSelect = async (groupKey: string, track: FullTrack) => {
    setSelectedVersions(prev => new Map(prev).set(groupKey, track));

    // If this is the currently playing track group, switch to this format
    const currentGroup = groupedTracks.find(g =>
      g.versions.some(v => v.id === currentTrack?.id)
    );
    if (currentGroup?.groupKey === groupKey) {
      try {
        await commands.playTrack(track.id);
      } catch (err) {
        console.error('Failed to switch format:', err);
      }
    }
  };

  // Handle track click
  const handleTrackClick = async (group: GroupedTrack) => {
    const activeVersion = getActiveVersion(group);
    try {
      await commands.playTrack(activeVersion.id);
    } catch (err) {
      console.error('Failed to play track:', err);
    }
  };

  const formatTime = (seconds: number | undefined) => {
    if (!seconds || !isFinite(seconds)) return '--:--';
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  };

  // Empty state
  if (!currentTrack) {
    return (
      <div className="h-full flex flex-col items-center justify-center">
        <div className="w-24 h-24 rounded-full bg-muted flex items-center justify-center mb-6">
          <Music className="w-12 h-12 text-muted-foreground" />
        </div>
        <h2 className="text-xl font-medium text-muted-foreground mb-2">{t('sidebar.noTrackPlaying')}</h2>
        <p className="text-sm text-muted-foreground mb-6">{t('home.welcomeSubtitle')}</p>
        <button
          onClick={() => navigate('/library')}
          className="px-6 py-3 bg-primary text-primary-foreground rounded-lg hover:bg-primary/80 transition-colors"
        >
          {t('common.browse')}
        </button>
      </div>
    );
  }

  // Context helpers
  const getContextIcon = (contextType: ContextType | undefined) => {
    switch (contextType) {
      case 'album': return <Disc3 className="w-4 h-4 text-muted-foreground" />;
      case 'playlist': return <ListMusic className="w-4 h-4 text-muted-foreground" />;
      case 'artist': return <Users className="w-4 h-4 text-muted-foreground" />;
      case 'genre': return <Guitar className="w-4 h-4 text-muted-foreground" />;
      default: return <Library className="w-4 h-4 text-muted-foreground" />;
    }
  };

  const getContextLabel = (contextType: ContextType | undefined): string => {
    switch (contextType) {
      case 'album': return t('nowPlaying.playingFromAlbum', 'Playing from album');
      case 'playlist': return t('nowPlaying.playingFromPlaylist', 'Playing from playlist');
      case 'artist': return t('nowPlaying.playingFromArtist', 'Playing from artist');
      case 'genre': return t('nowPlaying.playingFromGenre', 'Playing from genre');
      default: return t('nowPlaying.fromLibrary', 'From Library');
    }
  };

  const headerTitle = playbackContext?.contextName || currentTrack.album || t('nowPlaying.fromLibrary');
  const headerSubtitle = getContextLabel(playbackContext?.contextType);
  const headerIcon = getContextIcon(playbackContext?.contextType);

  return (
    <div className="h-full flex items-center justify-center">
      <div className="flex gap-10 max-w-6xl w-full items-center">
        {/* Left Side - Artwork */}
        <div className="w-[500px] flex-shrink-0">
          <div className="w-full aspect-square rounded-2xl overflow-hidden shadow-2xl bg-muted">
            <ArtworkImage
              trackId={currentTrack.id}
              coverArtPath={currentTrack.coverArtPath}
              alt={currentTrack.album || currentTrack.title}
              className="w-full h-full object-cover"
              fallbackClassName="w-full h-full flex items-center justify-center bg-muted"
            />
          </div>
        </div>

        {/* Right Side - Tracklist */}
        <div className="flex-1 flex flex-col min-w-0 max-h-[500px]">
          <div className="mb-3">
            <div className="flex items-center gap-2 text-xs text-muted-foreground uppercase tracking-wide mb-1">
              {headerIcon}
              <span>{headerSubtitle}</span>
            </div>
            <h2 className="text-lg font-bold">{headerTitle}</h2>
            <p className="text-sm text-muted-foreground">
              {groupedTracks.length} {t('library.tracks')}
            </p>
          </div>

          <div className="flex-1 overflow-y-auto -mx-2">
            {loading ? (
              <div className="flex items-center justify-center h-full">
                <div className="animate-spin w-6 h-6 border-2 border-primary border-t-transparent rounded-full" />
              </div>
            ) : groupedTracks.length === 0 ? (
              <div className="flex flex-col items-center justify-center h-full text-muted-foreground">
                <Music className="w-12 h-12 mb-4 opacity-50" />
                <p>{t('sidebar.emptyQueue')}</p>
              </div>
            ) : (
              <div className="space-y-0.5">
                {groupedTracks.map((group, idx) => {
                  const activeVersion = getActiveVersion(group);
                  const isCurrentTrack = group.versions.some(v => v.id === currentTrack.id);

                  return (
                    <div
                      key={group.groupKey}
                      onClick={() => handleTrackClick(group)}
                      className={`w-full flex items-center gap-3 px-3 py-2.5 rounded-lg transition-colors cursor-pointer ${
                        isCurrentTrack
                          ? 'bg-primary/10 border border-primary/20'
                          : 'hover:bg-accent/30'
                      }`}
                    >
                      {/* Track Number or Playing Indicator */}
                      <div className="w-6 text-center flex-shrink-0">
                        {isCurrentTrack && isPlaying ? (
                          <div className="flex items-center justify-center gap-0.5">
                            <span className="w-0.5 h-3 bg-primary rounded-full animate-pulse" />
                            <span className="w-0.5 h-4 bg-primary rounded-full animate-pulse" style={{ animationDelay: '0.2s' }} />
                            <span className="w-0.5 h-2 bg-primary rounded-full animate-pulse" style={{ animationDelay: '0.4s' }} />
                          </div>
                        ) : isCurrentTrack ? (
                          <div className="flex items-center justify-center gap-0.5">
                            <span className="w-0.5 h-2 bg-primary/60 rounded-full" />
                            <span className="w-0.5 h-3 bg-primary/60 rounded-full" />
                            <span className="w-0.5 h-2 bg-primary/60 rounded-full" />
                          </div>
                        ) : (
                          <span className="text-sm text-muted-foreground">
                            {activeVersion.track_number || idx + 1}
                          </span>
                        )}
                      </div>

                      {/* Track Info */}
                      <div className="flex-1 min-w-0">
                        <p className={`truncate ${isCurrentTrack ? 'text-primary font-semibold' : 'text-sm'}`}>
                          {activeVersion.title}
                        </p>
                        <p className={`text-xs truncate ${isCurrentTrack ? 'text-primary/70' : 'text-muted-foreground'}`}>
                          {activeVersion.artist_name || currentTrack.artist}
                        </p>
                      </div>

                      {/* Format dropdown */}
                      <FormatDropdown
                        versions={group.versions}
                        activeVersion={activeVersion}
                        onSelect={(track) => handleFormatSelect(group.groupKey, track)}
                      />

                      {/* Duration */}
                      <span className={`text-xs flex-shrink-0 w-12 text-right ${isCurrentTrack ? 'text-primary/70' : 'text-muted-foreground'}`}>
                        {formatTime(activeVersion.duration_seconds)}
                      </span>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
