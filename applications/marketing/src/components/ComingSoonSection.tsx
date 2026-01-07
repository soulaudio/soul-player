'use client'

import { Smartphone, Radio } from 'lucide-react'

export function ComingSoonSection() {
  return (
    <section className="py-24 bg-black">
      <div className="container mx-auto px-6">
        <div className="text-center mb-16">
          <h2 className="text-4xl font-serif font-bold mb-4">
            On the Roadmap
          </h2>
          <p className="text-xl text-zinc-400">
            Expanding to new platforms and devices
          </p>
        </div>

        <div className="grid md:grid-cols-2 gap-8 max-w-4xl mx-auto">
          {/* Mobile */}
          <div className="bg-gradient-to-br from-zinc-900 to-zinc-950 border border-zinc-800 rounded-2xl p-8 relative overflow-hidden group">
            <div className="absolute top-0 right-0 w-32 h-32 bg-violet-500/10 rounded-full blur-3xl group-hover:bg-violet-500/20 transition-all" />

            <div className="relative">
              <div className="w-16 h-16 bg-zinc-800 rounded-2xl flex items-center justify-center mb-6 group-hover:bg-zinc-700 transition-colors">
                <Smartphone className="w-8 h-8 text-violet-400" />
              </div>

              <h3 className="text-2xl font-bold mb-3">
                Mobile Apps
              </h3>

              <p className="text-zinc-400 mb-4">
                Native iOS and Android apps with offline sync and streaming from your server.
              </p>

              <span className="inline-block text-sm font-mono text-violet-400 bg-violet-950/50 px-3 py-1 rounded-full">
                Coming Soon
              </span>

              <div className="mt-6 text-xs text-zinc-600 font-mono">
                Native mobile experience
              </div>
            </div>
          </div>

          {/* Physical DAP */}
          <div className="bg-gradient-to-br from-zinc-900 to-zinc-950 border border-zinc-800 rounded-2xl p-8 relative overflow-hidden group">
            <div className="absolute top-0 right-0 w-32 h-32 bg-amber-500/10 rounded-full blur-3xl group-hover:bg-amber-500/20 transition-all" />

            <div className="relative">
              <div className="w-16 h-16 bg-zinc-800 rounded-2xl flex items-center justify-center mb-6 group-hover:bg-zinc-700 transition-colors">
                <Radio className="w-8 h-8 text-amber-400" />
              </div>

              <h3 className="text-2xl font-bold mb-3">
                Physical DAP
              </h3>

              <p className="text-zinc-400 mb-4">
                Dedicated digital audio player hardware. High-quality DAC, e-ink display, and offline playback.
              </p>

              <span className="inline-block text-sm font-mono text-amber-400 bg-amber-950/50 px-3 py-1 rounded-full">
                Planned
              </span>

              <div className="mt-6 text-xs text-zinc-600 font-mono">
                High-quality DAC + E-ink display
              </div>
            </div>
          </div>
        </div>

        <div className="text-center mt-12">
          <p className="text-zinc-500 text-sm">
            Want to stay updated?{' '}
            <a href="#newsletter" className="text-violet-400 hover:text-violet-300 transition-colors">
              Join our newsletter
            </a>
          </p>
        </div>
      </div>
    </section>
  )
}
