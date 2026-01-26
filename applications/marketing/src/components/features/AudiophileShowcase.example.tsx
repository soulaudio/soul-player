/**
 * Example usage of AudiophileShowcase component
 * This file demonstrates different integration patterns
 */

import { AudiophileShowcase } from './AudiophileShowcase'

// ============================================================
// Example 1: Basic Usage in Landing Page
// ============================================================
export function LandingPageExample() {
  return (
    <div>
      {/* Hero section */}
      <section className="h-screen">
        <h1>Soul Player</h1>
      </section>

      {/* Audiophile showcase section */}
      <AudiophileShowcase />

      {/* Other sections */}
      <section>
        <h2>More Features</h2>
      </section>
    </div>
  )
}

// ============================================================
// Example 2: Features Page with Multiple Showcases
// ============================================================
export function FeaturesPageExample() {
  return (
    <div className="space-y-24">
      {/* Library Management Section */}
      <section className="py-24">
        <h2>Powerful Library Management</h2>
        {/* Library showcase content */}
      </section>

      {/* Audio Quality Section */}
      <AudiophileShowcase />

      {/* Discovery Section */}
      <section className="py-24">
        <h2>Smart Music Discovery</h2>
        {/* Discovery showcase content */}
      </section>
    </div>
  )
}

// ============================================================
// Example 3: Next.js Page Integration
// ============================================================
export default function AudioQualityPage() {
  return (
    <main>
      {/* Page header */}
      <div className="container mx-auto px-6 pt-32 pb-16">
        <h1 className="text-6xl font-bold text-center mb-4">
          Audiophile-Grade Audio Engine
        </h1>
        <p className="text-xl text-center text-zinc-400 max-w-3xl mx-auto">
          Professional audio processing with bit-perfect playback,
          exclusive mode support, and transparent DSP effects.
        </p>
      </div>

      {/* Showcase component */}
      <AudiophileShowcase />

      {/* Additional technical details */}
      <section className="py-24">
        <div className="container mx-auto px-6">
          <h2 className="text-4xl font-bold text-center mb-12">
            Technical Specifications
          </h2>
          {/* Detailed specs grid */}
        </div>
      </section>
    </main>
  )
}

// ============================================================
// Example 4: Custom Wrapper with Animation Controls
// ============================================================
export function CustomAnimationExample() {
  return (
    <div className="relative">
      {/* Decorative background */}
      <div className="absolute inset-0 bg-gradient-to-b from-transparent via-purple-500/5 to-transparent" />

      {/* Showcase with custom spacing */}
      <div className="relative z-10 py-32">
        <AudiophileShowcase />
      </div>

      {/* Follow-up CTA */}
      <div className="text-center py-16">
        <button className="px-8 py-4 bg-purple-600 hover:bg-purple-700 rounded-lg text-white font-semibold">
          Try Soul Player Free
        </button>
      </div>
    </div>
  )
}

// ============================================================
// Example 5: Side-by-Side Comparison Layout
// ============================================================
export function ComparisonLayoutExample() {
  return (
    <div className="container mx-auto px-6 py-24">
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-12 items-center mb-24">
        {/* Left: Text content */}
        <div>
          <h2 className="text-4xl font-bold mb-6">
            Why Audio Quality Matters
          </h2>
          <div className="space-y-4 text-zinc-400">
            <p>
              Streaming services compress your music, losing subtle details
              and dynamic range that artists intended you to hear.
            </p>
            <p>
              Soul Player preserves every nuance with bit-perfect playback,
              ensuring you hear music exactly as it was mastered.
            </p>
            <ul className="space-y-2 ml-6">
              <li>✓ No lossy compression artifacts</li>
              <li>✓ Full dynamic range preservation</li>
              <li>✓ Native high-resolution support</li>
              <li>✓ Professional-grade DSP effects</li>
            </ul>
          </div>
        </div>

        {/* Right: Mini showcase preview */}
        <div className="bg-zinc-900 rounded-xl p-8 border border-zinc-800">
          <div className="space-y-4">
            <div className="bg-zinc-950 rounded-lg p-4">
              <div className="text-xs text-zinc-500 mb-2">Your File</div>
              <div className="text-2xl font-bold text-purple-400">DSD256</div>
              <div className="text-sm text-zinc-400">11.2 MHz, 1-bit</div>
            </div>
            <div className="flex items-center gap-2 text-green-400">
              <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clipRule="evenodd" />
              </svg>
              <span className="text-sm font-medium">Bit-Perfect Output</span>
            </div>
          </div>
        </div>
      </div>

      {/* Full showcase below */}
      <AudiophileShowcase />
    </div>
  )
}

// ============================================================
// Example 6: Mobile-Optimized Layout
// ============================================================
export function MobileOptimizedExample() {
  return (
    <div className="py-12 md:py-24">
      {/* Mobile: Stacked layout */}
      {/* Desktop: Full 3D showcase */}
      <div className="block md:hidden px-4">
        {/* Simplified mobile version */}
        <div className="bg-zinc-900 rounded-lg p-6 border border-zinc-800">
          <h3 className="text-xl font-bold mb-4">Audiophile Features</h3>
          <div className="space-y-3">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 bg-purple-500/10 rounded-lg flex items-center justify-center">
                <span className="text-purple-400">🎵</span>
              </div>
              <div>
                <div className="font-medium">Bit-Perfect</div>
                <div className="text-xs text-zinc-400">Lossless playback</div>
              </div>
            </div>
            {/* More features... */}
          </div>
        </div>
      </div>

      {/* Desktop: Full 3D showcase */}
      <div className="hidden md:block">
        <AudiophileShowcase />
      </div>
    </div>
  )
}

// ============================================================
// Usage Notes:
// ============================================================
// 1. Component is self-contained, no props required
// 2. Handles scroll-based 3D rotation automatically
// 3. All animations are GPU-accelerated
// 4. Responsive design works on all screen sizes
// 5. Can be used multiple times on same page (each tracks own scroll)
// 6. No external dependencies beyond standard React + Lucide icons
// 7. Theming via Tailwind CSS custom properties
