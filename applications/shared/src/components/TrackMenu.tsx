import { useTranslation } from 'react-i18next'
import { MoreVertical, Trash, ListPlus, FolderOpen, FolderMinus } from 'lucide-react'
import * as DropdownMenu from '@radix-ui/react-dropdown-menu'
import { useBackend, type BackendTrack } from '../contexts/BackendContext'
import { usePlatform } from '../contexts/PlatformContext'

interface TrackMenuProps {
  track: BackendTrack
  onAddToPlaylist?: () => void
  onDelete?: () => void
  onRemoveFromManagedLibrary?: () => void
}

export function TrackMenu({
  track,
  onAddToPlaylist,
  onDelete,
  onRemoveFromManagedLibrary,
}: TrackMenuProps) {
  const { t } = useTranslation()
  const backend = useBackend()
  const { isDesktop, features } = usePlatform()

  const handleShowInExplorer = async () => {
    if (!track.file_path) return
    try {
      await backend.showInFileExplorer(track.file_path)
    } catch (error) {
      console.error('Failed to show file in explorer:', error)
    }
  }

  const handleDelete = async () => {
    if (onDelete) {
      onDelete()
    }
  }

  const handleRemoveFromManagedLibrary = () => {
    if (onRemoveFromManagedLibrary) {
      onRemoveFromManagedLibrary()
    }
  }

  // Only show menu on desktop
  if (!isDesktop) {
    return null
  }

  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger asChild>
        <button
          className="w-8 h-8 flex items-center justify-center rounded hover:bg-accent/50 transition-colors opacity-0 group-hover:opacity-100 focus:opacity-100"
          aria-label="Track options"
        >
          <MoreVertical className="w-4 h-4" />
        </button>
      </DropdownMenu.Trigger>

      <DropdownMenu.Portal>
        <DropdownMenu.Content
          className="min-w-[180px] bg-background border rounded-lg shadow-lg py-1 z-50
            data-[state=open]:animate-fade-in data-[state=open]:animate-zoom-in
            data-[side=bottom]:animate-slide-in-from-top
            data-[side=top]:animate-slide-in-from-bottom
            data-[side=left]:animate-slide-in-from-right
            data-[side=right]:animate-slide-in-from-left"
          sideOffset={5}
          align="end"
        >
          {/* Add to Playlist */}
          {onAddToPlaylist && features.canCreatePlaylists && (
            <DropdownMenu.Item
              className="relative flex cursor-pointer select-none items-center gap-2 rounded-sm px-4 py-2 text-sm outline-none transition-colors hover:bg-accent focus:bg-accent focus:text-accent-foreground data-[disabled]:pointer-events-none data-[disabled]:opacity-50"
              onSelect={onAddToPlaylist}
            >
              <ListPlus className="w-4 h-4" />
              <span>{t('playlist.addToPlaylist', 'Add to Playlist')}</span>
            </DropdownMenu.Item>
          )}

          {/* Show in File Explorer */}
          {track.file_path && (
            <DropdownMenu.Item
              className="relative flex cursor-pointer select-none items-center gap-2 rounded-sm px-4 py-2 text-sm outline-none transition-colors hover:bg-accent focus:bg-accent focus:text-accent-foreground data-[disabled]:pointer-events-none data-[disabled]:opacity-50"
              onSelect={handleShowInExplorer}
            >
              <FolderOpen className="w-4 h-4" />
              <span>{t('trackMenu.showInExplorer', 'Show in File Explorer')}</span>
            </DropdownMenu.Item>
          )}

          {/* Separator */}
          <DropdownMenu.Separator className="h-px bg-border my-1" />

          {/* Remove from Managed Library (only if in managed library) */}
          {track.is_in_managed_library && onRemoveFromManagedLibrary && (
            <DropdownMenu.Item
              className="relative flex cursor-pointer select-none items-center gap-2 rounded-sm px-4 py-2 text-sm outline-none transition-colors hover:bg-accent focus:bg-accent focus:text-accent-foreground data-[disabled]:pointer-events-none data-[disabled]:opacity-50 text-orange-600"
              onSelect={handleRemoveFromManagedLibrary}
            >
              <FolderMinus className="w-4 h-4" />
              <span>{t('trackMenu.removeFromManagedLibrary', 'Remove from Managed Library')}</span>
            </DropdownMenu.Item>
          )}

          {/* Delete from Library */}
          {onDelete && (
            <DropdownMenu.Item
              className="relative flex cursor-pointer select-none items-center gap-2 rounded-sm px-4 py-2 text-sm outline-none transition-colors hover:bg-accent focus:bg-accent focus:text-accent-foreground data-[disabled]:pointer-events-none data-[disabled]:opacity-50 text-red-600"
              onSelect={handleDelete}
            >
              <Trash className="w-4 h-4" />
              <span>{t('trackMenu.removeFromLibrary', 'Remove from Library')}</span>
            </DropdownMenu.Item>
          )}
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  )
}
