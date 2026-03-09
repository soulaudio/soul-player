/**
 * DetailPageLayout - Reusable layout for detail pages (album, artist, playlist)
 * with auto-hide header behavior on scroll
 */

import { useEffect, useRef, ReactNode, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { ArrowLeft } from 'lucide-react'
import { useScrollVisibility } from '../contexts/ScrollVisibilityContext'
import { useNavigateWithHistory } from '../hooks/useNavigateWithHistory'

interface DetailPageLayoutProps {
  /** Content above the main list (artist avatar, album cover, info, etc.) */
  header: ReactNode
  /** Main scrollable content (track lists, discography, etc.) */
  children: ReactNode
  /** Default back path when no history */
  fallbackBackPath: string
  /** Translation key for back button when no history */
  backLabelKey: string
}

export function DetailPageLayout({
  header,
  children,
  fallbackBackPath,
  backLabelKey,
}: DetailPageLayoutProps) {
  const { t } = useTranslation()
  const { goBack, hasHistory } = useNavigateWithHistory()
  const { showHeader, setShowHeader } = useScrollVisibility()
  const scrollContainerRef = useRef<HTMLDivElement>(null)
  const lastScrollTop = useRef(0)
  const idleTimerRef = useRef<number | null>(null)
  const hiddenByIdleRef = useRef(false)
  const showHeaderRef = useRef(showHeader)

  // Refs for direct DOM gradient manipulation — avoids React re-renders on every scroll frame
  const topGradientRef = useRef<HTMLDivElement>(null)
  const bottomGradientRef = useRef<HTMLDivElement>(null)
  // Throttle mousemove → idle timer resets (max once per 200ms)
  const lastMouseMoveRef = useRef(0)

  // Keep ref in sync with state
  useEffect(() => {
    showHeaderRef.current = showHeader
  }, [showHeader])

  // Reset idle timer and show header
  const resetIdleTimer = useCallback(() => {
    if (idleTimerRef.current !== null) {
      window.clearTimeout(idleTimerRef.current)
    }

    if (hiddenByIdleRef.current) {
      setShowHeader(true)
      hiddenByIdleRef.current = false
      if (topGradientRef.current) topGradientRef.current.style.opacity = '0'
    }

    idleTimerRef.current = window.setTimeout(() => {
      const scrollContainer = scrollContainerRef.current
      if (scrollContainer) {
        const scrollTop = scrollContainer.scrollTop
        const scrollHeight = scrollContainer.scrollHeight
        const clientHeight = scrollContainer.clientHeight
        const atBottom = scrollHeight - scrollTop - clientHeight < 10

        if (atBottom) {
          return
        }
      }

      setShowHeader(false)
      hiddenByIdleRef.current = true

      // Update gradient since header is now hidden and no scroll event is coming
      if (topGradientRef.current && scrollContainerRef.current) {
        const scrollTop = scrollContainerRef.current.scrollTop
        if (scrollTop > 10) topGradientRef.current.style.opacity = '1'
      }
    }, 3000)
  }, [setShowHeader])

  // Show header when component mounts
  useEffect(() => {
    setShowHeader(true)
    hiddenByIdleRef.current = false
    if (topGradientRef.current) topGradientRef.current.style.opacity = '0'
  }, [setShowHeader])

  // Hide/show header on scroll
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

          const atBottom = scrollHeight - scrollTop - clientHeight < 10

          // Update gradients directly on the DOM — no React state update, no re-render
          const gradientOpacity = !showHeaderRef.current && scrollTop > 10 ? '1' : '0'
          if (topGradientRef.current) topGradientRef.current.style.opacity = gradientOpacity
          if (bottomGradientRef.current) bottomGradientRef.current.style.opacity = atBottom ? '0' : '1'

          // Show and reset idle timer if scrolled to the very top
          if (scrollTop <= 10) {
            setShowHeader(true)
            hiddenByIdleRef.current = false
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

          // Hide when scrolling down past threshold
          if (scrollDelta > 0 && scrollTop > 50 && !atBottom) {
            setShowHeader(false)
            hiddenByIdleRef.current = false
            if (idleTimerRef.current !== null) {
              window.clearTimeout(idleTimerRef.current)
              idleTimerRef.current = null
            }
          }
          // Show when scrolling up
          else if (scrollDelta < 0 && !atBottom) {
            setShowHeader(true)
            hiddenByIdleRef.current = false
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
  }, [setShowHeader, resetIdleTimer])

  // Mouse movement detection for idle timeout
  useEffect(() => {
    const scrollContainer = scrollContainerRef.current
    if (!scrollContainer) return

    const handleMouseMove = (e: MouseEvent) => {
      const rect = scrollContainer.getBoundingClientRect()
      const mouseY = e.clientY - rect.top

      if (mouseY >= 0 && mouseY <= 100 && !showHeaderRef.current) {
        setShowHeader(true)
        hiddenByIdleRef.current = false
        if (topGradientRef.current) topGradientRef.current.style.opacity = '0'
      }

      // Throttle idle timer resets to max once per 200ms to avoid setTimeout churn
      const now = Date.now()
      if (now - lastMouseMoveRef.current > 200) {
        lastMouseMoveRef.current = now
        resetIdleTimer()
      }
    }

    resetIdleTimer()
    scrollContainer.addEventListener('mousemove', handleMouseMove, { passive: true })

    return () => {
      scrollContainer.removeEventListener('mousemove', handleMouseMove)
      if (idleTimerRef.current !== null) {
        window.clearTimeout(idleTimerRef.current)
      }
    }
  }, [resetIdleTimer, setShowHeader])

  return (
    <div className="h-full flex flex-col overflow-hidden relative">
      {/* Back button - auto-hide on scroll, absolutely positioned */}
      <div
        className={`absolute top-0 left-0 right-0 bg-background z-10 transition-transform duration-300 mr-6 pb-2 ${
          showHeader ? 'translate-y-0' : '-translate-y-full'
        }`}
      >
        <button
          onClick={() => goBack(fallbackBackPath)}
          className="flex items-center gap-2 text-muted-foreground hover:opacity-[var(--hover-text-opacity)] transition-opacity duration-[var(--transition-duration)]"
        >
          <ArrowLeft className="w-4 h-4" />
          <span>{hasHistory ? t('common.back') : t(backLabelKey)}</span>
        </button>
      </div>

      {/* Scrollable Content — no transition on this container (padding-top transitions cause
          layout reflow of all children on every header toggle; snap instantly instead) */}
      <div
        ref={scrollContainerRef}
        className={`flex-1 overflow-y-auto pr-6 pb-6 scrollbar-custom ${
          showHeader ? 'pt-10' : 'pt-4'
        }`}
      >
        {/* Header content (cover art, info, etc.) */}
        <div className="mb-6">
          {header}
        </div>

        {/* Main content (track list, discography, etc.) */}
        {children}
      </div>

      {/* Top gradient overlay — opacity controlled directly via ref to avoid re-renders */}
      <div
        ref={topGradientRef}
        className="absolute top-0 left-0 right-0 h-24 pointer-events-none z-10 transition-opacity duration-300"
        style={{
          background: 'linear-gradient(to bottom, hsl(var(--background)) 0%, transparent 100%)',
          opacity: 0
        }}
      />

      {/* Bottom gradient overlay — opacity controlled directly via ref to avoid re-renders */}
      <div
        ref={bottomGradientRef}
        className="absolute bottom-0 left-0 right-0 h-24 pointer-events-none z-10 transition-opacity duration-300"
        style={{
          background: 'linear-gradient(to top, hsl(var(--background) / 0.75) 0%, transparent 100%)',
          opacity: 1
        }}
      />
    </div>
  )
}
