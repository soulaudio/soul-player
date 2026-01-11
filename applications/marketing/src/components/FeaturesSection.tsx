'use client'

import { useEffect, useState } from 'react'
import { Music, Layers, Lock, Wand2, Globe, Server } from 'lucide-react'

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

interface Feature {
  icon: typeof Layers
  titleKey: string
  descriptionKey: string
  technicalKey: string
  comingSoon?: boolean
  subscription?: boolean
}

const FEATURES: Feature[] = [
  {
    icon: Layers,
    titleKey: 'marketing.features.multiSource.title',
    descriptionKey: 'marketing.features.multiSource.description',
    technicalKey: 'marketing.features.multiSource.technical',
  },
  {
    icon: Music,
    titleKey: 'marketing.features.effects.title',
    descriptionKey: 'marketing.features.effects.description',
    technicalKey: 'marketing.features.effects.technical',
    comingSoon: true,
  },
  {
    icon: Server,
    titleKey: 'marketing.features.multiUser.title',
    descriptionKey: 'marketing.features.multiUser.description',
    technicalKey: 'marketing.features.multiUser.technical',
  },
  {
    icon: Globe,
    titleKey: 'marketing.features.crossPlatform.title',
    descriptionKey: 'marketing.features.crossPlatform.description',
    technicalKey: 'marketing.features.crossPlatform.technical',
  },
  {
    icon: Lock,
    titleKey: 'marketing.features.privacy.title',
    descriptionKey: 'marketing.features.privacy.description',
    technicalKey: 'marketing.features.privacy.technical',
  },
  {
    icon: Wand2,
    titleKey: 'marketing.features.discovery.title',
    descriptionKey: 'marketing.features.discovery.description',
    technicalKey: 'marketing.features.discovery.technical',
    comingSoon: false,
    subscription: true,
  },
]

export function FeaturesSection() {
  const { t } = useMarketingTranslation()

  return (
    <section className="py-24 bg-zinc-950">
      <div className="container mx-auto px-6">
        <div className="text-center mb-16">
          <h2 className="text-5xl font-serif font-bold mb-4">
            {t('marketing.features.title')}
          </h2>
          <p className="text-xl text-zinc-400 max-w-2xl mx-auto">
            {t('marketing.features.subtitle')}
          </p>
        </div>

        <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-8 max-w-6xl mx-auto">
          {FEATURES.map((feature, i) => (
            <div
              key={i}
              className="relative bg-zinc-900/50 border border-zinc-800 rounded-xl p-6 hover:border-violet-600/50 transition-all group"
            >
              {feature.comingSoon && (
                <span className="absolute top-4 right-4 text-xs font-mono text-violet-400 bg-violet-950/50 px-2 py-1 rounded">
                  {t('marketing.features.comingSoon')}
                </span>
              )}
              {feature.subscription && (
                <span className="absolute top-4 right-4 text-xs font-mono text-amber-400 bg-amber-950/50 px-2 py-1 rounded">
                  {t('marketing.features.optional')}
                </span>
              )}

              <feature.icon className="w-10 h-10 text-violet-400 mb-4" />

              <h3 className="text-xl font-bold mb-2">
                {t(feature.titleKey)}
              </h3>

              <p className="text-zinc-400 mb-4">
                {t(feature.descriptionKey)}
              </p>

              <details className="text-xs text-zinc-500 group">
                <summary className="cursor-pointer font-mono hover:text-violet-400 transition-colors">
                  {t('marketing.features.technicalDetails')} →
                </summary>
                <p className="mt-2 text-zinc-500 font-mono">
                  {t(feature.technicalKey)}
                </p>
              </details>
            </div>
          ))}
        </div>
      </div>
    </section>
  )
}
