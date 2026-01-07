'use client'

import { DemoModeWrapper } from './DemoModeWrapper'

/**
 * Placeholder demo player - will be replaced with actual shared components
 * This shows the layout structure that will showcase the real Soul Player UI
 */
export function DemoShowcase() {
  return (
    <section id="demo" className="py-24 bg-zinc-950">
      <div className="container mx-auto px-6">
        <div className="text-center mb-16">
          <h2 className="text-5xl font-serif font-bold mb-4">
            Experience Soul Player
          </h2>
          <p className="text-xl text-zinc-400">
            A glimpse at the interface across all platforms
          </p>
        </div>

        <div className="max-w-6xl mx-auto">
          {/* Demo player container - will import from @soul-player/shared */}
          <DemoModeWrapper className="rounded-2xl overflow-hidden shadow-2xl border border-zinc-800">
            <div className="bg-gradient-to-br from-zinc-900 to-zinc-950 p-8">
              {/* Placeholder for actual player component */}
              <div className="aspect-video bg-zinc-800 rounded-lg flex items-center justify-center border border-zinc-700">
                <div className="text-center">
                  <div className="text-6xl mb-4">🎵</div>
                  <p className="text-zinc-400 font-mono text-sm">
                    Import actual player components from @soul-player/shared
                  </p>
                  <p className="text-zinc-500 text-xs mt-2">
                    DemoModeWrapper makes them non-interactive
                  </p>
                </div>
              </div>

              {/* Mock controls to show layout structure */}
              <div className="mt-6 flex items-center justify-between">
                <div className="flex items-center gap-4">
                  <div className="w-12 h-12 bg-zinc-800 rounded" />
                  <div>
                    <div className="h-4 w-32 bg-zinc-800 rounded mb-2" />
                    <div className="h-3 w-24 bg-zinc-800 rounded" />
                  </div>
                </div>

                <div className="flex items-center gap-4">
                  <div className="w-8 h-8 bg-zinc-800 rounded-full" />
                  <div className="w-10 h-10 bg-violet-600 rounded-full" />
                  <div className="w-8 h-8 bg-zinc-800 rounded-full" />
                </div>

                <div className="w-32 h-4 bg-zinc-800 rounded" />
              </div>
            </div>
          </DemoModeWrapper>

          <p className="text-center text-zinc-500 mt-8 text-sm">
            Non-interactive preview • Real UI components from the app
          </p>
        </div>
      </div>
    </section>
  )
}
