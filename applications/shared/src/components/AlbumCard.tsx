/**
 * Shared AlbumCard component - wrapper around MediaCard
 * Maintains backward compatibility with existing usage
 */

import { MediaCard } from './MediaCard'

export interface AlbumCardAlbum {
  id: number
  title: string
  artist_name?: string
  year?: number
  cover_art_path?: string
  coverUrl?: string
}

interface AlbumCardProps {
  album: AlbumCardAlbum
  /** Card width class (default: w-40) */
  className?: string
  /** Show artist and year below title */
  showArtist?: boolean
}

export function AlbumCard({ album, className = 'w-40', showArtist = true }: AlbumCardProps) {
  return (
    <MediaCard
      type="album"
      id={album.id}
      title={album.title}
      subtitle={showArtist ? album.artist_name : undefined}
      additionalInfo={showArtist && album.year ? String(album.year) : undefined}
      coverUrl={album.coverUrl || album.cover_art_path}
      className={className}
    />
  )
}
