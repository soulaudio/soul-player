/**
 * Shared PlaylistCard component - wrapper around MediaCard
 * Maintains backward compatibility with existing usage
 */

import { useTranslation } from 'react-i18next'
import { MediaCard } from './MediaCard'
import { type BackendPlaylist } from '../contexts/BackendContext'

interface PlaylistCardProps {
  playlist: BackendPlaylist
  /** Card width class (default: w-40) */
  className?: string
}

export function PlaylistCard({ playlist, className = 'w-40' }: PlaylistCardProps) {
  const { t } = useTranslation()

  return (
    <MediaCard
      type="playlist"
      id={playlist.id}
      title={playlist.name}
      subtitle={`${playlist.track_count} ${t('library.tracks')}`}
      coverUrl={playlist.cover_art_path ?? undefined}
      className={className}
    />
  )
}
