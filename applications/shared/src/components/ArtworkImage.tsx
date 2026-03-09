import { useEffect, useState, useCallback, memo, useRef } from 'react';
import { Music, User, ListMusic } from 'lucide-react';
import { ProgressiveImage } from './ProgressiveImage';
import { debug } from '../utils/debug';

interface ArtworkImageProps {
  trackId?: string | number;
  albumId?: number;
  artistId?: number;
  playlistId?: string;
  coverArtPath?: string; // Direct URL for browser environments (marketing demo)
  alt?: string;
  className?: string;
  fallbackClassName?: string;
  /** Icon to show when no artwork is available (defaults based on entity type) */
  fallbackIcon?: 'music' | 'users' | 'playlist';
  /** Shape: rounded for albums/playlists, circular for artists */
  shape?: 'rounded' | 'circular';
  /** Priority: if true, loads immediately without waiting for viewport. Use for above-the-fold items (first ~20-30 items) */
  priority?: boolean;
  /** When true and artist artwork fails, show blurred album cover + icon overlay instead of plain icon */
  blurredAlbumFallback?: boolean;
}

// Cache for artwork data URLs
const artworkCache = new Map<string, string>();

// Subscribers for cache invalidation notifications
type CacheListener = (key: string) => void;
const cacheListeners = new Set<CacheListener>();

/** Subscribe to cache invalidation events */
function subscribeToCacheInvalidation(listener: CacheListener): () => void {
  cacheListeners.add(listener);
  return () => cacheListeners.delete(listener);
}

/** Notify all listeners of a cache invalidation */
function notifyListeners(key: string): void {
  cacheListeners.forEach(listener => listener(key));
}

/** Clear a specific entry from the artwork cache */
export function clearArtworkCache(type: 'track' | 'album' | 'artist' | 'playlist', id: string | number): void {
  const key = `${type}:${id}`;
  artworkCache.delete(key);
  notifyListeners(key);
}

/** Clear all entries from the artwork cache */
export function clearAllArtworkCache(): void {
  artworkCache.clear();
  notifyListeners('*');
}

export const ArtworkImage = memo(function ArtworkImage({ trackId, albumId, artistId, playlistId, coverArtPath, alt, className, fallbackClassName, fallbackIcon, shape = 'rounded', priority = false, blurredAlbumFallback = false }: ArtworkImageProps) {
  const [artworkUrl, setArtworkUrl] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(false);
  const [refreshCounter, setRefreshCounter] = useState(0);
  const [isInViewport, setIsInViewport] = useState(priority); // Priority items start as visible
  const containerRef = useRef<HTMLDivElement>(null);
  // For artist blurred album fallback
  const [fallbackAlbumUrl, setFallbackAlbumUrl] = useState<string | null>(null);
  const [fallbackAttempted, setFallbackAttempted] = useState(false);

  // Determine the cache key for this component
  const getCacheKey = useCallback((): string | null => {
    if (trackId) return `track:${trackId}`;
    if (albumId) return `album:${albumId}`;
    if (artistId) return `artist:${artistId}`;
    if (playlistId) return `playlist:${playlistId}`;
    return null;
  }, [trackId, albumId, artistId, playlistId]);

  // Reset loading state when any ID prop changes
  useEffect(() => {
    setArtworkUrl(null);
    setLoading(true);
    setError(false);
  }, [trackId, albumId, artistId, playlistId, coverArtPath]);

  // Subscribe to cache invalidation events
  useEffect(() => {
    const cacheKey = getCacheKey();
    if (!cacheKey) return;

    const handleInvalidation = (invalidatedKey: string) => {
      if (invalidatedKey === '*' || invalidatedKey === cacheKey) {
        // Force a refetch by resetting state
        setArtworkUrl(null);
        setLoading(true);
        setError(false);
        setRefreshCounter(c => c + 1);
      }
    };

    const unsubscribe = subscribeToCacheInvalidation(handleInvalidation);
    return unsubscribe;
  }, [getCacheKey]);

  // Lazy loading with IntersectionObserver (only for non-priority items)
  useEffect(() => {
    if (priority || !containerRef.current) return;

    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            setIsInViewport(true);
            // Once in viewport, we can stop observing
            observer.disconnect();
          }
        });
      },
      {
        // Start loading slightly before entering viewport (500px margin)
        rootMargin: '500px',
        threshold: 0.01, // Trigger as soon as 1% is visible
      }
    );

    observer.observe(containerRef.current);

    return () => {
      observer.disconnect();
    };
  }, [priority]);

  useEffect(() => {
    // Only load artwork when in viewport (or priority is set)
    if (!isInViewport) return;

    let cancelled = false;

    async function loadArtwork() {
      // Parse artwork:// protocol URLs to extract IDs for Tauri invoke
      let effectiveTrackId = trackId;
      let effectiveAlbumId = albumId;
      let effectiveArtistId = artistId;
      let effectivePlaylistId = playlistId;

      if (coverArtPath) {
        const artworkMatch = coverArtPath.match(/^artwork:\/\/(track|album|artist|playlist)\/(.+)$/);
        if (artworkMatch) {
          const [, type, id] = artworkMatch;
          // Clear all effective IDs first - coverArtPath takes precedence
          effectiveTrackId = undefined;
          effectiveAlbumId = undefined;
          effectiveArtistId = undefined;
          effectivePlaylistId = undefined;

          if (type === 'track') {
            effectiveTrackId = id;
          } else if (type === 'album') {
            effectiveAlbumId = parseInt(id, 10);
          } else if (type === 'artist') {
            effectiveArtistId = parseInt(id, 10);
          } else if (type === 'playlist') {
            effectivePlaylistId = id;
          }
        } else {
          // Direct URL (data:, https:, etc.) - use as-is
          setArtworkUrl(coverArtPath);
          setLoading(false);
          setError(false);
          return;
        }
      }

      // Determine effective cache key based on what's provided (after parsing coverArtPath)
      let effectiveCacheKey: string | null = null;
      if (effectiveTrackId) effectiveCacheKey = `track:${effectiveTrackId}`;
      else if (effectiveAlbumId) effectiveCacheKey = `album:${effectiveAlbumId}`;
      else if (effectiveArtistId) effectiveCacheKey = `artist:${effectiveArtistId}`;
      else if (effectivePlaylistId) effectiveCacheKey = `playlist:${effectivePlaylistId}`;

      if (!effectiveCacheKey) {
        setLoading(false);
        setError(true);
        return;
      }

      // Check cache first
      if (artworkCache.has(effectiveCacheKey)) {
        setArtworkUrl(artworkCache.get(effectiveCacheKey)!);
        setLoading(false);
        return;
      }

      try {
        // Check if we're in Tauri environment
        if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
          const { invoke } = await import('@tauri-apps/api/core');

          let dataUrl: string | null;
          if (effectiveTrackId) {
            dataUrl = await invoke<string | null>('get_track_artwork', {
              trackId: effectiveTrackId.toString()
            });
          } else if (effectiveAlbumId) {
            dataUrl = await invoke<string | null>('get_album_artwork', { albumId: effectiveAlbumId });
          } else if (effectiveArtistId) {
            dataUrl = await invoke<string | null>('get_artist_artwork', { artistId: effectiveArtistId });
          } else if (effectivePlaylistId) {
            dataUrl = await invoke<string | null>('get_playlist_artwork', { playlistId: effectivePlaylistId });
          } else {
            dataUrl = null;
          }

          if (cancelled) return;

          if (dataUrl) {
            artworkCache.set(effectiveCacheKey, dataUrl);
            setArtworkUrl(dataUrl);
            setError(false);
          } else {
            setError(true);
          }
        } else {
          // Not in Tauri environment
          if (!cancelled) {
            setError(true);
          }
        }
      } catch (err) {
        debug.error('[ArtworkImage] Failed to load artwork:', err);
        if (!cancelled) {
          setError(true);
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    }

    if (loading) {
      loadArtwork();
    }

    return () => {
      cancelled = true;
    };
  }, [trackId, albumId, artistId, playlistId, coverArtPath, loading, refreshCounter, isInViewport, getCacheKey]);

  // Artist blurred album fallback: when artist has no custom artwork, load their first album's artwork
  useEffect(() => {
    if (!blurredAlbumFallback || !artistId || !error) return;
    // Only attempt once per artistId — use ref to avoid re-render loop
    if (fallbackAttempted) return;

    let cancelled = false;
    async function loadFallback() {
      try {
        if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
          const { invoke } = await import('@tauri-apps/api/core');
          const albums = await invoke<Array<{ id: number }>>('get_artist_albums', { artistId });
          debug.log('[ArtworkImage] Artist', artistId, 'albums for fallback:', albums?.length ?? 0);
          if (cancelled || !albums || albums.length === 0) {
            if (!cancelled) setFallbackAttempted(true);
            return;
          }
          for (const album of albums) {
            if (cancelled) return;
            const cacheKey = `album:${album.id}`;
            if (artworkCache.has(cacheKey)) {
              if (!cancelled) {
                setFallbackAlbumUrl(artworkCache.get(cacheKey)!);
                setFallbackAttempted(true);
              }
              return;
            }
            const dataUrl = await invoke<string | null>('get_album_artwork', { albumId: album.id });
            if (cancelled) return;
            if (dataUrl) {
              artworkCache.set(cacheKey, dataUrl);
              if (!cancelled) {
                setFallbackAlbumUrl(dataUrl);
                setFallbackAttempted(true);
              }
              return;
            }
          }
          // None of the albums had artwork
          if (!cancelled) setFallbackAttempted(true);
        }
      } catch (err) {
        debug.warn('[ArtworkImage] Blurred album fallback failed for artist', artistId, err);
        if (!cancelled) setFallbackAttempted(true);
      }
    }
    loadFallback();
    return () => { cancelled = true; };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [blurredAlbumFallback, artistId, error]);

  // Reset fallback state when artist changes
  useEffect(() => {
    setFallbackAlbumUrl(null);
    setFallbackAttempted(false);
  }, [artistId]);

  // Determine which icon to use for fallback
  const getFallbackIcon = () => {
    const iconType = fallbackIcon || (artistId ? 'users' : playlistId ? 'playlist' : 'music');
    switch (iconType) {
      case 'users':
        return <User className="w-12 h-12 text-muted-foreground" />;
      case 'playlist':
        return <ListMusic className="w-12 h-12 text-muted-foreground" />;
      default:
        return <Music className="w-12 h-12 text-muted-foreground" />;
    }
  };

  if (error || (!loading && !artworkUrl)) {
    // Artist blurred album fallback
    if (blurredAlbumFallback && fallbackAlbumUrl) {
      return (
        <div ref={containerRef} className="w-full h-full relative overflow-hidden">
          <img
            src={fallbackAlbumUrl}
            alt={alt || ''}
            className="w-full h-full object-cover blur-sm scale-105 brightness-50"
          />
          <div className="absolute inset-0 flex items-center justify-center">
            <User className="w-12 h-12 text-white/70 drop-shadow-lg" />
          </div>
        </div>
      );
    }
    // Still loading the blurred album fallback — show placeholder instead of plain icon
    if (blurredAlbumFallback && artistId && !fallbackAttempted) {
      return (
        <div ref={containerRef} className={fallbackClassName || 'flex items-center justify-center animate-pulse bg-muted'}>
          <span className="opacity-50">{getFallbackIcon()}</span>
        </div>
      );
    }
    return (
      <div ref={containerRef} className={fallbackClassName || 'flex items-center justify-center'}>
        {getFallbackIcon()}
      </div>
    );
  }

  if (loading) {
    return (
      <div ref={containerRef} className={fallbackClassName || 'flex items-center justify-center animate-pulse bg-muted'}>
        <span className="opacity-50">{getFallbackIcon()}</span>
      </div>
    );
  }

  return (
    <div ref={containerRef} className="w-full h-full">
      <ProgressiveImage
        src={artworkUrl!}
        alt={alt || 'Album artwork'}
        className={className}
        shape={shape}
      />
    </div>
  );
});
