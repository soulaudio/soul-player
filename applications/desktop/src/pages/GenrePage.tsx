import { useEffect, useState, useCallback, useDeferredValue, useMemo } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { TrackList, type Track, type QueueTrack, getDeduplicatedTracks, TrackMenu, type BackendTrack, AddToPlaylistDialog, LibraryPageLayout, useBackend, usePlayerCommands, debug } from '@soul-player/shared';
import { Play, Guitar, Clock } from 'lucide-react';

interface Genre {
  id: number;
  name: string;
  track_count: number;
}

// Extend BackendTrack for full track data from Tauri
interface DesktopTrack extends BackendTrack {
  // Track interface fields mapped from backend
}

export function GenrePage() {
  const { t } = useTranslation();
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const backend = useBackend();
  const commands = usePlayerCommands();

  const [genre, setGenre] = useState<Genre | null>(null);
  const [tracks, setTracks] = useState<DesktopTrack[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [searchQuery, setSearchQuery] = useState('');
  const deferredSearchQuery = useDeferredValue(searchQuery);

  // Add to playlist dialog state
  const [selectedTrackForPlaylist, setSelectedTrackForPlaylist] = useState<{
    id: number
    title: string
  } | null>(null);

  const loadGenre = useCallback(async (genreId: number) => {
    setLoading(true);
    setError(null);
    try {
      const [genreData, tracksData] = await Promise.all([
        invoke<Genre | null>('get_genre_by_id', { id: genreId }),
        invoke<DesktopTrack[]>('get_genre_tracks', { genreId }),
      ]);

      if (!genreData) {
        setError(t('genre.notFound', 'Genre not found'));
        return;
      }

      setGenre(genreData);
      setTracks(tracksData);
    } catch (err) {
      debug.error('Failed to load genre:', err);
      setError(err instanceof Error ? err.message : 'Failed to load genre');
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => {
    if (!id) return;
    loadGenre(parseInt(id, 10));
  }, [id, loadGenre]);

  const filteredTracks = useMemo(() => {
    if (!deferredSearchQuery.trim()) return tracks;
    const query = deferredSearchQuery.toLowerCase();
    return tracks.filter(
      t =>
        t.title?.toLowerCase().includes(query) ||
        (t.artist_name || '').toLowerCase().includes(query) ||
        (t.album_title || '').toLowerCase().includes(query),
    );
  }, [tracks, deferredSearchQuery]);

  const buildQueue = useCallback((allTracks: Track[], _clickedTrack: Track, _clickedIndex: number): QueueTrack[] => {
    // allTracks is already deduplicated by TrackList's internal grouping
    // We need to map back to DesktopTrack to get file_path
    const trackMap = new Map(filteredTracks.map(t => [String(t.id), t]));

    // Filter to only tracks we have file_path for
    const validTracks = allTracks.filter(t => {
      const desktopTrack = trackMap.get(String(t.id));
      return desktopTrack?.file_path;
    });

    // Return the full queue in original order
    // The startIndex passed to playQueue() will determine which track plays first
    return validTracks.map((t) => {
      const desktopTrack = trackMap.get(String(t.id))!;
      return {
        trackId: String(t.id),
        title: String(t.title || 'Unknown'),
        artist: desktopTrack.artist_name || 'Unknown Artist',
        album: desktopTrack.album_title || null,
        albumId: desktopTrack.album_id || undefined,
        artistId: desktopTrack.artist_id || undefined,
        filePath: desktopTrack.file_path!,
        durationSeconds: desktopTrack.duration_seconds || null,
        trackNumber: desktopTrack.track_number || null,
      };
    });
  }, [filteredTracks]);

  const handlePlayAll = async () => {
    // Deduplicate tracks (selects best quality version for each unique track)
    const deduplicatedTracks = getDeduplicatedTracks(tracks.filter(t => t.file_path));
    if (deduplicatedTracks.length === 0) return;

    const queue = deduplicatedTracks.map((t) => ({
      trackId: String(t.id),
      title: String(t.title || 'Unknown'),
      artist: t.artist_name || 'Unknown Artist',
      album: t.album_title || null,
      albumId: t.album_id || undefined,
      artistId: t.artist_id || undefined,
      filePath: t.file_path!,
      durationSeconds: t.duration_seconds || null,
      trackNumber: t.track_number || null,
    }));

    try {
      // Record playback context
      if (genre) {
        await backend.recordContext({
          contextType: 'genre',
          contextId: String(genre.id),
          contextName: genre.name,
          contextArtworkPath: null,
        });
      }
      await commands.playQueue(queue, 0);
    } catch (err) {
      debug.error('Failed to play all tracks:', err);
    }
  };

  const formatDuration = (seconds: number): string => {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    if (hours > 0) {
      return `${hours}h ${minutes}m`;
    }
    return `${minutes} min`;
  };

  const totalDuration = tracks.reduce((acc, t) => acc + (t.duration_seconds || 0), 0);

  return (
    <LibraryPageLayout
      searchQuery={searchQuery}
      setSearchQuery={setSearchQuery}
      itemCount={tracks.length}
      searchPlaceholderKey="library.search.tracksWithCount"
      isLoading={loading}
      itemType="track"
      gridClass="grid-cols-1"
      pageTestId="genre-detail-page"
    >
      {error || !genre ? (
        <div className="flex items-center justify-center py-12">
          <div className="text-center text-destructive">
            <p className="font-medium mb-2">{error || t('genre.notFound', 'Genre not found')}</p>
            <button
              onClick={() => navigate('/genres')}
              className="mt-4 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:opacity-[var(--hover-button-opacity)] transition-opacity duration-[var(--transition-duration)]"
            >
              {t('common.back', 'Back')}
            </button>
          </div>
        </div>
      ) : (
        <>
          {/* Genre Header */}
          <div className="mb-6">
            <div className="flex items-start gap-6">
              {/* Genre Icon */}
              <div className="w-32 h-32 bg-gradient-to-br from-primary/20 to-primary/5 rounded-xl flex items-center justify-center flex-shrink-0">
                <Guitar className="w-16 h-16 text-primary" />
              </div>

              {/* Genre Info */}
              <div className="flex-1">
                <p className="text-sm text-muted-foreground uppercase tracking-wider mb-1">
                  {t('library.genre', 'Genre')}
                </p>
                <h1 data-testid="genre-title" className="text-4xl font-bold mb-2">{genre.name}</h1>
                <p data-testid="genre-track-count" className="text-sm text-muted-foreground flex items-center gap-2 mb-4">
                  <Clock className="w-4 h-4" />
                  {t('library.tracks', { count: genre.track_count })} • {formatDuration(totalDuration)}
                </p>

                <button
                  data-testid="genre-play-all-button"
                  onClick={handlePlayAll}
                  disabled={tracks.filter(t => t.file_path).length === 0}
                  className="flex items-center gap-2 px-6 py-3 bg-primary text-primary-foreground rounded-full hover:opacity-[var(--hover-button-opacity)] disabled:opacity-[var(--disabled-opacity)] transition-opacity duration-[var(--transition-duration)]"
                >
                  <Play className="w-5 h-5" fill="currentColor" />
                  <span>{t('common.playAll', 'Play All')}</span>
                </button>
              </div>
            </div>
          </div>

          {/* Track List */}
          <TrackList
            tracks={filteredTracks.map(t => ({
              id: t.id,
              title: String(t.title || 'Unknown'),
              artist: t.artist_name,
              artistId: t.artist_id,
              artists: t.artists,
              album: t.album_title,
              albumId: t.album_id,
              duration: t.duration_seconds,
              trackNumber: t.track_number,
              isAvailable: !!t.file_path,
              format: t.file_format,
              bitrate: t.bit_rate,
              sampleRate: t.sample_rate,
              channels: t.channels,
            }))}
            buildQueue={buildQueue}
            showAlbumArt={true}
            onBeforePlay={async () => {
              if (genre) {
                await backend.recordContext({
                  contextType: 'genre',
                  contextId: String(genre.id),
                  contextName: genre.name,
                  contextArtworkPath: null,
                });
              }
            }}
            onTrackAction={() => {}}
            renderMenu={(track) => {
              const desktopTrack = filteredTracks.find(t => t.id === track.id);
              if (!desktopTrack) return null;
              return (
                <TrackMenu
                  track={desktopTrack}
                  onAddToPlaylist={() => {
                    setSelectedTrackForPlaylist({
                      id: desktopTrack.id,
                      title: desktopTrack.title,
                    });
                  }}
                  onDelete={async () => {
                    await invoke('delete_track', { id: desktopTrack.id });
                    if (id) loadGenre(parseInt(id, 10));
                  }}
                />
              );
            }}
          />

          {/* Add to Playlist Dialog */}
          {selectedTrackForPlaylist && (
            <AddToPlaylistDialog
              open={!!selectedTrackForPlaylist}
              onClose={() => setSelectedTrackForPlaylist(null)}
              mode="track"
              trackId={selectedTrackForPlaylist.id}
              trackTitle={selectedTrackForPlaylist.title}
            />
          )}
        </>
      )}
    </LibraryPageLayout>
  );
}
