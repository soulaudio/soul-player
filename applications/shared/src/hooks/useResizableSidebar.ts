import { useState, useEffect, useCallback, useRef } from 'react';

interface UseResizableSidebarOptions {
  /** Minimum width in pixels */
  minWidth?: number;
  /** Maximum width in pixels */
  maxWidth?: number;
  /** Default width in pixels */
  defaultWidth?: number;
  /** LocalStorage key for persisting width */
  storageKey?: string;
}

interface UseResizableSidebarReturn {
  /** Current width in pixels */
  width: number;
  /** Whether the sidebar is currently being resized */
  isResizing: boolean;
  /** Handler for mousedown on resize handle */
  handleMouseDown: (e: React.MouseEvent) => void;
  /** Ref to attach to the resizable element */
  resizableRef: React.RefObject<HTMLDivElement>;
}

const DEFAULT_MIN_WIDTH = 240; // 240px (w-60)
const DEFAULT_MAX_WIDTH = 480; // 480px
const DEFAULT_WIDTH = 288; // 288px (w-72)
const STORAGE_KEY = 'soul-player:sidebar-width';

export function useResizableSidebar(
  options: UseResizableSidebarOptions = {}
): UseResizableSidebarReturn {
  const {
    minWidth = DEFAULT_MIN_WIDTH,
    maxWidth = DEFAULT_MAX_WIDTH,
    defaultWidth = DEFAULT_WIDTH,
    storageKey = STORAGE_KEY,
  } = options;

  const resizableRef = useRef<HTMLDivElement>(null);
  const [isResizing, setIsResizing] = useState(false);

  // Load width from localStorage or use default
  const [width, setWidth] = useState(() => {
    try {
      const stored = localStorage.getItem(storageKey);
      if (stored) {
        const parsed = parseInt(stored, 10);
        if (!isNaN(parsed)) {
          return Math.max(minWidth, Math.min(maxWidth, parsed));
        }
      }
    } catch (error) {
      console.error('[useResizableSidebar] Failed to load width from storage:', error);
    }
    return defaultWidth;
  });

  // Save width to localStorage
  const saveWidth = useCallback((newWidth: number) => {
    try {
      localStorage.setItem(storageKey, String(newWidth));
    } catch (error) {
      console.error('[useResizableSidebar] Failed to save width to storage:', error);
    }
  }, [storageKey]);

  // Handle mouse move during resize
  const handleMouseMove = useCallback(
    (e: MouseEvent) => {
      if (!isResizing) return;

      const newWidth = e.clientX;
      const clampedWidth = Math.max(minWidth, Math.min(maxWidth, newWidth));
      setWidth(clampedWidth);
    },
    [isResizing, minWidth, maxWidth]
  );

  // Handle mouse up to end resize
  const handleMouseUp = useCallback(() => {
    if (isResizing) {
      setIsResizing(false);
      saveWidth(width);
    }
  }, [isResizing, width, saveWidth]);

  // Handle mouse down to start resize
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsResizing(true);
  }, []);

  // Add/remove event listeners for resize
  useEffect(() => {
    if (isResizing) {
      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleMouseUp);
      // Prevent text selection during resize
      document.body.style.cursor = 'ew-resize';
      document.body.style.userSelect = 'none';

      return () => {
        document.removeEventListener('mousemove', handleMouseMove);
        document.removeEventListener('mouseup', handleMouseUp);
        document.body.style.cursor = '';
        document.body.style.userSelect = '';
      };
    }
  }, [isResizing, handleMouseMove, handleMouseUp]);

  return {
    width,
    isResizing,
    handleMouseDown,
    resizableRef,
  };
}
