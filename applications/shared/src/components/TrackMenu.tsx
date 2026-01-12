import { useTranslation } from 'react-i18next'
import { MoreVertical, Trash, ListPlus, FolderOpen, ListEnd, ArrowUpCircle } from 'lucide-react'
import * as DropdownMenu from '@radix-ui/react-dropdown-menu'
import { useBackend, type BackendTrack } from '../contexts/BackendContext'
import { usePlatform } from '../contexts/PlatformContext'

interface TrackMenuProps {
  track: BackendTrack
  onAddToPlaylist: () => void
  onDelete?: () => void
  onPlayNext?: () => void
  onAddToQueue?: () => void
}

export function TrackMenu({
  track,
  onAddToPlaylist,
  onDelete,
  onPlayNext,
  onAddToQueue,
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
          {/* Play Next */}
          {onPlayNext && (
            <DropdownMenu.Item
              className="relative flex cursor-pointer select-none items-center gap-2 rounded-sm px-4 py-2 text-sm outline-none transition-colors hover:bg-accent focus:bg-accent focus:text-accent-foreground data-[disabled]:pointer-events-none data-[disabled]:opacity-50"
              onSelect={onPlayNext}
            >
              <ArrowUpCircle className="w-4 h-4" />
              <span>{t('queue.playNext', 'Play Next')}</span>
            </DropdownMenu.Item>
          )}

          {/* Add to Queue */}
          {onAddToQueue && (
            <DropdownMenu.Item
              className="relative flex cursor-pointer select-none items-center gap-2 rounded-sm px-4 py-2 text-sm outline-none transition-colors hover:bg-accent focus:bg-accent focus:text-accent-foreground data-[disabled]:pointer-events-none data-[disabled]:opacity-50"
              onSelect={onAddToQueue}
            >
              <ListEnd className="w-4 h-4" />
              <span>{t('queue.addToQueue', 'Add to Queue')}</span>
            </DropdownMenu.Item>
          )}

          {(onPlayNext || onAddToQueue) && <DropdownMenu.Separator className="h-px bg-border my-1" />}

          {/* Add to Playlist */}
          <DropdownMenu.Item
            className="relative flex cursor-pointer select-none items-center gap-2 rounded-sm px-4 py-2 text-sm outline-none transition-colors hover:bg-accent focus:bg-accent focus:text-accent-foreground data-[disabled]:pointer-events-none data-[disabled]:opacity-50"
            onSelect={onAddToPlaylist}
            disabled={!features.canCreatePlaylists}
          >
            <ListPlus className="w-4 h-4" />
            <span>{t('playlist.addToPlaylist', 'Add to Playlist')}</span>
          </DropdownMenu.Item>

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

          {/* Delete from Managed Library (only show for managed library tracks) */}
          {track.is_in_managed_library && onDelete && (
            <>
              <DropdownMenu.Separator className="h-px bg-border my-1" />
              <DropdownMenu.Item
                className="relative flex cursor-pointer select-none items-center gap-2 rounded-sm px-4 py-2 text-sm outline-none transition-colors hover:bg-accent focus:bg-accent focus:text-accent-foreground data-[disabled]:pointer-events-none data-[disabled]:opacity-50 text-red-600"
                onSelect={handleDelete}
              >
                <Trash className="w-4 h-4" />
                <span>{t('trackMenu.deleteFromManagedLibrary', 'Delete from Managed Library')}</span>
              </DropdownMenu.Item>
            </>
          )}
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  )
}
