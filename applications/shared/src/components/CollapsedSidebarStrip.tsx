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
      className="w-1.5 h-full bg-foreground/10 hover:bg-foreground/20 transition-opacity cursor-ew-resize flex-shrink-0"
      onMouseDown={handleMouseDown}
      role="separator"
      aria-orientation="vertical"
      aria-label={t('sidebar.expand', 'Expand sidebar')}
      data-testid="collapsed-sidebar-strip"
    />
  );
}
