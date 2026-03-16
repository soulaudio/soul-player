import { describe, it, expect, beforeEach, vi } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useCollapsibleSidebar } from '../useCollapsibleSidebar';

// ── localStorage mock ──────────────────────────────────────────────────────
const localStorageMock = (() => {
  let store: Record<string, string> = {};
  return {
    getItem: (key: string) => store[key] ?? null,
    setItem: (key: string, value: string) => { store[key] = value; },
    removeItem: (key: string) => { delete store[key]; },
    clear: () => { store = {}; },
  };
})();
Object.defineProperty(window, 'localStorage', { value: localStorageMock });

beforeEach(() => localStorageMock.clear());

describe('useCollapsibleSidebar — initialisation', () => {
  it('starts expanded with default width 288 when localStorage is empty', () => {
    const { result } = renderHook(() => useCollapsibleSidebar());
    expect(result.current.isCollapsed).toBe(false);
    expect(result.current.width).toBe(288);
  });

  it('reads isCollapsed=true from localStorage on mount', () => {
    localStorageMock.setItem('soul-player:sidebar-collapsed', 'true');
    localStorageMock.setItem('soul-player:sidebar-saved-width', '320');
    const { result } = renderHook(() => useCollapsibleSidebar());
    expect(result.current.isCollapsed).toBe(true);
    expect(result.current.width).toBe(0);
  });

  it('reads savedWidth from localStorage on mount', () => {
    localStorageMock.setItem('soul-player:sidebar-saved-width', '350');
    const { result } = renderHook(() => useCollapsibleSidebar());
    expect(result.current.savedWidth).toBe(350);
  });
});

describe('useCollapsibleSidebar — handleMouseDown', () => {
  it('sets isResizing to true', () => {
    const { result } = renderHook(() => useCollapsibleSidebar());
    act(() => {
      result.current.handleMouseDown({ preventDefault: vi.fn() } as any);
    });
    expect(result.current.isResizing).toBe(true);
  });
});

describe('useCollapsibleSidebar — snap to collapsed', () => {
  it('snaps to collapsed when mousemove goes below 200px threshold', () => {
    const { result } = renderHook(() => useCollapsibleSidebar());
    act(() => {
      result.current.handleMouseDown({ preventDefault: vi.fn() } as any);
    });
    act(() => {
      document.dispatchEvent(new MouseEvent('mousemove', { clientX: 100, bubbles: true }));
    });
    expect(result.current.isCollapsed).toBe(true);
    expect(result.current.width).toBe(0);
    expect(localStorageMock.getItem('soul-player:sidebar-collapsed')).toBe('true');
  });

  it('does NOT snap when mousemove stays at or above 200px', () => {
    const { result } = renderHook(() => useCollapsibleSidebar());
    act(() => {
      result.current.handleMouseDown({ preventDefault: vi.fn() } as any);
    });
    act(() => {
      document.dispatchEvent(new MouseEvent('mousemove', { clientX: 250, bubbles: true }));
    });
    expect(result.current.isCollapsed).toBe(false);
    expect(result.current.width).toBe(250);
  });

  it('does NOT write soul-player:sidebar-width during snap-to-collapsed', () => {
    localStorageMock.setItem('soul-player:sidebar-width', '300');
    const { result } = renderHook(() => useCollapsibleSidebar());
    act(() => {
      result.current.handleMouseDown({ preventDefault: vi.fn() } as any);
    });
    act(() => {
      document.dispatchEvent(new MouseEvent('mousemove', { clientX: 50, bubbles: true }));
    });
    // sidebar-width key must remain at its pre-collapse value
    expect(localStorageMock.getItem('soul-player:sidebar-width')).toBe('300');
    expect(localStorageMock.getItem('soul-player:sidebar-collapsed')).toBe('true');
  });
});

describe('useCollapsibleSidebar — expand()', () => {
  it('restores savedWidth and sets isCollapsed=false', () => {
    localStorageMock.setItem('soul-player:sidebar-collapsed', 'true');
    localStorageMock.setItem('soul-player:sidebar-saved-width', '320');
    const { result } = renderHook(() => useCollapsibleSidebar());
    act(() => result.current.expand());
    expect(result.current.isCollapsed).toBe(false);
    expect(result.current.width).toBe(320);
  });

  it('accepts an explicit width override', () => {
    const { result } = renderHook(() => useCollapsibleSidebar());
    act(() => result.current.expand(400));
    expect(result.current.width).toBe(400);
  });

  it('persists isCollapsed=false and sidebar-width to localStorage', () => {
    const { result } = renderHook(() => useCollapsibleSidebar());
    act(() => result.current.expand(400));
    expect(localStorageMock.getItem('soul-player:sidebar-collapsed')).toBe('false');
    expect(localStorageMock.getItem('soul-player:sidebar-width')).toBe('400');
  });
});
