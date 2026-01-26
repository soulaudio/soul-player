/**
 * SkeletonGrid - Grid of loading skeletons for albums/artists/playlists
 * Shows while data is loading for better perceived performance
 */

import { SkeletonCard } from './SkeletonCard'

interface SkeletonGridProps {
  /** Number of skeleton cards to display (default: 12) */
  count?: number
  /** Type of cards to render skeletons for */
  type: 'album' | 'artist' | 'playlist' | 'track'
  /** Optional custom grid classes (overrides default) */
  gridClass?: string
}

export function SkeletonGrid({ count = 12, type, gridClass }: SkeletonGridProps) {
  const defaultGridClass = 'grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4'

  return (
    <div className={gridClass || defaultGridClass}>
      {Array.from({ length: count }).map((_, i) => (
        <SkeletonCard key={i} type={type} />
      ))}
    </div>
  )
}
