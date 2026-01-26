/**
 * AlbumLink - Reusable component for linking to album pages
 * Makes album names clickable throughout the app
 */

import { useTranslation } from 'react-i18next'
import { useNavigateWithHistory } from '../hooks/useNavigateWithHistory'
import { debug } from '../utils/debug';

export interface AlbumLinkProps {
  /** Album ID - if missing, link will be disabled */
  albumId?: number
  /** Album name to display */
  albumName?: string
  /** Additional CSS classes */
  className?: string
  /** Click handler (optional - for custom behavior) */
  onClick?: (e: React.MouseEvent) => void
}

export function AlbumLink({
  albumId,
  albumName,
  className = '',
  onClick,
}: AlbumLinkProps) {
  const { navigate } = useNavigateWithHistory()
  const { t } = useTranslation()

  const displayName = albumName || t('common.unknownAlbum', 'Unknown Album')
  const isClickable = !!albumId

  const handleClick = (e: React.MouseEvent) => {
    debug.log('[AlbumLink] Click detected', { albumId, albumName })

    e.stopPropagation() // Prevent triggering parent click handlers (e.g., row click)

    if (onClick) {
      onClick(e)
      return
    }

    if (!isClickable) {
      debug.log('[AlbumLink] Not clickable - no albumId')
      return
    }

    debug.log('[AlbumLink] Navigating to album:', albumId)
    navigate(`/albums/${albumId}`)
  }

  if (!isClickable) {
    // Non-clickable text
    return <span className={className}>{displayName}</span>
  }

  // Clickable span styled as text (using span instead of button for consistency)
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
