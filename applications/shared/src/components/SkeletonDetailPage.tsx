/**
 * SkeletonDetailPage - Loading skeleton for album/artist detail pages
 * Shows while page data is loading for better perceived performance
 */

import { SkeletonTrackList } from './SkeletonTrackList'

interface SkeletonDetailPageProps {
  /** Type of detail page */
  type: 'album' | 'artist' | 'playlist'
  /** Whether to show track list skeleton (default: true) */
  showTracks?: boolean
  /** Number of track rows to show in skeleton (default: 10) */
  trackRows?: number
}

export function SkeletonDetailPage({
  type,
  showTracks = true,
  trackRows = 10,
}: SkeletonDetailPageProps) {
  const isArtist = type === 'artist'

  return (
    <div className="h-full flex flex-col animate-pulse">
      {/* Header Section */}
      <div className="mb-6">
        {/* Back button skeleton */}
        <div className="flex items-center gap-2 mb-4">
          <div className="h-4 w-4 bg-muted rounded"></div>
          <div className="h-4 w-20 bg-muted rounded"></div>
        </div>

        <div className="flex items-start gap-6">
          {/* Cover/Avatar skeleton */}
          <div
            className={`w-48 h-48 bg-muted flex-shrink-0 ${
              isArtist ? 'rounded-full' : 'rounded-lg'
            }`}
          />

          {/* Info skeleton */}
          <div className="flex-1 space-y-4">
            {/* Type label */}
            <div className="h-3 w-16 bg-muted rounded"></div>

            {/* Title */}
            <div className="h-8 w-2/3 bg-muted rounded"></div>

            {/* Description/Metadata */}
            <div className="space-y-2">
              <div className="h-4 w-3/4 bg-muted rounded"></div>
              <div className="h-4 w-1/2 bg-muted rounded"></div>
            </div>

            {/* Action buttons */}
            <div className="flex items-center gap-3">
              <div className="h-12 w-32 bg-muted rounded-full"></div>
              <div className="h-12 w-12 bg-muted rounded-full"></div>
            </div>
          </div>
        </div>
      </div>

      {/* Track List Section */}
      {showTracks && (
        <div className="flex-1 overflow-auto">
          <SkeletonTrackList rows={trackRows} />
        </div>
      )}

      {/* Artist page: Discography section */}
      {isArtist && (
        <div className="mt-6">
          <div className="h-6 w-32 bg-muted rounded mb-4"></div>
          <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 gap-4">
            {Array.from({ length: 6 }).map((_, i) => (
              <div key={i} className="space-y-2">
                <div className="w-full aspect-square bg-muted rounded-lg"></div>
                <div className="h-4 bg-muted rounded w-3/4"></div>
                <div className="h-3 bg-muted rounded w-1/2"></div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  )
}
