import { Music2 } from 'lucide-react';
import { AlbumCard, type Album } from './AlbumCard';

export type { Album };

interface AlbumGridProps {
  albums: Album[];
}

export function AlbumGrid({ albums }: AlbumGridProps) {
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
        <AlbumCard key={album.id} album={album} className="w-full" />
      ))}
    </div>
  );
}
