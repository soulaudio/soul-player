import { useRef, useEffect, useState } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { Music2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { AlbumCard, type Album } from './AlbumCard';

export type { Album };

interface AlbumGridProps {
  albums: Album[];
  /** Grid scale factor - affects number of columns */
  scale?: number;
}

/**
 * Calculate responsive column count based on container width and scale
 */
function getColumnCount(containerWidth: number, _scale: number): number {
  // Base column counts for different breakpoints (scale = 1)
  if (containerWidth < 640) return 2;       // sm
  if (containerWidth < 768) return 3;       // md
  if (containerWidth < 1024) return 4;      // lg
  if (containerWidth < 1280) return 5;      // xl
  return 6;                                 // 2xl+
}

/**
 * AlbumGrid with virtualization using @tanstack/react-virtual
 *
 * Only renders visible rows to improve performance with large libraries.
 * Automatically handles responsive column counts and window resizing.
 */
export function AlbumGrid({ albums, scale = 1 }: AlbumGridProps) {
  const { t } = useTranslation();
  const parentRef = useRef<HTMLDivElement>(null);
  const [containerWidth, setContainerWidth] = useState(0);
  const [columnCount, setColumnCount] = useState(4);

  // Measure container width and update column count
  useEffect(() => {
    if (!parentRef.current) return;

    const updateDimensions = () => {
      if (!parentRef.current) return;
      const width = parentRef.current.offsetWidth;
      setContainerWidth(width);

      let cols = getColumnCount(width, scale);

      // Adjust columns based on scale
      if (scale === 0.75) {
        cols = Math.min(cols + 2, 8); // Smaller cards = more columns
      } else if (scale === 1.25) {
        cols = Math.max(cols - 1, 2); // Larger cards = fewer columns
      } else if (scale === 1.5) {
        cols = Math.max(cols - 2, 1); // Even larger = even fewer
      }

      setColumnCount(cols);
    };

    // Initial measurement
    updateDimensions();

    // Update on resize
    const resizeObserver = new ResizeObserver(updateDimensions);
    resizeObserver.observe(parentRef.current);

    return () => resizeObserver.disconnect();
  }, [scale]);

  // Calculate row count (albums divided by columns)
  const rowCount = Math.ceil(albums.length / columnCount);

  // Card dimensions
  const cardWidth = containerWidth / columnCount;
  const cardHeight = cardWidth + 80; // Aspect ratio + text height

  // Create virtualizer for rows
  const rowVirtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => parentRef.current,
    estimateSize: () => cardHeight,
    overscan: 2, // Render 2 extra rows above/below viewport
  });

  if (albums.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
        <Music2 className="w-12 h-12 mb-4 opacity-50" />
        <p>{t('library.noAlbums')}</p>
        <p className="text-sm mt-1">{t('library.noAlbumsHint')}</p>
      </div>
    );
  }

  return (
    <div
      ref={parentRef}
      className="h-full overflow-auto"
      style={{ contain: 'strict' }}
    >
      <div
        style={{
          height: `${rowVirtualizer.getTotalSize()}px`,
          width: '100%',
          position: 'relative',
        }}
      >
        {rowVirtualizer.getVirtualItems().map((virtualRow) => {
          const startIndex = virtualRow.index * columnCount;
          const endIndex = Math.min(startIndex + columnCount, albums.length);
          const rowAlbums = albums.slice(startIndex, endIndex);

          return (
            <div
              key={virtualRow.key}
              style={{
                position: 'absolute',
                top: 0,
                left: 0,
                width: '100%',
                height: `${virtualRow.size}px`,
                transform: `translateY(${virtualRow.start}px)`,
              }}
            >
              <div
                className="grid gap-4"
                style={{
                  gridTemplateColumns: `repeat(${columnCount}, 1fr)`,
                  height: '100%',
                }}
              >
                {rowAlbums.map((album) => (
                  <AlbumCard
                    key={album.id}
                    album={album}
                    className="w-full"
                  />
                ))}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
