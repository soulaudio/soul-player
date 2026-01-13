/**
 * ArtistLink - Reusable component for linking to artist pages
 * Makes artist names clickable throughout the app
 */

import { useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'

export interface ArtistLinkProps {
  /** Artist ID - if missing, link will be disabled */
  artistId?: number
  /** Artist name to display */
  artistName?: string
  /** Additional CSS classes */
  className?: string
  /** Click handler (optional - for custom behavior) */
  onClick?: (e: React.MouseEvent) => void
}

export function ArtistLink({
  artistId,
  artistName,
  className = '',
  onClick,
}: ArtistLinkProps) {
  const navigate = useNavigate()
  const { t } = useTranslation()

  const displayName = artistName || t('common.unknownArtist', 'Unknown Artist')
  const isClickable = !!artistId

  const handleClick = (e: React.MouseEvent) => {
    console.log('[ArtistLink] Click detected', { artistId, artistName })

    e.stopPropagation() // Prevent triggering parent click handlers (e.g., row click)

    if (onClick) {
      onClick(e)
      return
    }

    if (!isClickable) {
      console.log('[ArtistLink] Not clickable - no artistId')
      return
    }

    console.log('[ArtistLink] Navigating to artist:', artistId)
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
      style={{ cursor: 'pointer', userSelect: 'none' }}
    >
      {displayName}
    </span>
  )
}
