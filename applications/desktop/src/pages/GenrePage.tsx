import { useEffect, useState, useCallback } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { TrackList, type Track, type QueueTrack, getDeduplicatedTracks } from '@soul-player/shared';
import { TrackMenu } from '../components/TrackMenu';
import { ArrowLeft, Play, Guitar, Clock } from 'lucide-react';
import { usePlaybackContext } from '../hooks/usePlaybackContext';

interface Genre {
  id: number;
  name: string;
  track_count: number;
}

interface DesktopTrack extends Track {
  artist_name?: string;
  album_title?: string;
  duration_seconds?: number;
  file_path?: string;
  year?: number;
  // Audio format info
  file_format?: string;
  bit_rate?: number;
  sample_rate?: number;
  channels?: number;
}

export function GenrePage() {
  const { t } = useTranslation();
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { recordContext } = usePlaybackContext();

  const [genre, setGenre] = useState<Genre | null>(null);
  const [tracks, setTracks] = useState<DesktopTrack[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!id) return;
    loadGenre(parseInt(id, 10));
  }, [id]);

  const loadGenre = async (genreId: number) => {
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
      console.error('Failed to load genre:', err);
      setError(err instanceof Error ? err.message : 'Failed to load genre');
    } finally {
      setLoading(false);
    }
  };

  const buildQueue = useCallback((allTracks: Track[], clickedTrack: Track, _clickedIndex: number): QueueTrack[] => {
    // allTracks is already deduplicated by TrackList's internal grouping
    // We need to map back to DesktopTrack to get file_path
    const trackMap = new Map(tracks.map(t => [String(t.id), t]));

    // Filter to only tracks we have file_path for
    const validTracks = allTracks.filter(t => {
      const desktopTrack = trackMap.get(String(t.id));
      return desktopTrack?.file_path;
    });

    const validClickedIndex = validTracks.findIndex(t => String(t.id) === String(clickedTrack.id));
    if (validClickedIndex === -1) return [];

    // Record playback context when playing from genre
    if (genre) {
      recordContext({
        contextType: 'genre',
        contextId: String(genre.id),
        contextName: genre.name,
        contextArtworkPath: null,
      });
    }

    return [
      ...validTracks.slice(validClickedIndex),
      ...validTracks.slice(0, validClickedIndex),
    ].map((t) => {
      const desktopTrack = trackMap.get(String(t.id))!;
      return {
        trackId: String(t.id),
        title: String(t.title || 'Unknown'),
        artist: desktopTrack.artist_name || 'Unknown Artist',
        album: desktopTrack.album_title || null,
        filePath: desktopTrack.file_path!,
        durationSeconds: desktopTrack.duration_seconds || null,
        trackNumber: desktopTrack.trackNumber || null,
      };
    });
  }, [tracks, genre, recordContext]);

  const handlePlayAll = async () => {
    // Deduplicate tracks (selects best quality version for each unique track)
    const deduplicatedTracks = getDeduplicatedTracks(tracks.filter(t => t.file_path));
    if (deduplicatedTracks.length === 0) return;

    const queue = deduplicatedTracks.map((t) => ({
      trackId: String(t.id),
      title: String(t.title || 'Unknown'),
      artist: t.artist_name || 'Unknown Artist',
      album: t.album_title || null,
      filePath: t.file_path!,
      durationSeconds: t.duration_seconds || null,
      trackNumber: t.trackNumber || null,
    }));

    try {
      // Record playback context
      if (genre) {
        await recordContext({
          contextType: 'genre',
          contextId: String(genre.id),
          contextName: genre.name,
          contextArtworkPath: null,
        });
      }
      await invoke('play_queue', { queue, startIndex: 0 });
    } catch (err) {
      console.error('Failed to play all tracks:', err);
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

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <div className="animate-spin w-8 h-8 border-4 border-primary border-t-transparent rounded-full mx-auto mb-4"></div>
          <p className="text-muted-foreground">{t('common.loading', 'Loading...')}</p>
        </div>
      </div>
    );
  }

  if (error || !genre) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center text-destructive">
          <p className="font-medium mb-2">{error || t('genre.notFound', 'Genre not found')}</p>
          <button
            onClick={() => navigate('/library?tab=genres')}
            className="mt-4 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90"
          >
            {t('common.back', 'Back')}
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="mb-6">
        <button
          onClick={() => navigate('/library?tab=genres')}
          className="flex items-center gap-2 text-muted-foreground hover:text-foreground mb-4"
        >
          <ArrowLeft className="w-4 h-4" />
          <span>{t('genre.backToGenres', 'Back to Genres')}</span>
        </button>

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
            <h1 className="text-4xl font-bold mb-2">{genre.name}</h1>
            <p className="text-sm text-muted-foreground flex items-center gap-2 mb-4">
              <Clock className="w-4 h-4" />
              {genre.track_count} {t('library.tracks', 'tracks')} • {formatDuration(totalDuration)}
            </p>

            <button
              onClick={handlePlayAll}
              disabled={tracks.filter(t => t.file_path).length === 0}
              className="flex items-center gap-2 px-6 py-3 bg-primary text-primary-foreground rounded-full hover:bg-primary/90 disabled:opacity-50"
            >
              <Play className="w-5 h-5" fill="currentColor" />
              <span>{t('common.playAll', 'Play All')}</span>
            </button>
          </div>
        </div>
      </div>

      {/* Track List */}
      <div className="flex-1 overflow-auto">
        <TrackList
          tracks={tracks.map(t => ({
            id: t.id,
            title: String(t.title || 'Unknown'),
            artist: t.artist_name,
            album: t.album_title,
            duration: t.duration_seconds,
            trackNumber: t.trackNumber,
            isAvailable: !!t.file_path,
            format: t.file_format,
            bitrate: t.bit_rate,
            sampleRate: t.sample_rate,
            channels: t.channels,
          }))}
          buildQueue={buildQueue}
          onTrackAction={() => {}}
          renderMenu={(track) => (
            <TrackMenu
              trackId={Number(track.id)}
              trackTitle={track.title}
              onDelete={() => {}}
            />
          )}
        />
      </div>
    </div>
  );
}
