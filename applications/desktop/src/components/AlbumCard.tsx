import { useState, useEffect } from 'react';
import { Play, Pause } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { ArtworkImage, getDeduplicatedTracks, usePlayerStore, usePlayerCommands } from '@soul-player/shared';
import { usePlaybackContext } from '../hooks/usePlaybackContext';

export interface Album {
  id: number;
  title: string;
  artist_name?: string;
  year?: number;
  cover_art_path?: string;
}

interface AlbumCardProps {
  album: Album;
  /** Card width class (default: w-40) */
  className?: string;
  /** Show artist and year below title */
  showArtist?: boolean;
}

interface AlbumTrack {
  id: number;
  title: string;
  artist_name?: string;
  album_title?: string;
  file_path?: string;
  duration_seconds?: number;
}

export function AlbumCard({ album, className = 'w-40', showArtist = true }: AlbumCardProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { recordContext, getCurrentContext } = usePlaybackContext();
  const { isPlaying } = usePlayerStore();
  const commands = usePlayerCommands();
  const [isThisAlbumPlaying, setIsThisAlbumPlaying] = useState(false);

  // Check if this album is the current playback context
  useEffect(() => {
    const checkContext = async () => {
      if (!isPlaying) {
        setIsThisAlbumPlaying(false);
        return;
      }

      try {
        const context = await getCurrentContext();
        const isActiveAlbum =
          context?.contextType === 'album' &&
          context?.contextId === String(album.id);
        setIsThisAlbumPlaying(isActiveAlbum);
      } catch {
        setIsThisAlbumPlaying(false);
      }
    };

    checkContext();
  }, [isPlaying, album.id, getCurrentContext]);

  const handleClick = () => {
    console.log('[AlbumCard] handleClick - navigating to album:', album.id);
    navigate(`/albums/${album.id}`);
  };

  const handlePlayPause = async (e: React.MouseEvent) => {
    e.stopPropagation();

    // If this album is currently playing, pause it
    if (isThisAlbumPlaying) {
      console.log('[AlbumCard] handlePlayPause - pausing album:', album.id);
      try {
        await commands.pausePlayback();
      } catch (err) {
        console.error('Failed to pause:', err);
      }
      return;
    }

    // Otherwise, play the album
    console.log('[AlbumCard] handlePlayPause - playing album:', album.id);

    try {
      const tracks = await invoke<AlbumTrack[]>('get_album_tracks', {
        albumId: album.id,
      });
      console.log('[AlbumCard] Got tracks:', tracks.length);

      // Deduplicate tracks (selects best quality version for each unique track)
      const deduplicatedTracks = getDeduplicatedTracks(tracks.filter((t) => t.file_path));
      if (deduplicatedTracks.length === 0) {
        console.log('[AlbumCard] No valid tracks with file_path');
        return;
      }
      console.log('[AlbumCard] Deduplicated to', deduplicatedTracks.length, 'tracks from', tracks.length);

      const queue = deduplicatedTracks.map((t) => ({
        trackId: String(t.id),
        title: t.title || 'Unknown',
        artist: t.artist_name || 'Unknown Artist',
        album: t.album_title || album.title,
        filePath: t.file_path!,
        durationSeconds: t.duration_seconds || null,
        trackNumber: null,
      }));

      // Record playback context
      await recordContext({
        contextType: 'album',
        contextId: String(album.id),
        contextName: album.title,
        contextArtworkPath: album.cover_art_path,
      });

      await invoke('play_queue', { queue, startIndex: 0 });
    } catch (err) {
      console.error('Failed to play album:', err);
    }
  };

  return (
    <div className={`flex-shrink-0 cursor-pointer group ${className}`}>
      <div
        className="aspect-square rounded-lg overflow-hidden bg-muted mb-2 shadow group-hover:shadow-md transition-shadow relative cursor-pointer"
        onClick={handleClick}
        role="button"
        tabIndex={0}
        onKeyDown={(e) => e.key === 'Enter' && handleClick()}
      >
        <ArtworkImage
          albumId={album.id}
          alt={album.title}
          className="w-full h-full object-cover group-hover:scale-105 transition-transform duration-200"
          fallbackClassName="w-full h-full flex items-center justify-center bg-muted"
        />
        {/* Play/Pause button - centered, smaller, visible on hover */}
        <button
          onClick={handlePlayPause}
          className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-14 h-14 flex items-center justify-center bg-black/50 hover:bg-black/70 rounded-xl opacity-0 group-hover:opacity-100 transition-all duration-200"
          aria-label={isThisAlbumPlaying ? t('playback.pause') : t('playback.play')}
        >
          {isThisAlbumPlaying ? (
            <Pause className="w-8 h-8 text-white drop-shadow-lg" fill="currentColor" />
          ) : (
            <Play className="w-8 h-8 text-white drop-shadow-lg" fill="currentColor" />
          )}
        </button>
      </div>
      <p
        className="font-medium truncate group-hover:text-primary transition-colors"
        title={album.title}
        onClick={handleClick}
      >
        {album.title}
      </p>
      {showArtist && (
        <p
          className="text-sm text-muted-foreground truncate"
          title={album.artist_name}
          onClick={handleClick}
        >
          {album.artist_name || 'Unknown Artist'}
          {album.year && ` • ${album.year}`}
        </p>
      )}
    </div>
  );
}
