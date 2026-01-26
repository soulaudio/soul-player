/**
 * Skeleton loading state for TrackList
 * Shows while tracks are loading for better perceived performance
 */

interface SkeletonTrackListProps {
  /** Number of skeleton rows to display (default: 10) */
  rows?: number
}

export function SkeletonTrackList({ rows = 10 }: SkeletonTrackListProps) {
  return (
    <div className="border rounded-lg overflow-hidden">
      {/* Header */}
      <div className="bg-muted/50">
        <div className="grid grid-cols-[40px_minmax(200px,1fr)_minmax(120px,180px)_minmax(120px,180px)_70px_70px_40px] gap-4 px-4 py-2 text-sm font-medium text-muted-foreground">
          <div className="text-center">#</div>
          <div>Title</div>
          <div>Artist</div>
          <div>Album</div>
          <div className="text-center">Format</div>
          <div className="text-right">Time</div>
          <div></div>
        </div>
      </div>

      {/* Skeleton Rows */}
      <div className="divide-y">
        {Array.from({ length: rows }).map((_, i) => (
          <div
            key={i}
            className="grid grid-cols-[40px_minmax(200px,1fr)_minmax(120px,180px)_minmax(120px,180px)_70px_70px_40px] gap-4 px-4 py-3 animate-pulse"
          >
            {/* Track Number */}
            <div className="flex items-center justify-center">
              <div className="h-4 w-6 bg-muted rounded"></div>
            </div>

            {/* Title */}
            <div className="flex items-center gap-2">
              <div className="h-4 bg-muted rounded w-3/4"></div>
            </div>

            {/* Artist */}
            <div className="flex items-center">
              <div className="h-4 bg-muted rounded w-2/3"></div>
            </div>

            {/* Album */}
            <div className="flex items-center">
              <div className="h-4 bg-muted rounded w-1/2"></div>
            </div>

            {/* Format */}
            <div className="flex items-center justify-center">
              <div className="h-5 w-12 bg-muted rounded"></div>
            </div>

            {/* Duration */}
            <div className="flex items-center justify-end">
              <div className="h-4 w-10 bg-muted rounded"></div>
            </div>

            {/* Menu */}
            <div className="flex items-center justify-center">
              <div className="h-6 w-6 bg-muted rounded-full"></div>
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}
