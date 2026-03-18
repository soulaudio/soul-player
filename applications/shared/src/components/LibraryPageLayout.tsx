/**
 * LibraryPageLayout - Reusable layout for library pages with search and auto-hide header
 */

import { useEffect, useRef, ReactNode, useCallback, useState, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { Search, X } from 'lucide-react'
import { FeatureGate, useFeatures } from '../contexts/PlatformContext'
import { useScrollVisibility } from '../contexts/ScrollVisibilityContext'
import { useBackend } from '../contexts/BackendContext'
import { SkeletonCard } from './SkeletonCard'
import { useScrollRestoration } from '../hooks/useScrollRestoration'

interface LibraryPageLayoutProps {
  /** Current search query */
  searchQuery: string
  /** Search query setter */
  setSearchQuery: (query: string) => void
  /** Total number of items for search placeholder */
  itemCount: number
  /** Translation key for search placeholder (e.g., 'library.search.albumsWithCount') */
  searchPlaceholderKey: string
  /** Optional health warning message */
  healthWarning?: string | null
  /** Optional additional buttons next to search (e.g., Create Playlist button) */
  additionalButtons?: ReactNode
  /** Loading state */
  isLoading?: boolean
  /** Item type for skeleton cards */
  itemType?: 'album' | 'artist' | 'playlist' | 'track'
  /** Grid class for layout */
  gridClass?: string
  /** Cache key for storing item count in localStorage */
  cacheKey?: string
  /** Optional data-testid for the page container (e.g., 'albums-page') */
  pageTestId?: string
  /** Optional expandable filter panel rendered below the search bar */
  filterPanel?: ReactNode
  /** When true, increases content top padding to account for filter panel height */
  filterPanelVisible?: boolean
  /** Custom skeleton to show while loading (overrides default SkeletonCard grid) */
  customSkeleton?: ReactNode
  /** The main content (grid, list, etc.) */
  children: ReactNode
}

export function LibraryPageLayout({
  searchQuery,
  setSearchQuery,
  itemCount,
  searchPlaceholderKey,
  healthWarning,
  additionalButtons,
  isLoading = false,
  itemType = 'album',
  gridClass = 'grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6',
  cacheKey,
  pageTestId,
  filterPanel,
  filterPanelVisible = false,
  customSkeleton,
  children,
}: LibraryPageLayoutProps) {
  const { t } = useTranslation()
  const backend = useBackend()
  const { hasAutoHideSearch } = useFeatures()
  const { showHeader: showSearchBar, setShowHeader: setShowSearchBar, scrollContainerRef } = useScrollVisibility()
  useScrollRestoration(scrollContainerRef)
  const autoHideSearchRef = useRef(hasAutoHideSearch)
  const [showGradients, setShowGradients] = useState(true)

  useEffect(() => {
    Promise.all([
      backend.getUserSetting('ui.hide_library_search'),
      backend.getUserSetting('ui.show_library_gradients'),
    ])
      .then(([hideSearch, showGrad]) => {
        const autoHide = hasAutoHideSearch ? (hideSearch ?? true) : false
        autoHideSearchRef.current = autoHide
        if (!autoHide) setShowSearchBar(true)
        setShowGradients(showGrad ?? true)
      })
      .catch(() => {/* ignore */})

    const searchHandler = (e: Event) => {
      const val = hasAutoHideSearch ? (e as CustomEvent).detail.autoHide as boolean : false
      autoHideSearchRef.current = val
      if (!val) setShowSearchBar(true)
    }
    const gradientsHandler = (e: Event) => {
      setShowGradients((e as CustomEvent).detail.show as boolean)
    }
    window.addEventListener('library-search-hidden-changed', searchHandler)
    window.addEventListener('library-gradients-changed', gradientsHandler)
    return () => {
      window.removeEventListener('library-search-hidden-changed', searchHandler)
      window.removeEventListener('library-gradients-changed', gradientsHandler)
    }
  }, [backend, setShowSearchBar])

  const lastScrollTop = useRef(0)
  const idleTimerRef = useRef<number | null>(null)
  const hiddenByIdleRef = useRef(false)
  const showSearchBarRef = useRef(showSearchBar)
  // Accumulates scroll delta in the current direction — only trigger show/hide
  // once 50px of net directional scroll has built up, preventing flicker on
  // fast up/down scrolling where individual frames alternate direction.
  const scrollAccumulatorRef = useRef(0)

  // Refs for direct DOM gradient manipulation — avoids React re-renders on every scroll frame
  const topGradientRef = useRef<HTMLDivElement>(null)
  const bottomGradientRef = useRef<HTMLDivElement>(null)
  // Throttle mousemove → idle timer resets (max once per 200ms)
  const lastMouseMoveRef = useRef(0)

  // Keep ref in sync with state
  useEffect(() => {
    showSearchBarRef.current = showSearchBar
  }, [showSearchBar])

  // Cache item count in localStorage
  useEffect(() => {
    if (cacheKey && itemCount > 0 && !isLoading) {
      try {
        localStorage.setItem(cacheKey, String(itemCount))
      } catch (e) {
        // Ignore localStorage errors
      }
    }
  }, [cacheKey, itemCount, isLoading])

  // Get cached item count for skeleton loading
  const cachedCount = useMemo(() => {
    if (!cacheKey) return 12 // Default skeleton count
    try {
      const cached = localStorage.getItem(cacheKey)
      return cached ? parseInt(cached, 10) : 12
    } catch (e) {
      return 12
    }
  }, [cacheKey])

  // Always show header when loading
  useEffect(() => {
    if (isLoading) {
      setShowSearchBar(true)
      hiddenByIdleRef.current = false
      if (topGradientRef.current) topGradientRef.current.style.opacity = '0'
    }
  }, [isLoading, setShowSearchBar])

  // Reset idle timer and show header
  const resetIdleTimer = useCallback(() => {
    // Clear existing timer
    if (idleTimerRef.current !== null) {
      window.clearTimeout(idleTimerRef.current)
    }

    // Show header on activity (only if hidden by idle)
    if (hiddenByIdleRef.current) {
      setShowSearchBar(true)
      hiddenByIdleRef.current = false
      if (topGradientRef.current) topGradientRef.current.style.opacity = '0'
    }

    // Set new timer to hide after 3 seconds of inactivity (only when auto-hide is enabled)
    idleTimerRef.current = window.setTimeout(() => {
      if (!autoHideSearchRef.current) return

      // Check if at bottom before hiding to prevent padding change loop
      const scrollContainer = scrollContainerRef.current
      if (scrollContainer) {
        const scrollTop = scrollContainer.scrollTop
        const scrollHeight = scrollContainer.scrollHeight
        const clientHeight = scrollContainer.clientHeight
        const atBottom = scrollHeight - scrollTop - clientHeight < 10

        // Don't hide if at bottom (prevents infinite loop from padding changes)
        if (atBottom) {
          return
        }
      }

      // Hide after idle timeout
      setShowSearchBar(false)
      hiddenByIdleRef.current = true

      // Update gradient since header is now hidden and no scroll event is coming
      if (topGradientRef.current && scrollContainerRef.current) {
        const scrollTop = scrollContainerRef.current.scrollTop
        if (scrollTop > 10) topGradientRef.current.style.opacity = '1'
      }
    }, 3000)
  }, [setShowSearchBar, scrollContainerRef])

  // Show header when component mounts (page/tab switch)
  useEffect(() => {
    scrollAccumulatorRef.current = 0
    setShowSearchBar(true)
    hiddenByIdleRef.current = false
    if (topGradientRef.current) topGradientRef.current.style.opacity = '0'
  }, [setShowSearchBar])

  // Hide/show search bar on scroll
  useEffect(() => {
    const scrollContainer = scrollContainerRef.current
    if (!scrollContainer) return

    let ticking = false

    const handleScroll = () => {
      if (!ticking) {
        window.requestAnimationFrame(() => {
          const scrollTop = scrollContainer.scrollTop
          const scrollDelta = scrollTop - lastScrollTop.current
          const scrollHeight = scrollContainer.scrollHeight
          const clientHeight = scrollContainer.clientHeight

          // Check if at bottom (within 10px threshold)
          const atBottom = scrollHeight - scrollTop - clientHeight < 10

          // Update gradients directly on the DOM — no React state update, no re-render
          const gradientOpacity = autoHideSearchRef.current && !showSearchBarRef.current && scrollTop > 10 ? '1' : '0'
          if (topGradientRef.current) topGradientRef.current.style.opacity = gradientOpacity
          if (bottomGradientRef.current) bottomGradientRef.current.style.opacity = atBottom ? '0' : '1'

          // Show and reset idle timer if scrolled to the very top — immediate, no debounce
          if (scrollTop <= 10) {
            scrollAccumulatorRef.current = 0
            setShowSearchBar(true)
            hiddenByIdleRef.current = false
            resetIdleTimer()
            lastScrollTop.current = scrollTop
            ticking = false
            return
          }

          // Accumulate scroll delta in the current direction.
          // If direction reverses, reset accumulator so fast up/down thrashing
          // doesn't trigger show/hide until the user commits to a direction.
          const prev = scrollAccumulatorRef.current
          if ((scrollDelta > 0 && prev < 0) || (scrollDelta < 0 && prev > 0)) {
            scrollAccumulatorRef.current = scrollDelta
          } else {
            scrollAccumulatorRef.current += scrollDelta
          }
          const accumulated = scrollAccumulatorRef.current

          // Hide when 50px of downward scroll has accumulated past threshold
          if (autoHideSearchRef.current && accumulated > 50 && scrollTop > 50 && !atBottom) {
            scrollAccumulatorRef.current = 0
            setShowSearchBar(false)
            hiddenByIdleRef.current = false
            if (idleTimerRef.current !== null) {
              window.clearTimeout(idleTimerRef.current)
              idleTimerRef.current = null
            }
          }
          // Show when 50px of upward scroll has accumulated (not at bottom to prevent loop)
          else if (accumulated < -50 && !atBottom) {
            scrollAccumulatorRef.current = 0
            setShowSearchBar(true)
            hiddenByIdleRef.current = false
            if (autoHideSearchRef.current) resetIdleTimer()
          }

          lastScrollTop.current = scrollTop
          ticking = false
        })
        ticking = true
      }
    }

    scrollContainer.addEventListener('scroll', handleScroll, { passive: true })
    return () => scrollContainer.removeEventListener('scroll', handleScroll)
  }, [setShowSearchBar, resetIdleTimer, scrollContainerRef])

  // Mouse movement detection for idle timeout - only in main content area
  useEffect(() => {
    const scrollContainer = scrollContainerRef.current
    if (!scrollContainer) return

    const handleMouseMove = (e: MouseEvent) => {
      // Get the bounding rect of the scroll container
      const rect = scrollContainer.getBoundingClientRect()
      const mouseY = e.clientY - rect.top

      // If mouse is at the top of the scroll container (within 100px from top)
      // and header is hidden, show it
      if (mouseY >= 0 && mouseY <= 100 && !showSearchBarRef.current) {
        setShowSearchBar(true)
        hiddenByIdleRef.current = false
        if (topGradientRef.current) topGradientRef.current.style.opacity = '0'
      }

      // Throttle idle timer resets to max once per 200ms to avoid setTimeout churn
      if (autoHideSearchRef.current) {
        const now = Date.now()
        if (now - lastMouseMoveRef.current > 200) {
          lastMouseMoveRef.current = now
          resetIdleTimer()
        }
      }
    }

    // Start idle timer on mount (only when auto-hide is enabled)
    if (autoHideSearchRef.current) resetIdleTimer()

    // Only listen to mouse movement within the scroll container (main content area)
    scrollContainer.addEventListener('mousemove', handleMouseMove, { passive: true })

    return () => {
      scrollContainer.removeEventListener('mousemove', handleMouseMove)
      if (idleTimerRef.current !== null) {
        window.clearTimeout(idleTimerRef.current)
      }
    }
  }, [resetIdleTimer, setShowSearchBar, scrollContainerRef])

  return (
    <div className="h-full flex flex-col overflow-hidden relative" data-testid={pageTestId}>
      {/* Health warning */}
      <FeatureGate feature="hasHealthCheck">
        {healthWarning && (
          <div className="mb-4 p-4 bg-yellow-500/10 border border-yellow-500/20 rounded-lg">
            <div className="flex items-start gap-3">
              <div className="flex-shrink-0 w-5 h-5 rounded-full bg-yellow-500/20 flex items-center justify-center mt-0.5">
                <span className="text-yellow-600 dark:text-yellow-400 text-sm">!</span>
              </div>
              <div className="flex-1">
                <p className="text-sm text-yellow-800 dark:text-yellow-200 font-medium">
                  {t('library.databaseIssue')}
                </p>
                <p className="text-sm text-yellow-700 dark:text-yellow-300 mt-1">
                  {healthWarning}
                </p>
              </div>
            </div>
          </div>
        )}
      </FeatureGate>

      {/* Search bar — absolute overlay so scrollbar starts at top */}
      <div
        className={`absolute top-0 left-0 right-0 z-10 bg-background mr-4 sm:mr-6 pt-8 pb-2 transition-all duration-300 ${
          showSearchBar ? 'opacity-100 translate-y-0' : 'opacity-0 -translate-y-full pointer-events-none'
        }`}
      >
          <div className="flex items-center gap-4">
            <div className="relative flex-1 sm:max-w-md">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
              <input
                type="text"
                data-testid="search-input"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder={
                  isLoading || itemCount === 0
                    ? t(searchPlaceholderKey.replace('WithCount', ''))
                    : t(searchPlaceholderKey, { count: itemCount })
                }
                className="w-full pl-10 pr-4 py-2 rounded-lg bg-muted border border-transparent focus:border-primary focus:outline-none text-sm"
                disabled={isLoading}
              />
              {searchQuery && (
                <button
                  onClick={() => setSearchQuery('')}
                  className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:opacity-[var(--hover-text-opacity)] transition-opacity duration-[var(--transition-duration)]"
                >
                  <X className="w-4 h-4" />
                </button>
              )}
            </div>
            {additionalButtons}
          </div>
          {filterPanel}
        </div>

      {/* Scrollable Content — scrollbar starts at very top */}
      <div
        ref={scrollContainerRef}
        data-testid="scroll-container"
        className="flex-1 overflow-y-auto pr-0 scrollbar-custom"
      >
        <div className={`pr-6 pb-20 sm:pb-6 ${
          showSearchBar
            ? filterPanelVisible ? 'pt-24' : 'pt-14'
            : 'pt-4 sm:pt-2'
        }`}>
          {isLoading ? (
            customSkeleton || (
              <div className={`grid gap-3 sm:gap-4 ${gridClass}`}>
                {Array.from({ length: cachedCount }).map((_, index) => (
                  <SkeletonCard key={index} type={itemType} />
                ))}
              </div>
            )
          ) : (
            children
          )}
        </div>
      </div>

      {/* Top gradient overlay — opacity controlled directly via ref to avoid re-renders */}
      {showGradients && (
        <div
          ref={topGradientRef}
          className="absolute top-0 left-0 right-0 h-24 pointer-events-none z-10 transition-opacity duration-300"
          style={{
            background: 'linear-gradient(to bottom, hsl(var(--background)) 0%, transparent 100%)',
            opacity: 0
          }}
        />
      )}

      {/* Bottom gradient overlay — opacity controlled directly via ref to avoid re-renders */}
      {showGradients && (
        <div
          ref={bottomGradientRef}
          className="absolute bottom-0 left-0 right-0 h-24 pointer-events-none z-10 transition-opacity duration-300"
          style={{
            background: 'linear-gradient(to top, hsl(var(--background) / 0.75) 0%, transparent 100%)',
            opacity: 1
          }}
        />
      )}
    </div>
  )
}
