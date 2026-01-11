import { useEffect, useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { ArrowLeft, Play, ListMusic, Clock, Trash2, MoreHorizontal } from 'lucide-react';
import { ConfirmDialog } from '../components/ConfirmDialog';

interface Playlist {
  id: string;
  name: string;
  description?: string;
  owner_id: number;
  is_public: boolean;
  is_favorite: boolean;
  track_count: number;
  created_at: string;
  updated_at: string;
}

interface PlaylistTrack {
  track_id: string;
  position: number;
  title?: string;
  artist_name?: string;
  duration_seconds?: number;
}

export function PlaylistPage() {
  const { t } = useTranslation();
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();

  const [playlist, setPlaylist] = useState<Playlist | null>(null);
  const [tracks, setTracks] = useState<PlaylistTrack[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [deleteConfirm, setDeleteConfirm] = useState<{ type: 'playlist' | 'track'; trackId?: string } | null>(null);

  useEffect(() => {
    if (!id) return;
    loadPlaylist(id);
  }, [id]);

  const loadPlaylist = async (playlistId: string) => {
    setLoading(true);
    setError(null);
    try {
      const [playlistData, tracksData] = await Promise.all([
        invoke<Playlist | null>('get_playlist_by_id', { id: playlistId }),
        invoke<PlaylistTrack[]>('get_playlist_tracks', { id: playlistId }),
      ]);

      if (!playlistData) {
        setError(t('playlist.notFound', 'Playlist not found'));
        return;
      }

      setPlaylist(playlistData);
      setTracks(tracksData);
    } catch (err) {
      console.error('Failed to load playlist:', err);
      setError(err instanceof Error ? err.message : 'Failed to load playlist');
    } finally {
      setLoading(false);
    }
  };

  const handlePlayAll = async () => {
    if (tracks.length === 0) return;

    // Load full track info for each track
    const queue = await Promise.all(
      tracks.map(async (t) => {
        try {
          const track = await invoke<{ file_path?: string } | null>('get_track_by_id', { id: parseInt(t.track_id, 10) });
          return {
            trackId: t.track_id,
            title: t.title || 'Unknown',
            artist: t.artist_name || 'Unknown Artist',
            album: null,
            filePath: track?.file_path || '',
            durationSeconds: t.duration_seconds || null,
            trackNumber: null,
          };
        } catch {
          return null;
        }
      })
    );

    const validQueue = queue.filter((t): t is NonNullable<typeof t> => t !== null && !!t.filePath);

    if (validQueue.length === 0) return;

    try {
      await invoke('play_queue', { queue: validQueue, startIndex: 0 });
    } catch (err) {
      console.error('Failed to play playlist:', err);
    }
  };

  const handleRemoveTrack = async (trackId: string) => {
    if (!playlist) return;

    try {
      await invoke('remove_track_from_playlist', {
        playlistId: playlist.id,
        trackId,
      });
      // Reload playlist
      await loadPlaylist(playlist.id);
    } catch (err) {
      console.error('Failed to remove track:', err);
    }
    setDeleteConfirm(null);
  };

  const handleDeletePlaylist = async () => {
    if (!playlist) return;

    try {
      await invoke('delete_playlist', { id: playlist.id });
      navigate('/library?tab=playlists');
    } catch (err) {
      console.error('Failed to delete playlist:', err);
    }
    setDeleteConfirm(null);
  };

  const formatDuration = (seconds: number): string => {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    if (hours > 0) {
      return `${hours}h ${minutes}m`;
    }
    return `${minutes} min`;
  };

  const formatTrackDuration = (seconds?: number): string => {
    if (!seconds) return '--:--';
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
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

  if (error || !playlist) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center text-destructive">
          <p className="font-medium mb-2">{error || t('playlist.notFound', 'Playlist not found')}</p>
          <button
            onClick={() => navigate('/library?tab=playlists')}
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
          onClick={() => navigate('/library?tab=playlists')}
          className="flex items-center gap-2 text-muted-foreground hover:text-foreground mb-4"
        >
          <ArrowLeft className="w-4 h-4" />
          <span>{t('playlist.backToPlaylists', 'Back to Playlists')}</span>
        </button>

        <div className="flex items-start gap-6">
          {/* Playlist Icon */}
          <div className="w-48 h-48 bg-gradient-to-br from-primary/30 to-primary/5 rounded-lg flex items-center justify-center flex-shrink-0">
            <ListMusic className="w-24 h-24 text-primary" />
          </div>

          {/* Playlist Info */}
          <div className="flex-1">
            <p className="text-sm text-muted-foreground uppercase tracking-wider mb-1">
              {t('library.playlist', 'Playlist')}
            </p>
            <h1 className="text-4xl font-bold mb-2">{playlist.name}</h1>
            {playlist.description && (
              <p className="text-muted-foreground mb-2">{playlist.description}</p>
            )}
            <p className="text-sm text-muted-foreground flex items-center gap-2 mb-4">
              <Clock className="w-4 h-4" />
              {tracks.length} {t('library.tracks', 'tracks')} • {formatDuration(totalDuration)}
            </p>

            <div className="flex items-center gap-3">
              <button
                onClick={handlePlayAll}
                disabled={tracks.length === 0}
                className="flex items-center gap-2 px-6 py-3 bg-primary text-primary-foreground rounded-full hover:bg-primary/90 disabled:opacity-50"
              >
                <Play className="w-5 h-5" fill="currentColor" />
                <span>{t('common.playAll', 'Play All')}</span>
              </button>

              <button
                onClick={() => setDeleteConfirm({ type: 'playlist' })}
                className="p-3 rounded-full hover:bg-destructive/10 text-destructive"
                title={t('playlist.delete', 'Delete Playlist')}
              >
                <Trash2 className="w-5 h-5" />
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* Track List */}
      <div className="flex-1 overflow-auto">
        {tracks.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
            <ListMusic className="w-12 h-12 mb-4 opacity-50" />
            <p className="font-medium">{t('playlist.empty', 'This playlist is empty')}</p>
            <p className="text-sm mt-1">{t('playlist.emptyHint', 'Add tracks from your library')}</p>
          </div>
        ) : (
          <table className="w-full">
            <thead>
              <tr className="text-left text-sm text-muted-foreground border-b">
                <th className="pb-2 w-12">#</th>
                <th className="pb-2">{t('common.title', 'Title')}</th>
                <th className="pb-2">{t('common.artist', 'Artist')}</th>
                <th className="pb-2 w-20 text-right">{t('common.duration', 'Duration')}</th>
                <th className="pb-2 w-12"></th>
              </tr>
            </thead>
            <tbody>
              {tracks.map((track, index) => (
                <tr
                  key={track.track_id}
                  className="group hover:bg-muted/50 cursor-pointer"
                >
                  <td className="py-3 text-muted-foreground">{index + 1}</td>
                  <td className="py-3 font-medium">{track.title || 'Unknown'}</td>
                  <td className="py-3 text-muted-foreground">{track.artist_name || 'Unknown Artist'}</td>
                  <td className="py-3 text-right text-muted-foreground">
                    {formatTrackDuration(track.duration_seconds)}
                  </td>
                  <td className="py-3">
                    <div className="relative">
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          setDeleteConfirm({ type: 'track', trackId: track.track_id });
                        }}
                        className="p-1 rounded hover:bg-muted opacity-0 group-hover:opacity-100 transition-opacity"
                      >
                        <MoreHorizontal className="w-4 h-4" />
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      {/* Delete confirmation dialogs */}
      <ConfirmDialog
        open={deleteConfirm?.type === 'playlist'}
        title={t('playlist.deleteConfirmTitle', 'Delete Playlist')}
        message={t('playlist.deleteConfirmMessage', `Are you sure you want to delete "${playlist.name}"? This cannot be undone.`)}
        confirmText={t('common.delete', 'Delete')}
        confirmVariant="danger"
        onConfirm={handleDeletePlaylist}
        onClose={() => setDeleteConfirm(null)}
      />

      <ConfirmDialog
        open={deleteConfirm?.type === 'track'}
        title={t('playlist.removeTrackTitle', 'Remove Track')}
        message={t('playlist.removeTrackMessage', 'Remove this track from the playlist?')}
        confirmText={t('common.remove', 'Remove')}
        confirmVariant="danger"
        onConfirm={() => deleteConfirm?.trackId && handleRemoveTrack(deleteConfirm.trackId)}
        onClose={() => setDeleteConfirm(null)}
      />
    </div>
  );
}
