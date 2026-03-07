/**
 * Shared PlaylistCard component - wrapper around MediaCard
 * Maintains backward compatibility with existing usage
 */

import { useTranslation } from 'react-i18next'
import { MediaCard } from './MediaCard'
import { type BackendPlaylist } from '../contexts/BackendContext'

interface PlaylistCardProps {
  playlist: BackendPlaylist
  /** Card width class (default: w-full for responsive grid) */
  className?: string
  /** Priority: if true, loads artwork immediately without lazy loading. Use for above-the-fold items (first ~20-30 items) */
  priority?: boolean
  onAddToPlaylist?: () => void
}

export function PlaylistCard({ playlist, className = 'w-full', priority = false, onAddToPlaylist }: PlaylistCardProps) {
  const { t } = useTranslation()

  return (
    <MediaCard
      type="playlist"
      id={playlist.id}
      title={playlist.name}
      subtitle={t('library.tracks', { count: playlist.track_count })}
      coverUrl={playlist.cover_art_path ?? undefined}
      className={className}
      priority={priority}
      onAddToPlaylist={onAddToPlaylist}
    />
  )
}
