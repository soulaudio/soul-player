'use client'

import React from 'react'
import { FadeIn } from './animations/FadeIn'
import { StreamingCritique } from './StreamingCritique'
import Link from 'next/link'

function PlannedButton() {
  return (
    <Link
      href="#subscribe"
      className="inline-flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-all hover:opacity-90"
      style={{
        backgroundColor: 'hsl(262 83% 58%)',
        color: 'white',
      }}
    >
      <svg className="w-4 h-4" fill="none" stroke="currentColor" strokeWidth={2} viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" d="M4.318 6.318a4.5 4.5 0 000 6.364L12 20.364l7.682-7.682a4.5 4.5 0 00-6.364-6.364L12 7.636l-1.318-1.318a4.5 4.5 0 00-6.364 0z" />
      </svg>
      Planned - Help us!
    </Link>
  )
}

function PlaceholderImage({ type }: { type: 'library' | 'users' | 'discover' | 'audio' | 'mobile' }) {
  const configs = {
    library: {
      icon: (
        <svg className="w-12 h-12 md:w-16 md:h-16" fill="none" stroke="currentColor" strokeWidth={1} viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" d="M9 9l10.5-3m0 6.553v3.75a2.25 2.25 0 01-1.632 2.163l-1.32.377a1.803 1.803 0 11-.99-3.467l2.31-.66a2.25 2.25 0 001.632-2.163zm0 0V2.25L9 5.25v10.303m0 0v3.75a2.25 2.25 0 01-1.632 2.163l-1.32.377a1.803 1.803 0 01-.99-3.467l2.31-.66A2.25 2.25 0 009 15.553z" />
        </svg>
      ),
    },
    users: {
      icon: (
        <svg className="w-12 h-12 md:w-16 md:h-16" fill="none" stroke="currentColor" strokeWidth={1} viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" d="M18 18.72a9.094 9.094 0 003.741-.479 3 3 0 00-4.682-2.72m.94 3.198l.001.031c0 .225-.012.447-.037.666A11.944 11.944 0 0112 21c-2.17 0-4.207-.576-5.963-1.584A6.062 6.062 0 016 18.719m12 0a5.971 5.971 0 00-.941-3.197m0 0A5.995 5.995 0 0012 12.75a5.995 5.995 0 00-5.058 2.772m0 0a3 3 0 00-4.681 2.72 8.986 8.986 0 003.74.477m.94-3.197a5.971 5.971 0 00-.94 3.197M15 6.75a3 3 0 11-6 0 3 3 0 016 0zm6 3a2.25 2.25 0 11-4.5 0 2.25 2.25 0 014.5 0zm-13.5 0a2.25 2.25 0 11-4.5 0 2.25 2.25 0 014.5 0z" />
        </svg>
      ),
    },
    discover: {
      icon: (
        <svg className="w-12 h-12 md:w-16 md:h-16" fill="none" stroke="currentColor" strokeWidth={1} viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" d="M12 21a9.004 9.004 0 008.716-6.747M12 21a9.004 9.004 0 01-8.716-6.747M12 21c2.485 0 4.5-4.03 4.5-9S14.485 3 12 3m0 18c-2.485 0-4.5-4.03-4.5-9S9.515 3 12 3m0 0a8.997 8.997 0 017.843 4.582M12 3a8.997 8.997 0 00-7.843 4.582m15.686 0A11.953 11.953 0 0112 10.5c-2.998 0-5.74-1.1-7.843-2.918m15.686 0A8.959 8.959 0 0121 12c0 .778-.099 1.533-.284 2.253m0 0A17.919 17.919 0 0112 16.5c-3.162 0-6.133-.815-8.716-2.247m0 0A9.015 9.015 0 013 12c0-1.605.42-3.113 1.157-4.418" />
        </svg>
      ),
    },
    audio: {
      icon: (
        <svg className="w-12 h-12 md:w-16 md:h-16" fill="none" stroke="currentColor" strokeWidth={1} viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" d="M9.348 14.651a3.75 3.75 0 010-5.303m5.304 0a3.75 3.75 0 010 5.303m-7.425 2.122a6.75 6.75 0 010-9.546m9.546 0a6.75 6.75 0 010 9.546M5.106 18.894c-3.808-3.808-3.808-9.98 0-13.789m13.788 0c3.808 3.808 3.808 9.981 0 13.79M12 12h.008v.007H12V12zm.375 0a.375.375 0 11-.75 0 .375.375 0 01.75 0z" />
        </svg>
      ),
    },
    mobile: {
      icon: (
        <svg className="w-12 h-12 md:w-16 md:h-16" fill="none" stroke="currentColor" strokeWidth={1} viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" d="M10.5 1.5H8.25A2.25 2.25 0 006 3.75v16.5a2.25 2.25 0 002.25 2.25h7.5A2.25 2.25 0 0018 20.25V3.75a2.25 2.25 0 00-2.25-2.25H13.5m-3 0V3h3V1.5m-3 0h3m-3 18.75h3" />
        </svg>
      ),
    },
  }

  const config = configs[type]

  return (
    <div
      className="relative w-full rounded-2xl overflow-hidden"
      style={{
        backgroundColor: 'hsl(var(--muted) / 0.3)',
        border: '1px solid hsl(var(--border))',
      }}
    >
      <div className="aspect-[16/10] flex flex-col items-center justify-center p-8 md:p-12">
        <div style={{ color: 'hsl(var(--muted-foreground) / 0.4)' }}>
          {config.icon}
        </div>
        <div className="mt-6 md:mt-8 space-y-2 w-full max-w-[240px]">
          <div className="h-2 rounded-full w-[90%]" style={{ backgroundColor: 'hsl(var(--muted-foreground) / 0.1)' }} />
          <div className="h-2 rounded-full w-[70%]" style={{ backgroundColor: 'hsl(var(--muted-foreground) / 0.1)' }} />
          <div className="h-2 rounded-full w-[80%]" style={{ backgroundColor: 'hsl(var(--muted-foreground) / 0.1)' }} />
        </div>
      </div>
    </div>
  )
}

function FeatureItem({ title, description, delay = 0 }: { title: string; description: string; delay?: number }) {
  return (
    <FadeIn delay={delay} direction="up">
      <li className="flex items-start gap-3">
        <div
          className="mt-2.5 w-1.5 h-1.5 rounded-full shrink-0"
          style={{ backgroundColor: 'hsl(var(--primary))' }}
        />
        <p className="text-base md:text-lg leading-relaxed" style={{ color: 'hsl(var(--muted-foreground))' }}>
          <span style={{ color: 'hsl(var(--foreground))' }} className="font-medium">
            {title}
          </span>{' '}
          {description}
        </p>
      </li>
    </FadeIn>
  )
}

interface FeatureSectionProps {
  title: string
  highlight: string
  description: string
  features: { title: string; description: string }[]
  imageType: 'library' | 'users' | 'discover' | 'audio' | 'mobile'
  imageFirst?: boolean
  extra?: React.ReactNode
  showPlannedButton?: boolean
}

function FeatureSection({
  title,
  highlight,
  description,
  features,
  imageType,
  imageFirst = false,
  extra,
  showPlannedButton = false
}: FeatureSectionProps) {
  const content = (
    <div className="flex flex-col">
      <FadeIn direction={imageFirst ? 'right' : 'left'}>
        <h3
          className="text-2xl sm:text-3xl md:text-4xl lg:text-5xl font-serif font-semibold tracking-tight leading-tight"
          style={{ color: 'hsl(var(--foreground))' }}
        >
          {title} <span style={{ color: 'hsl(var(--primary))' }}>{highlight}</span>
        </h3>
      </FadeIn>

      <FadeIn delay={0.1} direction={imageFirst ? 'right' : 'left'}>
        <p
          className="mt-6 text-lg md:text-xl leading-relaxed"
          style={{ color: 'hsl(var(--muted-foreground))' }}
        >
          {description}
        </p>
      </FadeIn>

      {showPlannedButton && (
        <FadeIn delay={0.15} direction={imageFirst ? 'right' : 'left'}>
          <div className="mt-6">
            <PlannedButton />
          </div>
        </FadeIn>
      )}

      {features.length > 0 && (
        <ul className="mt-8 space-y-4">
          {features.map((feature, i) => (
            <FeatureItem
              key={i}
              title={feature.title}
              description={feature.description}
              delay={0.2 + i * 0.05}
            />
          ))}
        </ul>
      )}

      {extra && (
        <FadeIn delay={0.4} direction="up">
          <div className="mt-8">{extra}</div>
        </FadeIn>
      )}
    </div>
  )

  const image = (
    <FadeIn direction={imageFirst ? 'left' : 'right'} delay={0.2}>
      <PlaceholderImage type={imageType} />
    </FadeIn>
  )

  return (
    <div className="grid grid-cols-1 lg:grid-cols-2 gap-12 md:gap-16 lg:gap-24 items-center">
      {imageFirst ? (
        <>
          <div className="order-2 lg:order-1">{image}</div>
          <div className="order-1 lg:order-2">{content}</div>
        </>
      ) : (
        <>
          <div>{content}</div>
          <div>{image}</div>
        </>
      )}
    </div>
  )
}

function SupportSection() {
  const backgroundRef = React.useRef<HTMLDivElement>(null)
  const cardRef = React.useRef<HTMLDivElement>(null)
  const animationRef = React.useRef<number | null>(null)
  const targetRef = React.useRef({ bgX: 0, bgY: 0, rotateX: 0, rotateY: 0 })
  const currentRef = React.useRef({ bgX: 0, bgY: 0, rotateX: 0, rotateY: 0 })

  // Smooth lerp animation
  React.useEffect(() => {
    const smoothing = 0.08 // Lower = smoother, higher = snappier

    const animate = () => {
      const target = targetRef.current
      const current = currentRef.current

      // Lerp current values toward target
      current.bgX += (target.bgX - current.bgX) * smoothing
      current.bgY += (target.bgY - current.bgY) * smoothing
      current.rotateX += (target.rotateX - current.rotateX) * smoothing
      current.rotateY += (target.rotateY - current.rotateY) * smoothing

      // Apply transforms
      if (backgroundRef.current) {
        backgroundRef.current.style.transform = `translate3d(${current.bgX}px, ${current.bgY}px, 0)`
      }
      if (cardRef.current) {
        cardRef.current.style.transform = `perspective(1000px) rotateX(${current.rotateX}deg) rotateY(${current.rotateY}deg)`
      }

      animationRef.current = requestAnimationFrame(animate)
    }

    animationRef.current = requestAnimationFrame(animate)

    return () => {
      if (animationRef.current) {
        cancelAnimationFrame(animationRef.current)
      }
    }
  }, [])

  const handleMouseMove = (e: React.MouseEvent<HTMLDivElement>) => {
    const rect = e.currentTarget.getBoundingClientRect()
    const x = e.clientX - rect.left
    const y = e.clientY - rect.top
    const centerX = rect.width / 2
    const centerY = rect.height / 2

    // Set target values (animation loop will smoothly interpolate)
    targetRef.current.rotateX = (y - centerY) / 500
    targetRef.current.rotateY = (centerX - x) / 500
    targetRef.current.bgX = -(x - centerX) / 25
    targetRef.current.bgY = -(y - centerY) / 25
  }

  const handleMouseLeave = () => {
    // Reset targets to zero (animation loop will smoothly return)
    targetRef.current = { bgX: 0, bgY: 0, rotateX: 0, rotateY: 0 }
  }

  const perks = [
    {
      title: 'Discord Special Channel',
      description: 'Direct access to developers and community',
      icon: (
        <svg className="w-5 h-5" viewBox="0 0 24 24" fill="currentColor">
          <path d="M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028 14.09 14.09 0 0 0 1.226-1.994.076.076 0 0 0-.041-.106 13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10.2 10.2 0 0 0 .372-.292.074.074 0 0 1 .077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.299 12.299 0 0 1-1.873.892.077.077 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.839 19.839 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.03zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418z"/>
        </svg>
      ),
    },
    {
      title: 'Early Access',
      description: 'Be first to try new features',
      icon: (
        <svg className="w-5 h-5" fill="none" stroke="currentColor" strokeWidth={1.5} viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" d="M15.59 14.37a6 6 0 01-5.84 7.38v-4.8m5.84-2.58a14.98 14.98 0 006.16-12.12A14.98 14.98 0 009.631 8.41m5.96 5.96a14.926 14.926 0 01-5.841 2.58m-.119-8.54a6 6 0 00-7.381 5.84h4.8m2.581-5.84a14.927 14.927 0 00-2.58 5.84m2.699 2.7c-.103.021-.207.041-.311.06a15.09 15.09 0 01-2.448-2.448 14.9 14.9 0 01.06-.312m-2.24 2.39a4.493 4.493 0 00-1.757 4.306 4.493 4.493 0 004.306-1.758M16.5 9a1.5 1.5 0 11-3 0 1.5 1.5 0 013 0z" />
        </svg>
      ),
    },
    {
      title: 'Discovery Features',
      description: 'Discogs, Bandcamp, AcoustID, MusicBrainz (once released)',
      icon: (
        <svg className="w-5 h-5" fill="none" stroke="currentColor" strokeWidth={1.5} viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" d="M12 21a9.004 9.004 0 008.716-6.747M12 21a9.004 9.004 0 01-8.716-6.747M12 21c2.485 0 4.5-4.03 4.5-9S14.485 3 12 3m0 18c-2.485 0-4.5-4.03-4.5-9S9.515 3 12 3m0 0a8.997 8.997 0 017.843 4.582M12 3a8.997 8.997 0 00-7.843 4.582m15.686 0A11.953 11.953 0 0112 10.5c-2.998 0-5.74-1.1-7.843-2.918m15.686 0A8.959 8.959 0 0121 12c0 .778-.099 1.533-.284 2.253m0 0A17.919 17.919 0 0112 16.5c-3.162 0-6.133-.815-8.716-2.247m0 0A9.015 9.015 0 013 12c0-1.605.42-3.113 1.157-4.418" />
        </svg>
      ),
    },
    {
      title: 'Feature Requests',
      description: 'Priority on your feature ideas',
      icon: (
        <svg className="w-5 h-5" fill="none" stroke="currentColor" strokeWidth={1.5} viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" d="M9.813 15.904L9 18.75l-.813-2.846a4.5 4.5 0 00-3.09-3.09L2.25 12l2.846-.813a4.5 4.5 0 003.09-3.09L9 5.25l.813 2.846a4.5 4.5 0 003.09 3.09L15.75 12l-2.846.813a4.5 4.5 0 00-3.09 3.09zM18.259 8.715L18 9.75l-.259-1.035a3.375 3.375 0 00-2.455-2.456L14.25 6l1.036-.259a3.375 3.375 0 002.455-2.456L18 2.25l.259 1.035a3.375 3.375 0 002.456 2.456L21.75 6l-1.035.259a3.375 3.375 0 00-2.456 2.456zM16.894 20.567L16.5 21.75l-.394-1.183a2.25 2.25 0 00-1.423-1.423L13.5 18.75l1.183-.394a2.25 2.25 0 001.423-1.423l.394-1.183.394 1.183a2.25 2.25 0 001.423 1.423l1.183.394-1.183.394a2.25 2.25 0 00-1.423 1.423z" />
        </svg>
      ),
    },
  ]

  const supports = [
    {
      title: 'Audiophile Features',
      description: 'Bit-perfect, WASAPI/ASIO, DSD, free for everyone once released',
      icon: (
        <svg className="w-5 h-5" fill="none" stroke="currentColor" strokeWidth={1.5} viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" d="M9.348 14.651a3.75 3.75 0 010-5.303m5.304 0a3.75 3.75 0 010 5.303m-7.425 2.122a6.75 6.75 0 010-9.546m9.546 0a6.75 6.75 0 010 9.546M5.106 18.894c-3.808-3.808-3.808-9.98 0-13.789m13.788 0c3.808 3.808 3.808 9.981 0 13.79M12 12h.008v.007H12V12zm.375 0a.375.375 0 11-.75 0 .375.375 0 01.75 0z" />
        </svg>
      ),
    },
    {
      title: 'Mobile Apps',
      description: 'iOS & Android development, free for everyone once released',
      icon: (
        <svg className="w-5 h-5" fill="none" stroke="currentColor" strokeWidth={1.5} viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" d="M10.5 1.5H8.25A2.25 2.25 0 006 3.75v16.5a2.25 2.25 0 002.25 2.25h7.5A2.25 2.25 0 0018 20.25V3.75a2.25 2.25 0 00-2.25-2.25H13.5m-3 0V3h3V1.5m-3 0h3m-3 18.75h3" />
        </svg>
      ),
    },
    {
      title: 'Physical DAP',
      description: 'E-Ink hardware player development',
      icon: (
        <svg className="w-5 h-5" fill="none" stroke="currentColor" strokeWidth={1.5} viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" d="M19.114 5.636a9 9 0 010 12.728M16.463 8.288a5.25 5.25 0 010 7.424M6.75 8.25l4.72-4.72a.75.75 0 011.28.53v15.88a.75.75 0 01-1.28.53l-4.72-4.72H4.51c-.88 0-1.704-.507-1.938-1.354A9.01 9.01 0 012.25 12c0-.83.112-1.633.322-2.396C2.806 8.756 3.63 8.25 4.51 8.25H6.75z" />
        </svg>
      ),
    },
    {
      title: 'Open Source Forever',
      description: 'Independent development, no corporate interests',
      icon: (
        <svg className="w-5 h-5" fill="none" stroke="currentColor" strokeWidth={1.5} viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" d="M21 8.25c0-2.485-2.099-4.5-4.688-4.5-1.935 0-3.597 1.126-4.312 2.733-.715-1.607-2.377-2.733-4.313-2.733C5.1 3.75 3 5.765 3 8.25c0 7.22 9 12 9 12s9-4.78 9-12z" />
        </svg>
      ),
    },
  ]

  return (
    <div
      id="subscribe"
      className="relative py-24 md:py-32 lg:py-40 overflow-hidden"
      style={{ backgroundColor: 'hsl(var(--background))' }}
    >
      {/* Grainy radial gradient - like hero section */}
      <div
        ref={backgroundRef}
        className="grain-visible absolute -inset-[10%] z-0 pointer-events-none"
        style={{
          background: 'radial-gradient(ellipse 120% 80% at 50% 50%, hsl(var(--primary) / 0.15) 0%, hsl(var(--primary) / 0.12) 30%, hsl(var(--primary) / 0.08) 50%, hsl(var(--primary) / 0.04) 65%, transparent 80%)',
          willChange: 'transform',
        }}
      />

      <div className="max-w-7xl mx-auto px-6 md:px-8 lg:px-12 relative z-10">
        <FadeIn direction="up">
          <div
            ref={cardRef}
            className="group relative p-8 md:p-12 lg:p-16 rounded-3xl overflow-hidden hover:scale-[1.02]"
            style={{
              backgroundColor: 'hsl(var(--card))',
              border: '1px solid hsl(var(--border))',
              boxShadow: '0 25px 50px -12px hsl(var(--primary) / 0.15)',
              transform: 'perspective(1000px)',
              transformStyle: 'preserve-3d',
              willChange: 'transform',
            }}
            onMouseMove={handleMouseMove}
            onMouseLeave={handleMouseLeave}
          >
            {/* Grainy gradient overlay */}
            <div
              className="grain-visible absolute inset-0 pointer-events-none rounded-3xl"
              style={{
                background: `radial-gradient(ellipse 80% 50% at 50% 0%, hsl(var(--primary) / 0.12) 0%, transparent 60%),
                            radial-gradient(ellipse 60% 40% at 100% 100%, hsl(var(--primary) / 0.08) 0%, transparent 50%)`,
              }}
            />

            {/* Subtle shine effect on hover */}
            <div
              className="absolute inset-0 opacity-0 group-hover:opacity-100 transition-opacity duration-700 pointer-events-none rounded-3xl"
              style={{
                background: 'linear-gradient(105deg, transparent 40%, hsl(var(--primary) / 0.02) 45%, hsl(var(--primary) / 0.04) 50%, hsl(var(--primary) / 0.02) 55%, transparent 60%)',
              }}
            />

            {/* Header row */}
            <div className="flex flex-col sm:flex-row justify-between items-start gap-4 mb-10">
              <div>
                <span
                  className="text-xs font-semibold uppercase tracking-wider"
                  style={{ color: 'hsl(var(--muted-foreground))' }}
                >
                  Community Support
                </span>
                <h2
                  className="mt-2 text-3xl sm:text-4xl md:text-5xl font-serif font-bold tracking-tight"
                  style={{ color: 'hsl(var(--foreground))' }}
                >
                  Take Music <span style={{ color: 'hsl(var(--primary))' }}>Back</span>
                </h2>
              </div>
              <div className="flex flex-col items-end gap-1">
                <div
                  className="px-4 py-2 rounded-full text-sm font-semibold"
                  style={{
                    backgroundColor: 'hsl(var(--primary) / 0.1)',
                    color: 'hsl(var(--primary))',
                  }}
                >
                  Starting at 5€/month
                </div>
                <span
                  className="text-xs"
                  style={{ color: 'hsl(var(--muted-foreground))' }}
                >
                  Name your price
                </span>
              </div>
            </div>

            {/* Description */}
            <p
              className="text-lg md:text-xl leading-relaxed mb-10 max-w-3xl"
              style={{ color: 'hsl(var(--muted-foreground))' }}
            >
              Soul Player is free and open source. Your support keeps it that way—funding development,
              infrastructure, and our mission to give you true music ownership.
            </p>

            {/* Two column layout */}
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-10 mb-12">
              {/* What you get */}
              <div>
                <h3
                  className="text-sm font-semibold uppercase tracking-wider mb-6"
                  style={{ color: 'hsl(var(--foreground))' }}
                >
                  What you get
                </h3>
                <div className="space-y-5">
                  {perks.map((perk, i) => (
                    <div key={i} className="flex items-start gap-3">
                      <div
                        className="shrink-0 mt-0.5"
                        style={{ color: 'hsl(var(--primary))' }}
                      >
                        {perk.icon}
                      </div>
                      <div>
                        <h4
                          className="font-semibold"
                          style={{ color: 'hsl(var(--foreground))' }}
                        >
                          {perk.title}
                        </h4>
                        <p
                          className="text-sm mt-1"
                          style={{ color: 'hsl(var(--muted-foreground))' }}
                        >
                          {perk.description}
                        </p>
                      </div>
                    </div>
                  ))}
                </div>
              </div>

              {/* What you support */}
              <div>
                <h3
                  className="text-sm font-semibold uppercase tracking-wider mb-6"
                  style={{ color: 'hsl(var(--foreground))' }}
                >
                  What you support
                </h3>
                <div className="space-y-5">
                  {supports.map((support, i) => (
                    <div key={i} className="flex items-start gap-3">
                      <div
                        className="shrink-0 mt-0.5"
                        style={{ color: 'hsl(var(--primary))' }}
                      >
                        {support.icon}
                      </div>
                      <div>
                        <h4
                          className="font-semibold"
                          style={{ color: 'hsl(var(--foreground))' }}
                        >
                          {support.title}
                        </h4>
                        <p
                          className="text-sm mt-1"
                          style={{ color: 'hsl(var(--muted-foreground))' }}
                        >
                          {support.description}
                        </p>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </div>

            {/* CTA row */}
            <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-6 pt-8" style={{ borderTop: '1px solid hsl(var(--border))' }}>
              <p
                className="text-sm"
                style={{ color: 'hsl(var(--muted-foreground) / 0.7)' }}
              >
                The core app remains free forever. Cancel anytime.
              </p>
              <div className="flex items-center gap-3">
                <Link
                  href="https://discord.gg/soulplayer"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="inline-flex items-center gap-2 py-3 px-6 rounded-lg text-sm font-semibold transition-all hover:opacity-90"
                  style={{
                    backgroundColor: 'hsl(var(--muted))',
                    color: 'hsl(var(--foreground))',
                  }}
                >
                  <svg className="w-5 h-5" viewBox="0 0 24 24" fill="currentColor">
                    <path d="M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028 14.09 14.09 0 0 0 1.226-1.994.076.076 0 0 0-.041-.106 13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10.2 10.2 0 0 0 .372-.292.074.074 0 0 1 .077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.299 12.299 0 0 1-1.873.892.077.077 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.839 19.839 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.03zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418z"/>
                  </svg>
                  Join Discord
                </Link>
                <button
                  type="button"
                  className="inline-flex items-center gap-2 py-3 px-8 rounded-lg text-sm font-semibold transition-all hover:opacity-90"
                  style={{
                    backgroundColor: 'hsl(var(--primary))',
                    color: 'hsl(var(--primary-foreground))',
                  }}
                >
                  Become a Supporter
                </button>
              </div>
            </div>
          </div>
        </FadeIn>
      </div>
    </div>
  )
}

export function WhySoulPlayer() {
  return (
    <section
      className="relative transition-colors"
      style={{ backgroundColor: 'hsl(var(--background))' }}
    >
      {/* Hero Title */}
      <div className="pt-24 pb-8 md:pt-32 md:pb-10 lg:pt-40 lg:pb-12">
        <div className="max-w-7xl mx-auto px-6 md:px-8 lg:px-12">
          <FadeIn>
            <h2
              className="text-4xl sm:text-5xl md:text-6xl lg:text-7xl xl:text-8xl font-serif font-bold text-center tracking-tight"
              style={{ color: 'hsl(var(--foreground))' }}
            >
              Why Soul Player?
            </h2>
          </FadeIn>
        </div>
      </div>

      {/* Streaming Critique Section */}
      <StreamingCritique />

      {/* Section 1: Actually YOUR Music */}
      <div className="py-16 md:py-24 lg:py-32">
        <div className="max-w-7xl mx-auto px-6 md:px-8 lg:px-12">
          <FeatureSection
            title="Actually"
            highlight="YOUR Music"
            description="Your files. Your hardware. Your rules. No corporation deciding what you can play, when you can play it, or tracking every song you listen to."
            features={[
              { title: 'Local first.', description: 'Your music library lives on your machine. Fast, reliable, always available—even offline.' },
              { title: 'Privacy protected.', description: 'Zero telemetry. Zero tracking. Your listening habits are yours alone.' },
              { title: 'Open source.', description: 'Fully transparent code you can audit, modify, or fork.' },
              { title: 'Free forever.', description: 'No subscriptions required for the core experience.' },
              { title: 'Cross-platform.', description: 'Windows, macOS, Linux desktop apps available now.' },
            ]}
            imageType="library"
          />
        </div>
      </div>

      {/* Section 2: Don't Listen Alone */}
      <div className="py-16 md:py-24 lg:py-32">
        <div className="max-w-7xl mx-auto px-6 md:px-8 lg:px-12">
          <FeatureSection
            title="Don't Listen"
            highlight="Alone"
            description="Music is better shared. Build your library together with family, friends, or your entire household."
            features={[
              { title: 'Multi-user support.', description: 'Separate profiles with individual playlists and preferences.' },
              { title: 'Self-hosted server.', description: 'Run your own music server. Stream anywhere, securely.' },
              { title: 'Share everything.', description: 'Collaborate on playlists, share discoveries, enjoy together.' },
            ]}
            imageType="users"
            imageFirst
          />
        </div>
      </div>

      {/* Section 3: Actually Discover Music */}
      <div className="py-16 md:py-24 lg:py-32">
        <div className="max-w-7xl mx-auto px-6 md:px-8 lg:px-12">
          <FeatureSection
            title="Actually"
            highlight="Discover Music"
            showPlannedButton
            description="No weird algorithms pushing whatever pays the most. No 'discover weekly' that's really just label promotions."
            features={[
              { title: 'No algorithmic manipulation.', description: "We don't decide what you hear. You do." },
              { title: 'No paid promotions.', description: 'What you see is based on music, not marketing.' },
              { title: 'Discogs & Bandcamp.', description: 'Browse and discover through real music communities.' },
              { title: 'AcoustID fingerprinting.', description: 'Identify unknown tracks in your library automatically.' },
              { title: 'MusicBrainz metadata.', description: 'Rich, community-curated music information.' },
              { title: 'ListenBrainz scrobbling.', description: 'Track your listening history on your terms.' },
            ]}
            imageType="discover"
            extra={
              <div
                className="p-6 rounded-xl"
                style={{
                  backgroundColor: 'hsl(262 83% 58% / 0.1)',
                  border: '1px solid hsl(262 83% 58% / 0.3)',
                }}
              >
                <p
                  className="text-sm md:text-base leading-relaxed"
                  style={{ color: 'hsl(var(--foreground))' }}
                >
                  <strong>Building this costs money.</strong> Integrating with external APIs, maintaining metadata services,
                  and building discovery features require ongoing investment. Your community support makes it possible.
                </p>
                <Link
                  href="#subscribe"
                  className="inline-flex items-center gap-2 mt-4 text-sm font-semibold transition-opacity hover:opacity-80"
                  style={{ color: 'hsl(262 83% 58%)' }}
                >
                  Support development
                  <svg className="w-4 h-4" fill="none" stroke="currentColor" strokeWidth={2} viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" d="M13.5 4.5L21 12m0 0l-7.5 7.5M21 12H3" />
                  </svg>
                </Link>
              </div>
            }
          />
        </div>
      </div>

      {/* Section 4: Ready for Audiophiles */}
      <div className="py-16 md:py-24 lg:py-32">
        <div className="max-w-7xl mx-auto px-6 md:px-8 lg:px-12">
          <FeatureSection
            title="Ready for"
            highlight="Audiophiles"
            showPlannedButton
            description="An audio engine built for those who care about sound quality. Not 'good enough'—exceptional."
            features={[
              { title: 'Bit-perfect playback.', description: 'No resampling, no processing—pure audio as intended.' },
              { title: 'Gapless & crossfade.', description: 'Seamless transitions between tracks.' },
              { title: 'Parametric EQ.', description: 'Fine-tune your sound with precision controls.' },
              { title: 'ReplayGain support.', description: 'Consistent volume across your entire library.' },
              { title: 'WASAPI & ASIO.', description: 'Exclusive mode output for bit-perfect streaming on Windows.' },
              { title: 'High-res audio.', description: 'Native support for 24-bit/192kHz and beyond.' },
              { title: 'DSD playback.', description: 'Native DSD streaming for SACD enthusiasts.' },
              { title: 'Exclusive mode.', description: 'Bypass system mixer for purest signal path.' },
            ]}
            imageType="audio"
            imageFirst
            extra={
              <div
                className="p-6 rounded-xl"
                style={{
                  backgroundColor: 'hsl(262 83% 58% / 0.1)',
                  border: '1px solid hsl(262 83% 58% / 0.3)',
                }}
              >
                <p
                  className="text-sm md:text-base leading-relaxed"
                  style={{ color: 'hsl(var(--foreground))' }}
                >
                  <strong>Building this costs money.</strong> Professional audio engineering, low-level optimizations,
                  and hardware testing require specialized expertise. Your community support makes it possible.
                </p>
                <Link
                  href="#subscribe"
                  className="inline-flex items-center gap-2 mt-4 text-sm font-semibold transition-opacity hover:opacity-80"
                  style={{ color: 'hsl(262 83% 58%)' }}
                >
                  Support development
                  <svg className="w-4 h-4" fill="none" stroke="currentColor" strokeWidth={2} viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" d="M13.5 4.5L21 12m0 0l-7.5 7.5M21 12H3" />
                  </svg>
                </Link>
              </div>
            }
          />
        </div>
      </div>

      {/* Section 5: Listen on the Go */}
      <div className="py-16 md:py-24 lg:py-32">
        <div className="max-w-7xl mx-auto px-6 md:px-8 lg:px-12">
          <FeatureSection
            title="Listen on"
            highlight="the Go"
            showPlannedButton
            description="Your music, everywhere. Native mobile apps and dedicated hardware—all connected to your Soul Player ecosystem."
            features={[
              { title: 'iOS & Android apps.', description: 'Native mobile apps synced with your library and server.' },
              { title: 'Offline sync.', description: 'Download playlists for airplane mode and data-free listening.' },
              { title: 'Physical DAP.', description: 'E-Ink digital audio player—dedicated hardware for purists.' },
              { title: 'Unified ecosystem.', description: 'Same library, same playlists, same experience everywhere.' },
            ]}
            imageType="mobile"
            extra={
              <div
                className="p-6 rounded-xl"
                style={{
                  backgroundColor: 'hsl(262 83% 58% / 0.1)',
                  border: '1px solid hsl(262 83% 58% / 0.3)',
                }}
              >
                <p
                  className="text-sm md:text-base leading-relaxed"
                  style={{ color: 'hsl(var(--foreground))' }}
                >
                  <strong>Building this costs money.</strong> Mobile development, hardware prototyping,
                  and manufacturing require significant investment. Your community support makes it possible.
                </p>
                <Link
                  href="#subscribe"
                  className="inline-flex items-center gap-2 mt-4 text-sm font-semibold transition-opacity hover:opacity-80"
                  style={{ color: 'hsl(262 83% 58%)' }}
                >
                  Support development
                  <svg className="w-4 h-4" fill="none" stroke="currentColor" strokeWidth={2} viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" d="M13.5 4.5L21 12m0 0l-7.5 7.5M21 12H3" />
                  </svg>
                </Link>
              </div>
            }
          />
        </div>
      </div>

      {/* Support Our Journey - Community Tier */}
      <SupportSection />
    </section>
  )
}
