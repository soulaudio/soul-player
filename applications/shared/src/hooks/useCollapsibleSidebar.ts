import { useState, useEffect, useCallback, useRef } from 'react';
import { debug } from '../utils/debug';

// ── Storage keys (share the width key with useResizableSidebar) ────────────
const STORAGE_WIDTH    = 'soul-player:sidebar-width';
const STORAGE_SAVED    = 'soul-player:sidebar-saved-width';
const STORAGE_COLLAPSED = 'soul-player:sidebar-collapsed';

const DEFAULT_WIDTH      = 288;
const MIN_WIDTH          = 240;
const MAX_WIDTH          = 480;
const COLLAPSE_THRESHOLD = 200; // px — snap shut when drag goes below this

function tryReadInt(key: string, fallback: number): number {
  try {
    const raw = localStorage.getItem(key);
    if (raw) { const n = parseInt(raw, 10); if (!isNaN(n)) return n; }
  } catch { /* storage unavailable */ }
  return fallback;
}

function tryWrite(key: string, value: string): void {
  try { localStorage.setItem(key, value); } catch { /* ignore */ }
}

export interface CollapsibleSidebarState {
  width: number;
  isCollapsed: boolean;
  isResizing: boolean;
  savedWidth: number;
  handleMouseDown: (e: React.MouseEvent) => void;
  expand: (width?: number) => void;
  resizableRef: React.RefObject<HTMLDivElement>;
}

export function useCollapsibleSidebar(): CollapsibleSidebarState {
  const resizableRef = useRef<HTMLDivElement>(null);

  const [isCollapsed, setIsCollapsed] = useState(() => {
    try { return localStorage.getItem(STORAGE_COLLAPSED) === 'true'; } catch { return false; }
  });

  const [savedWidth, setSavedWidth] = useState(() =>
    tryReadInt(STORAGE_SAVED, DEFAULT_WIDTH)
  );

  const [width, setWidth] = useState(() => {
    try { if (localStorage.getItem(STORAGE_COLLAPSED) === 'true') return 0; } catch {}
    return tryReadInt(STORAGE_WIDTH, DEFAULT_WIDTH);
  });

  const [isResizing, setIsResizing] = useState(false);

  // Refs for stable access inside event listeners (avoid stale closures)
  const widthRef      = useRef(width);
  const savedWidthRef = useRef(savedWidth);
  const isCollapsedRef = useRef(isCollapsed);
  widthRef.current      = width;
  savedWidthRef.current = savedWidth;
  isCollapsedRef.current = isCollapsed;

  const expand = useCallback((targetWidth?: number) => {
    const w = targetWidth ?? savedWidthRef.current;
    setWidth(w);
    setIsCollapsed(false);
    tryWrite(STORAGE_COLLAPSED, 'false');
    tryWrite(STORAGE_WIDTH, String(w));
  }, []);

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsResizing(true);
  }, []);

  useEffect(() => {
    if (!isResizing) return;

    const onMouseMove = (e: MouseEvent) => {
      // getBoundingClientRect().left is 0 for the sidebar (always at left edge)
      // but we use it correctly in case layout ever adds left inset
      const sidebarLeft = resizableRef.current?.getBoundingClientRect().left ?? 0;
      const newWidth = e.clientX - sidebarLeft;

      if (newWidth < COLLAPSE_THRESHOLD) {
        // Snap to collapsed — save current width first
        const sw = widthRef.current > 0 ? widthRef.current : savedWidthRef.current;
        setSavedWidth(sw);
        tryWrite(STORAGE_SAVED, String(sw));
        tryWrite(STORAGE_COLLAPSED, 'true');
        setWidth(0);
        setIsCollapsed(true);
        setIsResizing(false); // triggers useEffect cleanup, removes listeners
      } else {
        setWidth(Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, newWidth)));
      }
    };

    const onMouseUp = () => {
      if (!isCollapsedRef.current) {
        tryWrite(STORAGE_WIDTH, String(widthRef.current));
      }
      setIsResizing(false);
    };

    document.addEventListener('mousemove', onMouseMove);
    document.addEventListener('mouseup', onMouseUp);
    document.body.style.cursor = 'ew-resize';
    document.body.style.userSelect = 'none';

    return () => {
      document.removeEventListener('mousemove', onMouseMove);
      document.removeEventListener('mouseup', onMouseUp);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    };
  }, [isResizing]);

  debug.log('useCollapsibleSidebar', { width, isCollapsed, isResizing, savedWidth });

  return { width, isCollapsed, isResizing, savedWidth, handleMouseDown, expand, resizableRef };
}
