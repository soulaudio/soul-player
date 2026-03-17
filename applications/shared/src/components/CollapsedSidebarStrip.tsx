import { useTranslation } from 'react-i18next';

interface CollapsedSidebarStripProps {
  onExpand: () => void;
  onStartResizeDrag: () => void;
}

/**
 * Thin vertical strip shown when the sidebar is fully collapsed.
 * - Click: instantly restores sidebar to saved width.
 * - Drag right: live resize — sidebar follows cursor until mouse release.
 */
export function CollapsedSidebarStrip({ onExpand, onStartResizeDrag }: CollapsedSidebarStripProps) {
  const { t } = useTranslation();

  const handleMouseDown = (e: React.MouseEvent) => {
    e.preventDefault();
    onStartResizeDrag();
  };

  return (
    <div
      className="group w-3 h-full flex-shrink-0 flex items-center justify-center cursor-ew-resize"
      onMouseDown={handleMouseDown}
      onClick={onExpand}
      role="button"
      aria-label={t('sidebar.expand', 'Expand sidebar')}
      data-testid="collapsed-sidebar-strip"
    >
      {/* Small centered pill — not full height */}
      <div className="w-1 h-14 rounded-full bg-foreground/15 group-hover:bg-foreground/35 transition-colors" />
    </div>
  );
}
