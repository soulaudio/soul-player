/**
 * DetailPageLayout - Reusable layout for detail pages (album, artist, playlist)
 * Back navigation is handled by the title bar back button.
 */

import { useEffect, ReactNode } from 'react'
import { useScrollVisibility } from '../contexts/ScrollVisibilityContext'

interface DetailPageLayoutProps {
  /** Content above the main list (artist avatar, album cover, info, etc.) */
  header: ReactNode
  /** Main scrollable content (track lists, discography, etc.) */
  children: ReactNode
}

export function DetailPageLayout({
  header,
  children,
}: DetailPageLayoutProps) {
  const { setShowHeader } = useScrollVisibility()

  // Ensure MainLayout top padding is correct when entering a detail page
  useEffect(() => {
    setShowHeader(true)
  }, [setShowHeader])

  return (
    <div className="h-full flex flex-col overflow-hidden relative">
      {/* Scrollable Content */}
      <div className="flex-1 overflow-y-auto pr-0 scrollbar-custom">
        <div className="pr-4 sm:pr-6 pt-8 pb-20 sm:pb-6">
          {/* Header content (cover art, info, etc.) */}
          <div className="mb-6">
            {header}
          </div>

          {/* Main content (track list, discography, etc.) */}
          {children}
        </div>
      </div>

      {/* Bottom gradient overlay */}
      <div
        className="absolute bottom-0 left-0 right-0 h-24 pointer-events-none z-10"
        style={{
          background: 'linear-gradient(to top, hsl(var(--background) / 0.75) 0%, transparent 100%)',
        }}
      />
    </div>
  )
}
