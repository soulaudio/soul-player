import { MoreVertical, Trash } from 'lucide-react';
import * as DropdownMenu from '@radix-ui/react-dropdown-menu';

interface TrackMenuProps {
  trackId: number;
  trackTitle: string;
  onDelete: (trackId: number) => void;
}

export function TrackMenu({ trackId, trackTitle, onDelete }: TrackMenuProps) {
  const handleDelete = () => {
    onDelete(trackId);
  };

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
          <DropdownMenu.Item
            className="relative flex cursor-pointer select-none items-center gap-2 rounded-sm px-4 py-2 text-sm outline-none transition-colors hover:bg-accent focus:bg-accent focus:text-accent-foreground data-[disabled]:pointer-events-none data-[disabled]:opacity-50 text-red-600"
            onSelect={handleDelete}
          >
            <Trash className="w-4 h-4" />
            <span>Remove from Library</span>
          </DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  );
}
