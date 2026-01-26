/**
 * Example usage of LocalFirstShowcase component
 *
 * This file demonstrates how to integrate the LocalFirstShowcase
 * into your marketing pages.
 */

import { LocalFirstShowcase } from './LocalFirstShowcase'

/**
 * Example 1: Basic usage in a landing page
 */
export function BasicExample() {
  return (
    <div>
      <section className="py-20 bg-background">
        <div className="container mx-auto px-6 text-center">
          <h1 className="text-5xl font-bold mb-4">Welcome to Soul Player</h1>
          <p className="text-xl text-muted-foreground mb-8">
            The music player that respects your privacy
          </p>
        </div>
      </section>

      <LocalFirstShowcase />

      <section className="py-20 bg-muted">
        <div className="container mx-auto px-6 text-center">
          <h2 className="text-4xl font-bold mb-4">More Features</h2>
          {/* Additional content */}
        </div>
      </section>
    </div>
  )
}

/**
 * Example 2: Integration in features page
 */
export function FeaturesPageExample() {
  return (
    <div className="space-y-0">
      {/* Hero section */}
      <section className="min-h-screen flex items-center justify-center">
        <div className="text-center">
          <h1 className="text-6xl font-bold">Features</h1>
        </div>
      </section>

      {/* Privacy feature showcase */}
      <LocalFirstShowcase />

      {/* Other feature sections */}
      <section className="py-24 bg-background">
        <div className="container mx-auto px-6">
          <h2 className="text-4xl font-bold text-center mb-12">
            Audiophile-Grade Quality
          </h2>
          {/* Audio features */}
        </div>
      </section>

      <section className="py-24 bg-muted">
        <div className="container mx-auto px-6">
          <h2 className="text-4xl font-bold text-center mb-12">
            Multi-Platform Streaming
          </h2>
          {/* Streaming features */}
        </div>
      </section>
    </div>
  )
}

/**
 * Example 3: Standalone privacy page
 */
export function PrivacyPageExample() {
  return (
    <div>
      <section className="py-20 bg-background">
        <div className="container mx-auto px-6 max-w-4xl">
          <h1 className="text-5xl font-bold mb-8">Privacy First</h1>
          <div className="prose prose-lg text-muted-foreground">
            <p>
              Soul Player is built from the ground up with privacy as a core principle.
              We believe your music listening habits are personal and should stay that way.
            </p>
            <p>
              Unlike streaming services that track every song you play, analyze your
              listening patterns, and sell your data to advertisers, Soul Player keeps
              everything local and private.
            </p>
          </div>
        </div>
      </section>

      <LocalFirstShowcase />

      <section className="py-20 bg-muted">
        <div className="container mx-auto px-6 max-w-4xl">
          <h2 className="text-4xl font-bold mb-8">Our Privacy Commitments</h2>
          <div className="space-y-6">
            <div className="flex gap-4">
              <div className="w-12 h-12 rounded-full bg-green-500/10 flex items-center justify-center flex-shrink-0">
                <span className="text-green-500 text-xl">✓</span>
              </div>
              <div>
                <h3 className="text-xl font-bold mb-2">No Analytics</h3>
                <p className="text-muted-foreground">
                  We don't track what you listen to, when you listen, or how often.
                </p>
              </div>
            </div>
            {/* More commitments */}
          </div>
        </div>
      </section>
    </div>
  )
}

/**
 * Tips for using LocalFirstShowcase:
 *
 * 1. Place it in the middle of your page flow for maximum scroll impact
 * 2. Pair it with sections that have contrasting backgrounds
 * 3. The 3D rotation effect works best when users scroll slowly
 * 4. Consider adding scroll hints if placed early in the page
 * 5. The component is self-contained and requires no props
 */
