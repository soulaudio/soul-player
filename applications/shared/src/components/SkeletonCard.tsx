/**
 * SkeletonCard - Loading skeleton for grid items
 */

interface SkeletonCardProps {
  /** Type of card to render skeleton for */
  type: 'album' | 'artist' | 'playlist' | 'track'
}

export function SkeletonCard({ type }: SkeletonCardProps) {
  const isCircle = type === 'artist'

  return (
    <div className="animate-pulse">
      {/* Image/Avatar skeleton */}
      <div
        className={`w-full aspect-square bg-muted/50 mb-3 ${
          isCircle ? 'rounded-full' : 'rounded-lg'
        }`}
      />

      {/* Title skeleton */}
      <div className="h-4 bg-muted/50 rounded mb-2 w-3/4" />

      {/* Subtitle skeleton */}
      <div className="h-3 bg-muted/50 rounded w-1/2" />
    </div>
  )
}
