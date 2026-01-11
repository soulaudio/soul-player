import { useEffect, useState, useCallback } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { TrackList, type Track, type QueueTrack } from '@soul-player/shared';
import { AlbumGrid, type Album } from '../components/AlbumGrid';
import { TrackMenu } from '../components/TrackMenu';
import { ArrowLeft, Users, Play, Disc3, Music } from 'lucide-react';

interface Artist {
  id: number;
  name: string;
  sort_name?: string;
  track_count: number;
  album_count: number;
}

interface DesktopTrack extends Track {
  artist_name?: string;
  album_title?: string;
  duration_seconds?: number;
  file_path?: string;
  year?: number;
}

export function ArtistPage() {
  const { t } = useTranslation();
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();

  const [artist, setArtist] = useState<Artist | null>(null);
  const [albums, setAlbums] = useState<Album[]>([]);
  const [tracks, setTracks] = useState<DesktopTrack[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<'albums' | 'tracks'>('albums');

  useEffect(() => {
    if (!id) return;
    loadArtist(parseInt(id, 10));
  }, [id]);

  const loadArtist = async (artistId: number) => {
    setLoading(true);
    setError(null);
    try {
      const [artistData, albumsData, tracksData] = await Promise.all([
        invoke<Artist | null>('get_artist_by_id', { id: artistId }),
        invoke<Album[]>('get_artist_albums', { artistId }),
        invoke<DesktopTrack[]>('get_artist_tracks', { artistId }),
      ]);

      if (!artistData) {
        setError(t('artist.notFound', 'Artist not found'));
        return;
      }

      setArtist(artistData);
      setAlbums(albumsData);
      setTracks(tracksData);
    } catch (err) {
      console.error('Failed to load artist:', err);
      setError(err instanceof Error ? err.message : 'Failed to load artist');
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

  const handleAlbumPlay = async (album: Album) => {
    navigate(`/albums/${album.id}`);
  };

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

  if (error || !artist) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center text-destructive">
          <p className="font-medium mb-2">{error || t('artist.notFound', 'Artist not found')}</p>
          <button
            onClick={() => navigate('/library?tab=artists')}
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
          onClick={() => navigate('/library?tab=artists')}
          className="flex items-center gap-2 text-muted-foreground hover:text-foreground mb-4"
        >
          <ArrowLeft className="w-4 h-4" />
          <span>{t('artist.backToArtists', 'Back to Artists')}</span>
        </button>

        <div className="flex items-start gap-6">
          {/* Artist Avatar */}
          <div className="w-32 h-32 bg-muted rounded-full flex items-center justify-center flex-shrink-0">
            <Users className="w-16 h-16 text-muted-foreground" />
          </div>

          {/* Artist Info */}
          <div className="flex-1">
            <h1 className="text-4xl font-bold mb-2">{artist.name}</h1>
            <p className="text-muted-foreground mb-4">
              {artist.album_count} {t('library.albums', 'albums')} • {artist.track_count} {t('library.tracks', 'tracks')}
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

      {/* Tab Navigation */}
      <div className="flex items-center gap-1 bg-muted rounded-lg p-1 mb-6 w-fit">
        <button
          onClick={() => setActiveTab('albums')}
          className={`px-4 py-2 rounded-md transition-colors flex items-center gap-2 ${
            activeTab === 'albums' ? 'bg-background shadow-sm' : 'hover:bg-background/50'
          }`}
        >
          <Disc3 className="w-4 h-4" />
          <span className="text-sm font-medium">{t('library.albums', 'Albums')}</span>
        </button>
        <button
          onClick={() => setActiveTab('tracks')}
          className={`px-4 py-2 rounded-md transition-colors flex items-center gap-2 ${
            activeTab === 'tracks' ? 'bg-background shadow-sm' : 'hover:bg-background/50'
          }`}
        >
          <Music className="w-4 h-4" />
          <span className="text-sm font-medium">{t('library.tracks', 'Tracks')}</span>
        </button>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto">
        {activeTab === 'albums' && (
          <AlbumGrid albums={albums} onPlay={handleAlbumPlay} />
        )}

        {activeTab === 'tracks' && (
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
        )}
      </div>
    </div>
  );
}
