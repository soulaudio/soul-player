import { useEffect, useRef, type RefObject } from 'react';
import { useLocation } from 'react-router-dom';

/**
 * Module-level store for scroll positions keyed by pathname.
 * Survives React component unmounts within a session but clears on page reload.
 */
const scrollPositions = new Map<string, number>();

/**
 * Saves and restores the scroll position of a container element when the
 * component mounts/unmounts due to route changes.
 *
 * Usage: call inside any component that owns a scrollable container, passing
 * the ref to that container. The current pathname is used as the cache key so
 * each page (/albums, /artists, /tracks, …) has an independent saved position.
 *
 * Timing:
 * - Save: captured via scroll event listener (avoids stale-ref issues at cleanup time)
 * - Restore: applied in a requestAnimationFrame after mount so virtualised content
 *   has had one frame to measure itself before we reposition
 */
export function useScrollRestoration(scrollEl: RefObject<HTMLDivElement | null>) {
  const { pathname, state } = useLocation();
  const isBack = !!(state as any)?.isBack;

  // Keep pathname ref always current so the unmount cleanup saves to the right key
  const pathnameRef = useRef(pathname);
  pathnameRef.current = pathname;

  // Track the latest scroll position via event listener.
  // We read from a ref rather than from scrollEl.current at cleanup time because
  // React may have already detached/replaced the ref by then.
  const lastScrollTop = useRef(0);

  useEffect(() => {
    const el = scrollEl.current;
    if (!el) return;

    // Sync initial value in case the element was already scrolled (e.g. fast navigation)
    lastScrollTop.current = el.scrollTop;

    const onScroll = () => {
      lastScrollTop.current = el.scrollTop;
    };

    el.addEventListener('scroll', onScroll, { passive: true });
    return () => el.removeEventListener('scroll', onScroll);
  }); // intentionally no dep array — re-attaches whenever el might change

  // Save position on unmount
  useEffect(() => {
    return () => {
      const pos = lastScrollTop.current;
      if (pos > 0) {
        scrollPositions.set(pathnameRef.current, pos);
      }
    };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Restore position on mount only when navigating back (via goBack()).
  // On fresh forward navigation, scroll resets to top.
  useEffect(() => {
    if (!isBack) return;

    const saved = scrollPositions.get(pathname);
    if (!saved) return;

    const frame = requestAnimationFrame(() => {
      if (scrollEl.current) {
        scrollEl.current.scrollTop = saved;
      }
    });

    return () => cancelAnimationFrame(frame);
  }, []); // eslint-disable-line react-hooks/exhaustive-deps
}
