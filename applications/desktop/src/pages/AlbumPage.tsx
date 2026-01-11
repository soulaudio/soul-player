import { useEffect, useState, useCallback } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { TrackList, type Track, type QueueTrack, ArtworkImage } from '@soul-player/shared';
import { TrackMenu } from '../components/TrackMenu';
import { ArrowLeft, Play, Clock } from 'lucide-react';

interface Album {
  id: number;
  title: string;
  artist_id?: number;
  artist_name?: string;
  year?: number;
  cover_art_path?: string;
}

interface DesktopTrack extends Track {
  artist_id?: number;
  artist_name?: string;
  album_title?: string;
  duration_seconds?: number;
  file_path?: string;
  year?: number;
}

export function AlbumPage() {
  const { t } = useTranslation();
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();

  const [album, setAlbum] = useState<Album | null>(null);
  const [tracks, setTracks] = useState<DesktopTrack[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!id) return;
    loadAlbum(parseInt(id, 10));
  }, [id]);

  const loadAlbum = async (albumId: number) => {
    setLoading(true);
    setError(null);
    try {
      const [albumData, tracksData] = await Promise.all([
        invoke<Album | null>('get_album_by_id', { id: albumId }),
        invoke<DesktopTrack[]>('get_album_tracks', { albumId }),
      ]);

      if (!albumData) {
        setError(t('album.notFound', 'Album not found'));
        return;
      }

      setAlbum(albumData);
      setTracks(tracksData);
    } catch (err) {
      console.error('Failed to load album:', err);
      setError(err instanceof Error ? err.message : 'Failed to load album');
    } finally {
      setLoading(false);
    }
  };

  const buildQueue = useCallback((_allTracks: Track[], clickedTrack: Track, _clickedIndex: number): QueueTrack[] => {
    const validTracks = tracks.filter((t) => t.file_path);
    const validClickedIndex = validTracks.findIndex(t => t.id === clickedTrack.id);
    if (validClickedIndex === -1) return [];

    return [
      ...validTracks.slice(validClickedIndex),
      ...validTracks.slice(0, validClickedIndex),
    ].map((t) => ({
      trackId: String(t.id),
      title: String(t.title || 'Unknown'),
      artist: t.artist_name || 'Unknown Artist',
      album: t.album_title || null,
      filePath: t.file_path!,
      durationSeconds: t.duration_seconds || null,
      trackNumber: t.trackNumber || null,
    }));
  }, [tracks]);

  const handlePlayAll = async () => {
    const validTracks = tracks.filter(t => t.file_path);
    if (validTracks.length === 0) return;

    const queue = validTracks.map((t) => ({
      trackId: String(t.id),
      title: String(t.title || 'Unknown'),
      artist: t.artist_name || 'Unknown Artist',
      album: t.album_title || null,
      filePath: t.file_path!,
      durationSeconds: t.duration_seconds || null,
      trackNumber: t.trackNumber || null,
    }));

    try {
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

  if (error || !album) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center text-destructive">
          <p className="font-medium mb-2">{error || t('album.notFound', 'Album not found')}</p>
          <button
            onClick={() => navigate('/library?tab=albums')}
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
          onClick={() => navigate('/library?tab=albums')}
          className="flex items-center gap-2 text-muted-foreground hover:text-foreground mb-4"
        >
          <ArrowLeft className="w-4 h-4" />
          <span>{t('album.backToAlbums', 'Back to Albums')}</span>
        </button>

        <div className="flex items-start gap-6">
          {/* Album Cover */}
          <div className="w-48 h-48 bg-muted rounded-lg overflow-hidden shadow-lg flex-shrink-0">
            <ArtworkImage
              albumId={album.id}
              alt={album.title}
              className="w-full h-full object-cover"
              fallbackClassName="w-full h-full flex items-center justify-center"
            />
          </div>

          {/* Album Info */}
          <div className="flex-1">
            <p className="text-sm text-muted-foreground uppercase tracking-wider mb-1">
              {t('library.album', 'Album')}
            </p>
            <h1 className="text-4xl font-bold mb-2">{album.title}</h1>
            <p className="text-lg mb-2">
              <button
                onClick={() => {
                  // Navigate to artist if we have artist_id
                  if (album?.artist_id) {
                    navigate(`/artists/${album.artist_id}`);
                  }
                }}
                className="hover:underline"
              >
                {album.artist_name || t('common.unknownArtist', 'Unknown Artist')}
              </button>
              {album.year && <span className="text-muted-foreground"> • {album.year}</span>}
            </p>
            <p className="text-sm text-muted-foreground flex items-center gap-2 mb-4">
              <Clock className="w-4 h-4" />
              {tracks.length} {t('library.tracks', 'tracks')} • {formatDuration(totalDuration)}
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
