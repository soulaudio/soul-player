import { Play, Music2 } from 'lucide-react';
import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { ArtworkImage } from '@soul-player/shared';

export interface Album {
  id: number;
  title: string;
  artist_name?: string;
  year?: number;
  cover_art_path?: string;
}

interface AlbumGridProps {
  albums: Album[];
  onPlay?: (album: Album) => void;
}

export function AlbumGrid({ albums, onPlay }: AlbumGridProps) {
  const [hoveredAlbumId, setHoveredAlbumId] = useState<number | null>(null);
  const navigate = useNavigate();

  const handleAlbumClick = (album: Album) => {
    navigate(`/albums/${album.id}`);
  };

  const handlePlayClick = (e: React.MouseEvent, album: Album) => {
    e.stopPropagation();
    onPlay?.(album);
  };

  if (albums.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
        <Music2 className="w-12 h-12 mb-4 opacity-50" />
        <p>No albums found</p>
        <p className="text-sm mt-1">Import music to get started</p>
      </div>
    );
  }

  return (
    <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4">
      {albums.map((album) => (
        <div
          key={album.id}
          className="group cursor-pointer"
          onMouseEnter={() => setHoveredAlbumId(album.id)}
          onMouseLeave={() => setHoveredAlbumId(null)}
          onClick={() => handleAlbumClick(album)}
        >
          <div className="relative aspect-square mb-3 bg-muted rounded-lg overflow-hidden shadow-md hover:shadow-xl transition-shadow">
            <ArtworkImage
              albumId={album.id}
              alt={album.title}
              className="w-full h-full object-cover"
              fallbackClassName="w-full h-full flex items-center justify-center"
            />
            {hoveredAlbumId === album.id && (
              <div className="absolute inset-0 bg-black/40 flex items-center justify-center transition-opacity">
                <button
                  onClick={(e) => handlePlayClick(e, album)}
                  className="w-12 h-12 bg-primary rounded-full flex items-center justify-center hover:scale-110 transition-transform shadow-lg"
                  aria-label={`Play ${album.title}`}
                >
                  <Play className="w-6 h-6 text-primary-foreground ml-0.5" fill="currentColor" />
                </button>
              </div>
            )}
          </div>
          <div className="space-y-1">
            <h3 className="font-medium truncate" title={album.title}>
              {album.title}
            </h3>
            <p className="text-sm text-muted-foreground truncate" title={album.artist_name}>
              {album.artist_name || 'Unknown Artist'}
              {album.year && ` • ${album.year}`}
            </p>
          </div>
        </div>
      ))}
    </div>
  );
}
