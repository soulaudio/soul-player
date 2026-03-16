/**
 * DetailPageLayout - Reusable layout for detail pages (album, artist, playlist)
 * Back button is always visible — does not auto-hide like library search bars.
 */

import { useEffect, ReactNode } from 'react'
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
  const { setShowHeader } = useScrollVisibility()

  // Ensure MainLayout top padding is correct when entering a detail page
  useEffect(() => {
    setShowHeader(true)
  }, [setShowHeader])

  return (
    <div className="h-full flex flex-col overflow-hidden relative">
      {/* Back button — always visible, no auto-hide */}
      <div className="bg-background z-10 pb-2 mr-6">
        <button
          onClick={() => goBack(fallbackBackPath)}
          className="flex items-center gap-2 text-muted-foreground hover:opacity-[var(--hover-text-opacity)] transition-opacity duration-[var(--transition-duration)]"
        >
          <ArrowLeft className="w-4 h-4" />
          <span>{hasHistory ? t('common.back') : t(backLabelKey)}</span>
        </button>
      </div>

      {/* Scrollable Content */}
      <div className="flex-1 overflow-y-auto pr-6 pb-6 scrollbar-custom">
        {/* Header content (cover art, info, etc.) */}
        <div className="mb-6">
          {header}
        </div>

        {/* Main content (track list, discography, etc.) */}
        {children}
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
