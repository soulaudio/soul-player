/**
 * LibraryPageLayout - Reusable layout for library pages with search and auto-hide header
 */

import { useEffect, useRef, ReactNode, useCallback, useState, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { Search, X } from 'lucide-react'
import { FeatureGate } from '../contexts/PlatformContext'
import { useScrollVisibility } from '../contexts/ScrollVisibilityContext'
import { SkeletonCard } from './SkeletonCard'

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
  children,
}: LibraryPageLayoutProps) {
  const { t } = useTranslation()
  const { showHeader: showSearchBar, setShowHeader: setShowSearchBar } = useScrollVisibility()
  const scrollContainerRef = useRef<HTMLDivElement>(null)
  const lastScrollTop = useRef(0)
  const idleTimerRef = useRef<number | null>(null)
  const hiddenByIdleRef = useRef(false) // Track if hidden by idle timeout
  const showSearchBarRef = useRef(showSearchBar) // Ref to track current visibility without re-running effects
  const [isAtBottom, setIsAtBottom] = useState(false)
  const [topGradientOpacity, setTopGradientOpacity] = useState(0) // Smooth gradient opacity based on scroll position

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
      // Hide gradient when header is visible during loading
      setTopGradientOpacity(0)
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
      // Hide gradient when header appears
      setTopGradientOpacity(0)
    }

    // Set new timer to hide after 3 seconds of inactivity
    idleTimerRef.current = window.setTimeout(() => {
      // Check if at bottom before hiding to prevent padding change loop
      const scrollContainer = scrollContainerRef.current
      if (scrollContainer) {
        const scrollTop = scrollContainer.scrollTop
        const scrollHeight = scrollContainer.scrollHeight
        const clientHeight = scrollContainer.clientHeight
        const atBottom = scrollHeight - scrollTop - clientHeight < 10

        // Don't hide if at bottom (prevents infinite loop from padding changes)
        // Next mouse movement will reset the timer
        if (atBottom) {
          return
        }
      }

      // Hide after idle timeout
      setShowSearchBar(false)
      hiddenByIdleRef.current = true // Mark as hidden by idle
    }, 3000)
  }, [setShowSearchBar])

  // Show header when component mounts (page/tab switch)
  useEffect(() => {
    setShowSearchBar(true)
    hiddenByIdleRef.current = false
    // Set initial gradient opacity - hidden since header is visible on mount
    setTopGradientOpacity(0)
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
          setIsAtBottom(atBottom)

          // Calculate top gradient opacity based on scroll position
          // Gradient only visible when:
          // 1. Header is hidden (showSearchBarRef.current is false)
          // 2. User has scrolled down (scrollTop > 10)
          let calculatedOpacity = 0

          if (!showSearchBarRef.current && scrollTop > 10) {
            // Header is hidden and scrolled down - show gradient
            calculatedOpacity = 1
          } else {
            // Header is visible OR at top - hide gradient
            calculatedOpacity = 0
          }

          setTopGradientOpacity(calculatedOpacity)

          // Show and reset idle timer if scrolled to the very top
          if (scrollTop <= 10) {
            setShowSearchBar(true)
            hiddenByIdleRef.current = false
            // Reset idle timer when at top
            resetIdleTimer()
            lastScrollTop.current = scrollTop
            ticking = false
            return
          }

          // Require at least 5px scroll to prevent jitter
          if (Math.abs(scrollDelta) < 5) {
            ticking = false
            return
          }

          // Hide when scrolling down past threshold (but not when at bottom to prevent loop)
          if (scrollDelta > 0 && scrollTop > 50 && !atBottom) {
            setShowSearchBar(false)
            hiddenByIdleRef.current = false // Hidden by manual scroll, not idle
            // Clear idle timer when manually hiding
            if (idleTimerRef.current !== null) {
              window.clearTimeout(idleTimerRef.current)
              idleTimerRef.current = null
            }
          }
          // Show when scrolling up (but not when at bottom to prevent padding change loop)
          else if (scrollDelta < 0 && !atBottom) {
            setShowSearchBar(true)
            hiddenByIdleRef.current = false // Reset idle flag
            // Reset idle timer on scroll up
            resetIdleTimer()
          }

          lastScrollTop.current = scrollTop
          ticking = false
        })
        ticking = true
      }
    }

    scrollContainer.addEventListener('scroll', handleScroll, { passive: true })
    return () => scrollContainer.removeEventListener('scroll', handleScroll)
  }, [setShowSearchBar, resetIdleTimer])

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
        // Hide gradient when header appears on hover
        setTopGradientOpacity(0)
      }

      // Always reset idle timer on mouse movement
      resetIdleTimer()
    }

    // Start idle timer on mount
    resetIdleTimer()

    // Only listen to mouse movement within the scroll container (main content area)
    scrollContainer.addEventListener('mousemove', handleMouseMove, { passive: true })

    return () => {
      scrollContainer.removeEventListener('mousemove', handleMouseMove)
      if (idleTimerRef.current !== null) {
        window.clearTimeout(idleTimerRef.current)
      }
    }
  }, [resetIdleTimer, setShowSearchBar])

  return (
    <div className="h-full flex flex-col overflow-hidden relative">
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

      {/* Search bar - auto-hide on scroll, absolutely positioned */}
      <div
        className={`absolute top-0 left-0 right-0 bg-background z-10 transition-all duration-300 mr-6 pb-3 ${
          showSearchBar ? 'translate-y-0' : '-translate-y-full'
        }`}
      >
        <div className="flex items-center gap-4">
          <div className="relative flex-1 sm:max-w-md">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
            <input
              type="text"
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
      </div>

      {/* Scrollable Content - with dynamic padding for search bar */}
      <div
        ref={scrollContainerRef}
        className={`flex-1 overflow-y-auto pr-6 pb-6 scrollbar-custom transition-all duration-300 ${
          showSearchBar ? 'pt-14' : 'pt-6'
        }`}
      >
        {isLoading ? (
          <div className={`grid gap-3 sm:gap-4 ${gridClass}`}>
            {Array.from({ length: cachedCount }).map((_, index) => (
              <SkeletonCard key={index} type={itemType} />
            ))}
          </div>
        ) : (
          children
        )}
      </div>

      {/* Top gradient overlay - smoothly fades based on scroll position */}
      <div
        className="absolute top-0 left-0 right-0 h-24 pointer-events-none z-10 transition-opacity duration-300"
        style={{
          background: 'linear-gradient(to bottom, hsl(var(--background)) 0%, transparent 100%)',
          opacity: topGradientOpacity
        }}
      />

      {/* Bottom gradient overlay - subtle and disappears at bottom */}
      <div
        className={`absolute bottom-0 left-0 right-0 h-24 pointer-events-none z-10 transition-opacity duration-300 ${
          isAtBottom ? 'opacity-0' : 'opacity-100'
        }`}
        style={{
          background: 'linear-gradient(to top, hsl(var(--background) / 0.75) 0%, transparent 100%)'
        }}
      />
    </div>
  )
}
