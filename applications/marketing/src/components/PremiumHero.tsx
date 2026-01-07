'use client'

import { DownloadButton } from './DownloadButton'
import { DemoModeWrapper } from './DemoModeWrapper'
import { ParallaxBranding } from './ParallaxBranding'
import { RotatingText } from './RotatingText'
import { DemoApp } from './demo/DemoApp'
import { DemoThemeSwitcher } from './demo/DemoThemeSwitcher'
import { DemoScaler } from './demo/DemoScaler'

export function PremiumHero() {
  return (
    <section data-hero-section className="relative min-h-screen flex flex-col items-center justify-center overflow-hidden pt-32 pb-20 transition-colors duration-700" style={{ backgroundColor: 'hsl(250, 15%, 4%)' }}>
      {/* Solid background layer */}
      <div className="absolute inset-0 -z-20 transition-colors duration-700" style={{ backgroundColor: 'inherit' }} />

      {/* Grainy radial gradient - centered on demo area */}
      <div
        data-demo-backdrop
        className="grain-visible absolute inset-0 z-0 transition-all duration-700"
        style={{
          background: 'radial-gradient(ellipse 120% 80% at 50% 65%, rgba(88, 50, 180, 0.15) 0%, rgba(75, 40, 160, 0.12) 30%, rgba(60, 30, 140, 0.08) 50%, rgba(45, 20, 100, 0.04) 65%, transparent 80%)',
        }}
      />

      {/* Content container */}
      <div className="relative z-10 container mx-auto px-6 py-20">
        {/* Hero header section */}
        <header className="text-center mb-16 max-w-6xl mx-auto px-4">
          {/* Main heading */}
          <h1 data-main-text className="text-2xl sm:text-3xl md:text-4xl lg:text-5xl xl:text-6xl text-zinc-300 mb-6 font-serif leading-relaxed transition-colors duration-700">
            Not just another music player,
            <br />
            <span
              data-heading-gradient
              className="bg-clip-text bg-gradient-to-r from-violet-600 to-violet-700 transition-all duration-700"
              style={{
                WebkitBackgroundClip: 'text',
                WebkitTextFillColor: 'transparent',
                backgroundClip: 'text',
                color: 'transparent'
              }}
            >
              a new way to <RotatingText /> your music
            </span>
          </h1>

          {/* Description */}
          <div data-desc-text className="mb-8 space-y-1 text-sm sm:text-base md:text-lg text-zinc-400 leading-relaxed transition-colors duration-700">
            <p>Local-first music player, optional self-hosted multi-user streaming server,</p>
            <p>optional paid* discovery</p>
          </div>

          {/* Download CTA */}
          <div className="mb-6">
            <DownloadButton />
          </div>

          {/* Feature badges */}
          <div data-badge-text className="flex items-center justify-center gap-4 text-xs text-zinc-400 transition-colors duration-700">
            <div className="flex items-center gap-1.5">
              <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
              <span>No subscriptions</span>
            </div>
            <div className="w-1 h-1 rounded-full bg-zinc-700" />
            <div className="flex items-center gap-1.5">
              <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
              <span>No tracking</span>
            </div>
            <div className="w-1 h-1 rounded-full bg-zinc-700" />
            <div className="flex items-center gap-1.5">
              <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
              <span>No ads</span>
            </div>
          </div>
        </header>

        {/* Demo showcase */}
        <div className="relative mt-16 animate-fade-in-delay-500 max-w-7xl mx-auto">
          {/* Theme switcher - above demo on the left */}
          <div className="flex justify-start mb-4">
            <div className="flex flex-col items-start gap-2">
              <span data-theme-label className="text-xs text-zinc-400 tracking-wide transition-colors duration-700">Pick your theme</span>
              <DemoThemeSwitcher />
            </div>
          </div>

          {/* Demo container */}
          <div className="relative px-2.5 sm:px-0">
            <div className="rounded-lg sm:rounded-2xl overflow-hidden border border-zinc-800/50 shadow-2xl backdrop-blur-sm bg-zinc-900/30">
              <DemoModeWrapper className="w-full aspect-[16/10]">
                <DemoScaler designWidth={1200} designHeight={750} minScale={0.25}>
                  <DemoApp />
                </DemoScaler>
              </DemoModeWrapper>
            </div>

            {/* Parallax branding - bottom right of demo */}
            <ParallaxBranding />

            {/* Decorative blur elements */}
            <div className="absolute -top-4 -left-4 w-24 h-24 bg-violet-500/20 rounded-full blur-2xl" />
            <div className="absolute -bottom-4 -right-4 w-32 h-32 bg-purple-500/20 rounded-full blur-3xl" />
          </div>
        </div>
      </div>
    </section>
  )
}
