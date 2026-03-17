import { useCallback, useRef } from 'react';
import { useTranslation } from 'react-i18next';

const RESTORE_THRESHOLD = 40; // px delta rightward — triggers expand

interface CollapsedSidebarStripProps {
  onExpand: () => void;
}

/**
 * Thin vertical strip shown when the sidebar is fully collapsed.
 * Drag rightward past 40px to restore the sidebar.
 */
export function CollapsedSidebarStrip({ onExpand }: CollapsedSidebarStripProps) {
  const { t } = useTranslation();
  const startXRef = useRef<number | null>(null);

  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      startXRef.current = e.clientX;

      const cleanup = () => {
        startXRef.current = null;
        document.removeEventListener('mousemove', onMouseMove);
        document.removeEventListener('mouseup', onMouseUp);
      };

      const onMouseMove = (ev: MouseEvent) => {
        if (startXRef.current === null) return;
        const delta = ev.clientX - startXRef.current;
        if (delta > RESTORE_THRESHOLD) {
          cleanup(); // one-shot — does not re-collapse on drag-back
          onExpand();
        }
      };

      const onMouseUp = () => cleanup();

      document.addEventListener('mousemove', onMouseMove);
      document.addEventListener('mouseup', onMouseUp);
    },
    [onExpand]
  );

  return (
    <div
      className="group w-3 h-full flex-shrink-0 flex items-center justify-center cursor-ew-resize"
      onMouseDown={handleMouseDown}
      role="separator"
      aria-orientation="vertical"
      aria-label={t('sidebar.expand', 'Expand sidebar')}
      data-testid="collapsed-sidebar-strip"
    >
      {/* Small centered pill — not full height */}
      <div className="w-1 h-14 rounded-full bg-foreground/15 group-hover:bg-foreground/35 transition-colors" />
    </div>
  );
}
