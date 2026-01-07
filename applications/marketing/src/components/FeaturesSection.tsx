'use client'

import { Music, Layers, Lock, Wand2, Globe, Server } from 'lucide-react'

const FEATURES = [
  {
    icon: Layers,
    title: 'Multi-Source Support',
    description: 'Play from local files, network shares, or your personal server. One library, multiple sources.',
    technical: 'Trait-based architecture supports local, streaming, and network audio sources',
  },
  {
    icon: Music,
    title: 'Advanced Effects Chain',
    description: 'Parametric EQ, dynamic compression, and custom effects. Professional audio quality.',
    technical: 'Real-time DSP with zero-allocation audio processing callbacks',
    comingSoon: true,
  },
  {
    icon: Server,
    title: 'Multi-User from Day 1',
    description: 'Personal libraries, shared playlists, user authentication. Built for families and teams.',
    technical: 'SQLite schema designed for multi-tenancy across all platforms',
  },
  {
    icon: Globe,
    title: 'Cross-Platform Native',
    description: 'Desktop (Windows, macOS, Linux), server streaming, and embedded hardware. Seamless experience everywhere.',
    technical: 'Native desktop apps, containerized server, and dedicated hardware support',
  },
  {
    icon: Lock,
    title: 'Privacy-First, Self-Hosted',
    description: 'Your music, your server, your data. No tracking, no subscriptions, no cloud lock-in.',
    technical: 'Local-first architecture with optional server sync',
  },
  {
    icon: Wand2,
    title: 'Optional Discovery Service',
    description: 'Bandcamp & Discogs integration, metadata enhancement, lyrics, AcoustID fingerprinting.',
    technical: 'Join our community subscription for enhanced discovery features',
    comingSoon: false,
    subscription: true,
  },
]

export function FeaturesSection() {
  return (
    <section className="py-24 bg-zinc-950">
      <div className="container mx-auto px-6">
        <div className="text-center mb-16">
          <h2 className="text-5xl font-serif font-bold mb-4">
            Why Soul Player?
          </h2>
          <p className="text-xl text-zinc-400 max-w-2xl mx-auto">
            Built for music lovers who value control, quality, and privacy
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
                  COMING SOON
                </span>
              )}
              {feature.subscription && (
                <span className="absolute top-4 right-4 text-xs font-mono text-amber-400 bg-amber-950/50 px-2 py-1 rounded">
                  OPTIONAL
                </span>
              )}

              <feature.icon className="w-10 h-10 text-violet-400 mb-4" />

              <h3 className="text-xl font-bold mb-2">
                {feature.title}
              </h3>

              <p className="text-zinc-400 mb-4">
                {feature.description}
              </p>

              <details className="text-xs text-zinc-500 group">
                <summary className="cursor-pointer font-mono hover:text-violet-400 transition-colors">
                  Technical details →
                </summary>
                <p className="mt-2 text-zinc-500 font-mono">
                  {feature.technical}
                </p>
              </details>
            </div>
          ))}
        </div>
      </div>
    </section>
  )
}
