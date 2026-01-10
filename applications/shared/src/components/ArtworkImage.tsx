import { useEffect, useState } from 'react';
import { Music } from 'lucide-react';

interface ArtworkImageProps {
  trackId?: string | number;
  albumId?: number;
  coverArtPath?: string; // Direct URL for browser environments (marketing demo)
  alt?: string;
  className?: string;
  fallbackClassName?: string;
}

// Cache for artwork data URLs
const artworkCache = new Map<string, string>();

export function ArtworkImage({ trackId, albumId, coverArtPath, alt, className, fallbackClassName }: ArtworkImageProps) {
  const [artworkUrl, setArtworkUrl] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(false);

  useEffect(() => {
    let cancelled = false;

    async function loadArtwork() {
      // Parse artwork:// protocol URLs to extract IDs for Tauri invoke
      // Format: artwork://track/123 or artwork://album/123
      let effectiveTrackId = trackId;
      let effectiveAlbumId = albumId;

      if (coverArtPath) {
        const artworkMatch = coverArtPath.match(/^artwork:\/\/(track|album)\/(\d+)$/);
        if (artworkMatch) {
          const [, type, id] = artworkMatch;
          if (type === 'track') {
            effectiveTrackId = id;
          } else if (type === 'album') {
            effectiveAlbumId = parseInt(id, 10);
          }
          // Fall through to invoke path below
        } else {
          // Direct URL (data:, https:, etc.) - use as-is
          setArtworkUrl(coverArtPath);
          setLoading(false);
          setError(false);
          return;
        }
      }

      // Determine cache key and fetch function based on what's provided
      const cacheKey = effectiveTrackId ? `track:${effectiveTrackId}` : effectiveAlbumId ? `album:${effectiveAlbumId}` : null;

      if (!cacheKey) {
        setLoading(false);
        setError(true);
        return;
      }

      // Check cache first
      if (artworkCache.has(cacheKey)) {
        setArtworkUrl(artworkCache.get(cacheKey)!);
        setLoading(false);
        return;
      }

      try {
        // Check if we're in Tauri environment
        if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
          // Import Tauri API
          const { invoke } = await import('@tauri-apps/api/core');

          let dataUrl: string | null;
          if (effectiveTrackId) {
            dataUrl = await invoke<string | null>('get_track_artwork', {
              trackId: effectiveTrackId.toString()
            });
          } else if (effectiveAlbumId) {
            dataUrl = await invoke<string | null>('get_album_artwork', { albumId: effectiveAlbumId });
          } else {
            dataUrl = null;
          }

          if (cancelled) return;

          if (dataUrl) {
            artworkCache.set(cacheKey, dataUrl);
            setArtworkUrl(dataUrl);
            setError(false);
          } else {
            setError(true);
          }
        } else {
          // Not in Tauri environment and no coverArtPath provided
          if (!cancelled) {
            setError(true);
          }
        }
      } catch (err) {
        console.error('[ArtworkImage] Failed to load artwork:', err);
        if (!cancelled) {
          setError(true);
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    }

    loadArtwork();

    return () => {
      cancelled = true;
    };
  }, [trackId, albumId, coverArtPath]);

  if (error || (!loading && !artworkUrl)) {
    return (
      <div className={fallbackClassName || 'flex items-center justify-center'}>
        <Music className="w-6 h-6 text-muted-foreground" />
      </div>
    );
  }

  if (loading) {
    return (
      <div className={fallbackClassName || 'flex items-center justify-center animate-pulse bg-muted'}>
        <Music className="w-6 h-6 text-muted-foreground opacity-50" />
      </div>
    );
  }

  return (
    <img
      src={artworkUrl!}
      alt={alt || 'Album artwork'}
      className={className}
      onError={() => setError(true)}
    />
  );
}
