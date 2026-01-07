'use client'

import { DownloadButton } from './DownloadButton'
import { DemoModeWrapper } from './DemoModeWrapper'
import { ParallaxBranding } from './ParallaxBranding'
import { RotatingText } from './RotatingText'
import { DemoApp } from './demo/DemoApp'
import { DemoThemeSwitcher } from './demo/DemoThemeSwitcher'
import { DemoScaler } from './demo/DemoScaler'
import { InteractiveBadge } from './demo/InteractiveBadge'

export function PremiumHero() {
  return (
    <section data-hero-section className="relative min-h-screen flex flex-col items-center justify-center overflow-hidden pt-32 pb-20 transition-colors duration-700 bg-background">
      {/* Solid background layer */}
      <div className="absolute inset-0 -z-20 transition-colors duration-700 bg-background" />

      {/* Grainy radial gradient - centered on demo area */}
      <div
        data-demo-backdrop
        className="grain-visible absolute inset-0 z-0 transition-all duration-700"
        style={{
          background: 'radial-gradient(ellipse 120% 80% at 50% 65%, hsl(var(--primary) / 0.15) 0%, hsl(var(--primary) / 0.12) 30%, hsl(var(--primary) / 0.08) 50%, hsl(var(--primary) / 0.04) 65%, transparent 80%)',
        }}
      />

      {/* Content container */}
      <div className="relative z-10 container mx-auto px-6 py-20">
        {/* Hero header section */}
        <header className="text-center mb-16 max-w-6xl mx-auto px-4">
          {/* Main heading */}
          <h1 data-main-text className="text-2xl sm:text-3xl md:text-4xl lg:text-5xl xl:text-6xl mb-6 font-serif leading-relaxed transition-colors duration-700 text-foreground">
            Not just another music player,
            <br />
            <span
              data-heading-gradient
              className="text-transparent bg-clip-text transition-all duration-700"
              style={{
                backgroundImage: 'linear-gradient(135deg, hsl(var(--primary)) 0%, color-mix(in srgb, hsl(var(--primary)) 30%, hsl(var(--foreground)) 70%) 30%, hsl(var(--foreground)) 50%, color-mix(in srgb, hsl(var(--foreground)) 70%, hsl(var(--accent)) 30%) 70%, color-mix(in srgb, hsl(var(--foreground)) 60%, hsl(var(--accent)) 40%) 100%)',
                WebkitBackgroundClip: 'text',
                WebkitTextFillColor: 'transparent',
              }}
            >
              a new way to <RotatingText /> your music
            </span>
          </h1>

          {/* Description */}
          <div data-desc-text className="mb-8 space-y-1 text-sm sm:text-base md:text-lg leading-relaxed transition-colors duration-700 text-muted-foreground">
            <p>Local-first music player, optional self-hosted multi-user streaming server,</p>
            <p>optional paid* discovery</p>
          </div>

          {/* Download CTA */}
          <div>
            <DownloadButton />
          </div>
        </header>

        {/* Demo showcase */}
        <div className="relative mt-16 animate-fade-in-delay-500 max-w-7xl mx-auto">
          {/* Theme switcher and interactive badge - above demo */}
          <div className="flex justify-between items-center mb-4">
            <div className="flex flex-col items-start gap-2">
              <span data-theme-label className="text-xs tracking-wide transition-colors duration-700 text-muted-foreground">Pick your theme</span>
              <DemoThemeSwitcher />
            </div>
            <InteractiveBadge />
          </div>

          {/* Demo container */}
          <div className="relative px-2.5 sm:px-0">
            {/* Decorative glow elements - above demo */}
            <div
              className="absolute -top-24 left-1/4 w-48 h-48 rounded-full blur-3xl transition-colors duration-700 pointer-events-none"
              style={{ background: 'hsl(var(--primary) / 0.25)' }}
            />
            <div
              className="absolute -top-32 right-1/3 w-64 h-64 rounded-full blur-3xl transition-colors duration-700 pointer-events-none"
              style={{ background: 'hsl(var(--primary) / 0.2)' }}
            />

            <div className="rounded-lg sm:rounded-2xl overflow-hidden border shadow-2xl backdrop-blur-sm transition-colors duration-700" style={{ borderColor: 'hsl(var(--border))', backgroundColor: 'hsl(var(--card) / 0.3)' }}>
              <DemoModeWrapper interactive={true} className="w-full aspect-[16/10]">
                <DemoScaler designWidth={1200} designHeight={750} minScale={0.25}>
                  <DemoApp />
                </DemoScaler>
              </DemoModeWrapper>
            </div>

            {/* Parallax branding - bottom right of demo */}
            <ParallaxBranding />
          </div>
        </div>
      </div>
    </section>
  )
}
