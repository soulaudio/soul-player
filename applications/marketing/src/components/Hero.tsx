'use client'

import { useEffect, useState } from 'react'
import { DownloadButton } from './DownloadButton'
import Link from 'next/link'

// Marketing pages use dynamic import to avoid SSR issues with i18n
function useMarketingTranslation() {
  const [t, setT] = useState<(key: string) => string>(() => (key: string) => key)

  useEffect(() => {
    import('@soul-player/shared/i18n').then(({ initI18n }) => {
      const i18n = initI18n()
      setT(() => (key: string) => i18n.t(key) || key)
    })
  }, [])

  return { t }
}

export function Hero() {
  const { t } = useMarketingTranslation()

  return (
    <div
      className="relative overflow-hidden min-h-screen flex items-center justify-center bg-background"
    >
      {/* Grainy radial gradient backdrop */}
      <div
        className="grain-visible absolute inset-0 z-0"
        style={{
          background: 'radial-gradient(ellipse 100% 80% at 50% 50%, hsl(var(--primary) / 0.12) 0%, hsl(var(--primary) / 0.08) 40%, hsl(var(--primary) / 0.03) 70%, transparent 90%)',
        }}
      />
      <div className="relative z-10 w-full">
      <div className="container mx-auto px-4 sm:px-6 py-12 sm:py-16 md:py-24 text-center">
        <h1 className="font-serif font-extrabold mb-4 sm:mb-6 tracking-tight whitespace-nowrap text-foreground" style={{ fontSize: 'clamp(1.5rem, 8vw, 6rem)' }}>
          {t('marketing.hero.title1')}
          <br />
          <span
            className="text-transparent bg-clip-text"
            style={{
              backgroundImage: 'linear-gradient(135deg, hsl(var(--primary)) 0%, color-mix(in srgb, hsl(var(--primary)) 30%, hsl(var(--foreground)) 70%) 30%, hsl(var(--foreground)) 50%, color-mix(in srgb, hsl(var(--foreground)) 70%, hsl(var(--accent)) 30%) 70%, color-mix(in srgb, hsl(var(--foreground)) 60%, hsl(var(--accent)) 40%) 100%)',
              WebkitBackgroundClip: 'text',
              WebkitTextFillColor: 'transparent',
            }}
          >
            {t('marketing.hero.title2')}
          </span>
        </h1>

        <p className="text-base sm:text-lg md:text-xl lg:text-2xl max-w-3xl mx-auto mb-8 sm:mb-12 leading-relaxed px-4 text-muted-foreground whitespace-pre-line">
          {t('marketing.hero.subtitle')}
        </p>

        <div className="flex flex-col items-center gap-4 sm:gap-6 mb-6 sm:mb-8">
          <DownloadButton />

          <Link
            href="#demo"
            className="transition-colors text-xs sm:text-sm text-muted-foreground hover:text-foreground"
          >
            {t('marketing.hero.seeItInAction')} ↓
          </Link>
        </div>

        <div className="mt-12 sm:mt-16 md:mt-24 grid grid-cols-1 sm:grid-cols-3 gap-6 sm:gap-8 max-w-3xl mx-auto text-sm px-2">
          <div>
            <div className="text-2xl sm:text-3xl font-bold mb-2 text-foreground">{t('marketing.hero.crossPlatform')}</div>
            <div className="text-xs sm:text-sm text-muted-foreground">{t('marketing.hero.crossPlatformDesc')}</div>
          </div>
          <div>
            <div className="text-2xl sm:text-3xl font-bold mb-2 text-foreground">{t('marketing.hero.privacyFirst')}</div>
            <div className="text-xs sm:text-sm text-muted-foreground">{t('marketing.hero.privacyFirstDesc')}</div>
          </div>
          <div>
            <div className="text-2xl sm:text-3xl font-bold mb-2 text-foreground">{t('marketing.hero.openSource')}</div>
            <div className="text-xs sm:text-sm text-muted-foreground">{t('marketing.hero.openSourceDesc')}</div>
          </div>
        </div>
      </div>
      </div>
    </div>
  )
}
