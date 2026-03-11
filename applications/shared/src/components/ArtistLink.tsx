/**
 * ArtistLink - Reusable component for linking to artist pages
 * Makes artist names clickable throughout the app
 */

import { useTranslation } from 'react-i18next'
import { useNavigateWithHistory } from '../hooks/useNavigateWithHistory'
import { debug } from '../utils/debug';

export interface ArtistLinkProps {
  /** Artist ID - if missing, link will be disabled */
  artistId?: number
  /** Artist name to display */
  artistName?: string
  /** Additional CSS classes */
  className?: string
  /** Click handler (optional - for custom behavior) */
  onClick?: (e: React.MouseEvent) => void
  /** Inline styles */
  style?: React.CSSProperties
}

export function ArtistLink({
  artistId,
  artistName,
  className = '',
  onClick,
  style,
}: ArtistLinkProps) {
  const { navigate } = useNavigateWithHistory()
  const { t } = useTranslation()

  const displayName = artistName || t('common.unknownArtist', 'Unknown Artist')
  const isClickable = !!artistId

  const handleClick = (e: React.MouseEvent) => {
    debug.log('[ArtistLink] handleClick called!', { artistId, artistName, isClickable })

    if (onClick) {
      debug.log('[ArtistLink] Custom onClick provided')
      onClick(e)
      return
    }

    if (!isClickable) {
      debug.log('[ArtistLink] Not clickable - no artistId')
      return
    }

    debug.log('[ArtistLink] Navigating to /artists/' + artistId)
    navigate(`/artists/${artistId}`)
  }

  if (!isClickable) {
    // Non-clickable text
    return <span className={className}>{displayName}</span>
  }

  // Clickable span styled as text (using span instead of button for better event handling)
  return (
    <span
      onClick={handleClick}
      className={`hover:underline cursor-pointer transition-colors inline-block ${className}`}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          handleClick(e as any)
        }
      }}
      style={{ cursor: 'pointer', userSelect: 'none', ...style }}
    >
      {displayName}
    </span>
  )
}

interface ArtistLinksProps {
  /** Preferred: full artist list from Track.artists */
  artists?: Array<{ id: number; name: string }>
  /** Fallback: single artist (backward compat when artists[] not available) */
  artistId?: number
  artistName?: string
  /** Additional CSS classes */
  className?: string
}

/**
 * ArtistLinks - Renders one or more artist names as clickable links.
 *
 * Uses `artists` array when available (from track_artists junction).
 * Falls back to single `artistId`/`artistName` for backward compatibility.
 */
export function ArtistLinks({
  artists,
  artistId,
  artistName,
  className = '',
}: ArtistLinksProps) {
  const { t } = useTranslation()

  // Prefer junction-sourced list; fall back to single artist
  const list: Array<{ id?: number; name: string }> =
    artists && artists.length > 0
      ? artists
      : artistId || artistName
        ? [{ id: artistId, name: artistName ?? '' }]
        : []

  if (list.length === 0) {
    return <span className={className}>{t('common.unknownArtist', 'Unknown Artist')}</span>
  }

  return (
    <span className={className}>
      {list.map((a, i) => (
        <span key={a.id ?? a.name}>
          <ArtistLink artistId={a.id} artistName={a.name} />
          {i < list.length - 1 && (
            <span className="text-muted-foreground">, </span>
          )}
        </span>
      ))}
    </span>
  )
}
