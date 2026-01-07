import { MoreVertical, Trash } from 'lucide-react';
import { useState, useEffect, useRef } from 'react';

interface TrackMenuProps {
  trackId: number;
  trackTitle: string;
  onDelete: (trackId: number) => void;
}

export function TrackMenu({ trackId, trackTitle, onDelete }: TrackMenuProps) {
  const [isOpen, setIsOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };

    if (isOpen) {
      document.addEventListener('mousedown', handleClickOutside);
      return () => document.removeEventListener('mousedown', handleClickOutside);
    }
  }, [isOpen]);

  const handleDelete = () => {
    setIsOpen(false);
    onDelete(trackId);
  };

  return (
    <div className="relative" ref={menuRef}>
      <button
        onClick={() => setIsOpen(!isOpen)}
        className="w-8 h-8 flex items-center justify-center rounded hover:bg-accent/50 transition-colors opacity-0 group-hover:opacity-100"
        aria-label="Track options"
      >
        <MoreVertical className="w-4 h-4" />
      </button>

      {isOpen && (
        <div className="absolute right-0 top-full mt-1 bg-background border rounded-lg shadow-lg py-1 z-10 min-w-[180px]">
          <button
            onClick={handleDelete}
            className="w-full px-4 py-2 text-left flex items-center gap-2 hover:bg-accent transition-colors text-red-600"
          >
            <Trash className="w-4 h-4" />
            <span>Remove from Library</span>
          </button>
        </div>
      )}
    </div>
  );
}
