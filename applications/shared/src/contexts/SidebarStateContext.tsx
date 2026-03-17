import { createContext, useContext, ReactNode } from 'react';
import { useCollapsibleSidebar, type CollapsibleSidebarState } from '../hooks/useCollapsibleSidebar';

// No-op defaults — consumers outside a provider fail silently (graceful degradation)
const SidebarStateContext = createContext<CollapsibleSidebarState>({
  width: 288,
  isCollapsed: false,
  isResizing: false,
  savedWidth: 288,
  handleMouseDown: () => {},
  expand: () => {},
  startResizeFromCollapsed: () => {},
  resizableRef: { current: null } as React.RefObject<HTMLDivElement>,
});

export const useSidebarState = () => useContext(SidebarStateContext);

/**
 * SidebarStateProvider — owns useCollapsibleSidebar() and publishes state
 * to all children via context. Render this in MainLayout wrapping both
 * LeftSidebar and NowPlayingFloating so they share the same collapse state.
 */
export function SidebarStateProvider({ children }: { children: ReactNode }) {
  const state = useCollapsibleSidebar();
  return (
    <SidebarStateContext.Provider value={state}>
      {children}
    </SidebarStateContext.Provider>
  );
}
