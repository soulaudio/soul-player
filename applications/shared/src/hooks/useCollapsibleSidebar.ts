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

const MOBILE_BREAKPOINT = 640; // px — matches Tailwind `sm`

export interface CollapsibleSidebarState {
  width: number;
  isCollapsed: boolean;
  isResizing: boolean;
  savedWidth: number;
  isMobile: boolean;
  mobileShowContent: boolean;
  setMobileShowContent: (show: boolean) => void;
  handleMouseDown: (e: React.MouseEvent) => void;
  expand: (width?: number) => void;
  startResizeFromCollapsed: () => void;
  resizableRef: React.RefObject<HTMLDivElement>;
}

export function useCollapsibleSidebar(): CollapsibleSidebarState {
  const resizableRef = useRef<HTMLDivElement>(null);

  // ── Mobile detection ──────────────────────────────────────────────────────
  const [isMobile, setIsMobile] = useState(() =>
    typeof window !== 'undefined' ? window.innerWidth < MOBILE_BREAKPOINT : false
  );
  const [mobileShowContent, setMobileShowContent] = useState(false);

  useEffect(() => {
    const mql = window.matchMedia(`(max-width: ${MOBILE_BREAKPOINT - 1}px)`);
    const onChange = (e: MediaQueryListEvent) => {
      setIsMobile(e.matches);
      if (!e.matches) setMobileShowContent(false); // leaving mobile → reset
    };
    mql.addEventListener('change', onChange);
    return () => mql.removeEventListener('change', onChange);
  }, []);

  const [isCollapsed, setIsCollapsed] = useState(() => {
    try { return localStorage.getItem(STORAGE_COLLAPSED) === 'true'; } catch { return false; }
  });

  const [savedWidth, setSavedWidth] = useState(() =>
    tryReadInt(STORAGE_SAVED, DEFAULT_WIDTH)
  );

  const [width, setWidth] = useState(() => {
    try { if (localStorage.getItem(STORAGE_COLLAPSED) === 'true') return 0; } catch { /* storage unavailable */ }
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

  // Tracks whether collapse happened during an active drag so we can re-expand
  // without waiting for a React re-render to update isCollapsedRef.
  const collapsedDuringDragRef = useRef(false);

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

  // Called from CollapsedSidebarStrip on mousedown — starts a live resize drag
  // from the collapsed state so the sidebar follows the cursor until mouse release.
  const startResizeFromCollapsed = useCallback(() => {
    collapsedDuringDragRef.current = true;
    setIsResizing(true);
  }, []);

  useEffect(() => {
    if (!isResizing) return;

    const onMouseMove = (e: MouseEvent) => {
      if (collapsedDuringDragRef.current) {
        // Sidebar collapsed during this drag — re-expand if user drags right past threshold
        if (e.clientX > COLLAPSE_THRESHOLD + 20) {
          const w = Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, e.clientX));
          collapsedDuringDragRef.current = false;
          setWidth(w);
          setIsCollapsed(false);
          tryWrite(STORAGE_COLLAPSED, 'false');
          tryWrite(STORAGE_WIDTH, String(w));
        }
        return;
      }

      // getBoundingClientRect().left is 0 for the sidebar (always at left edge)
      // but we use it correctly in case layout ever adds left inset
      const sidebarLeft = resizableRef.current?.getBoundingClientRect().left ?? 0;
      const newWidth = e.clientX - sidebarLeft;

      if (newWidth < COLLAPSE_THRESHOLD) {
        // Snap to collapsed — save current width first, keep drag active
        const sw = widthRef.current > 0 ? widthRef.current : savedWidthRef.current;
        setSavedWidth(sw);
        savedWidthRef.current = sw;
        tryWrite(STORAGE_SAVED, String(sw));
        tryWrite(STORAGE_COLLAPSED, 'true');
        setWidth(0);
        setIsCollapsed(true);
        collapsedDuringDragRef.current = true;
        // Do NOT setIsResizing(false) — keep drag active so user can re-expand
      } else {
        setWidth(Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, newWidth)));
      }
    };

    const onMouseUp = () => {
      collapsedDuringDragRef.current = false;
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

  debug.log('useCollapsibleSidebar', { width, isCollapsed, isResizing, savedWidth, isMobile, mobileShowContent });

  return { width, isCollapsed, isResizing, savedWidth, isMobile, mobileShowContent, setMobileShowContent, handleMouseDown, expand, startResizeFromCollapsed, resizableRef };
}
