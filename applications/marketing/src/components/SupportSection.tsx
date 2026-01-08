'use client'

import { Smartphone, Music, MessageSquare, FileText, Disc, Search, FileAudio, Fingerprint, Users, Sparkles, Headphones } from 'lucide-react'
import Link from 'next/link'
import { FadeIn } from './animations/FadeIn'
import { PhoneMockup } from './PhoneMockup'

export function SupportSection() {
  return (
    <div className="bg-black">
      {/* Main Header */}
      <section className="py-32 bg-gradient-to-b from-zinc-950 to-black relative overflow-hidden">
        <div className="absolute inset-0 bg-[radial-gradient(ellipse_80%_80%_at_50%_-20%,rgba(120,60,200,0.15),transparent)]" />

        <div className="container mx-auto px-6 relative z-10">
          <FadeIn>
            <div className="text-center max-w-4xl mx-auto">
              <h2 className="text-5xl md:text-7xl font-serif font-bold mb-6 bg-clip-text text-transparent bg-gradient-to-r from-violet-400 via-purple-400 to-pink-400">
                Support Our Vision
              </h2>
              <p className="text-xl md:text-2xl text-zinc-300 leading-relaxed">
                Join our community and help build the best open-source music player
              </p>
            </div>
          </FadeIn>

          {/* Pricing Card */}
          <FadeIn delay={0.2}>
            <div className="mt-16 max-w-3xl mx-auto">
              <div className="relative">
                <div className="absolute inset-0 bg-gradient-to-br from-violet-950/50 to-purple-950/50 rounded-3xl blur-xl" />
                <div className="relative bg-zinc-900/50 backdrop-blur-xl border border-violet-500/20 rounded-3xl p-12">
                  <div className="text-center">
                    <div className="inline-flex items-center gap-2 px-4 py-2 bg-violet-500/10 border border-violet-500/20 rounded-full text-sm text-violet-300 mb-6">
                      <Sparkles className="w-4 h-4" />
                      <span>Community Subscription</span>
                    </div>

                    <div className="mb-8">
                      <div className="text-6xl font-bold mb-2">
                        <span className="bg-clip-text text-transparent bg-gradient-to-r from-violet-400 to-purple-400">
                          €5
                        </span>
                        <span className="text-2xl text-zinc-500">/month</span>
                      </div>
                      <p className="text-zinc-400">
                        or name your price (minimum €5)
                      </p>
                    </div>

                    <Link
                      href="#subscribe"
                      className="inline-flex items-center gap-2 px-10 py-5 bg-gradient-to-r from-violet-600 to-purple-600 hover:from-violet-500 hover:to-purple-500 text-white rounded-full font-semibold text-lg transition-all shadow-lg shadow-violet-500/25 hover:shadow-violet-500/40 hover:scale-105 transform"
                    >
                      Become a Supporter
                    </Link>

                    <p className="text-sm text-zinc-500 mt-6">
                      Cancel anytime • All core features remain free forever
                    </p>
                  </div>
                </div>
              </div>
            </div>
          </FadeIn>
        </div>
      </section>

      {/* Discord Community - Available Now */}
      <section className="py-32 bg-gradient-to-b from-black via-zinc-950 to-black relative overflow-hidden">
        <div className="absolute inset-0 opacity-30">
          <div className="absolute top-1/2 left-1/4 w-96 h-96 bg-violet-500/20 rounded-full blur-3xl" />
          <div className="absolute bottom-1/4 right-1/4 w-96 h-96 bg-purple-500/20 rounded-full blur-3xl" />
        </div>

        <div className="container mx-auto px-6 relative z-10">
          <div className="max-w-6xl mx-auto">
            <FadeIn>
              <div className="grid lg:grid-cols-2 gap-16 items-center">
                {/* Left: Content */}
                <div>
                  <div className="inline-flex items-center gap-2 px-4 py-2 bg-green-500/10 border border-green-500/20 rounded-full text-sm text-green-400 mb-6">
                    <div className="w-2 h-2 bg-green-400 rounded-full animate-pulse" />
                    <span>Available Now</span>
                  </div>

                  <h3 className="text-4xl md:text-5xl font-serif font-bold mb-6">
                    Discord Community
                  </h3>

                  <p className="text-xl text-zinc-300 mb-8 leading-relaxed">
                    Join our exclusive supporter community and connect directly with the development team
                  </p>

                  <div className="space-y-4">
                    <div className="flex items-start gap-4">
                      <div className="w-10 h-10 rounded-full bg-violet-500/10 flex items-center justify-center flex-shrink-0">
                        <MessageSquare className="w-5 h-5 text-violet-400" />
                      </div>
                      <div>
                        <h4 className="font-semibold text-white mb-1">Exclusive Channels</h4>
                        <p className="text-zinc-400">Access supporter-only discussions and behind-the-scenes updates</p>
                      </div>
                    </div>

                    <div className="flex items-start gap-4">
                      <div className="w-10 h-10 rounded-full bg-violet-500/10 flex items-center justify-center flex-shrink-0">
                        <Users className="w-5 h-5 text-violet-400" />
                      </div>
                      <div>
                        <h4 className="font-semibold text-white mb-1">Direct Communication</h4>
                        <p className="text-zinc-400">Chat directly with developers and influence the roadmap</p>
                      </div>
                    </div>

                    <div className="flex items-start gap-4">
                      <div className="w-10 h-10 rounded-full bg-violet-500/10 flex items-center justify-center flex-shrink-0">
                        <Sparkles className="w-5 h-5 text-violet-400" />
                      </div>
                      <div>
                        <h4 className="font-semibold text-white mb-1">Early Announcements</h4>
                        <p className="text-zinc-400">Be the first to know about new features and releases</p>
                      </div>
                    </div>
                  </div>
                </div>

                {/* Right: Visual */}
                <FadeIn delay={0.3} direction="left">
                  <div className="relative">
                    <div className="bg-gradient-to-br from-violet-950/50 to-purple-950/50 rounded-3xl p-12 border border-violet-500/20">
                      <MessageSquare className="w-24 h-24 text-violet-400 mb-6" />
                      <div className="space-y-4">
                        <div className="bg-zinc-900/50 rounded-xl p-4 border border-zinc-800">
                          <div className="flex items-center gap-3 mb-2">
                            <div className="w-8 h-8 rounded-full bg-violet-500/20" />
                            <span className="text-sm text-zinc-300 font-medium">Developer</span>
                          </div>
                          <p className="text-sm text-zinc-400">New mobile app preview dropping tomorrow! 🎉</p>
                        </div>
                        <div className="bg-zinc-900/50 rounded-xl p-4 border border-zinc-800">
                          <div className="flex items-center gap-3 mb-2">
                            <div className="w-8 h-8 rounded-full bg-purple-500/20" />
                            <span className="text-sm text-zinc-300 font-medium">Supporter</span>
                          </div>
                          <p className="text-sm text-zinc-400">Can't wait! Love the progress 💜</p>
                        </div>
                      </div>
                    </div>
                  </div>
                </FadeIn>
              </div>
            </FadeIn>
          </div>
        </div>
      </section>

      {/* Planned Features Header */}
      <section className="py-20 bg-black">
        <div className="container mx-auto px-6">
          <FadeIn>
            <div className="text-center max-w-3xl mx-auto">
              <h2 className="text-4xl md:text-5xl font-serif font-bold mb-4">
                Coming Soon
              </h2>
              <p className="text-xl text-zinc-400">
                Planned features for community supporters
              </p>
            </div>
          </FadeIn>
        </div>
      </section>

      {/* Discovery Features */}
      <section className="py-32 bg-gradient-to-b from-black to-zinc-950 relative overflow-hidden">
        <div className="absolute inset-0 opacity-20">
          <div className="absolute top-1/3 right-1/4 w-96 h-96 bg-blue-500/20 rounded-full blur-3xl" />
        </div>

        <div className="container mx-auto px-6 relative z-10">
          <div className="max-w-6xl mx-auto">
            <FadeIn>
              <div className="grid lg:grid-cols-2 gap-16 items-center">
                {/* Left: Visual */}
                <div className="relative order-2 lg:order-1">
                  <div className="bg-gradient-to-br from-blue-950/30 to-indigo-950/30 rounded-3xl p-12 border border-blue-500/20">
                    <div className="grid grid-cols-2 gap-4">
                      <div className="bg-zinc-900/50 rounded-xl p-6 border border-zinc-800 flex flex-col items-center text-center">
                        <Search className="w-12 h-12 text-blue-400 mb-3" />
                        <p className="text-sm font-medium text-zinc-300">Bandcamp</p>
                        <p className="text-xs text-zinc-500 mt-1">Discovery</p>
                      </div>
                      <div className="bg-zinc-900/50 rounded-xl p-6 border border-zinc-800 flex flex-col items-center text-center">
                        <Disc className="w-12 h-12 text-indigo-400 mb-3" />
                        <p className="text-sm font-medium text-zinc-300">Discogs</p>
                        <p className="text-xs text-zinc-500 mt-1">Database</p>
                      </div>
                      <div className="col-span-2 bg-zinc-900/50 rounded-xl p-6 border border-zinc-800 flex items-center gap-4">
                        <Music className="w-10 h-10 text-violet-400 flex-shrink-0" />
                        <div className="text-left">
                          <p className="text-sm font-medium text-zinc-300 mb-1">Smart Recommendations</p>
                          <p className="text-xs text-zinc-500">AI-powered music suggestions</p>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>

                {/* Right: Content */}
                <div className="order-1 lg:order-2">
                  <div className="inline-flex items-center gap-2 px-4 py-2 bg-amber-500/10 border border-amber-500/20 rounded-full text-sm text-amber-400 mb-6">
                    <span className="text-xs">●</span>
                    <span>Planned</span>
                  </div>

                  <h3 className="text-4xl md:text-5xl font-serif font-bold mb-6">
                    Discovery Features
                  </h3>

                  <p className="text-xl text-zinc-300 mb-8 leading-relaxed">
                    Explore new music with Bandcamp & Discogs integration powered by our community infrastructure
                  </p>

                  <div className="space-y-4">
                    <div className="flex items-start gap-4">
                      <div className="w-10 h-10 rounded-full bg-blue-500/10 flex items-center justify-center flex-shrink-0">
                        <Search className="w-5 h-5 text-blue-400" />
                      </div>
                      <div>
                        <h4 className="font-semibold text-white mb-1">Browse Independent Artists</h4>
                        <p className="text-zinc-400">Direct integration with Bandcamp's catalog</p>
                      </div>
                    </div>

                    <div className="flex items-start gap-4">
                      <div className="w-10 h-10 rounded-full bg-indigo-500/10 flex items-center justify-center flex-shrink-0">
                        <Disc className="w-5 h-5 text-indigo-400" />
                      </div>
                      <div>
                        <h4 className="font-semibold text-white mb-1">Discogs Integration</h4>
                        <p className="text-zinc-400">Access the world's largest music database</p>
                      </div>
                    </div>

                    <div className="flex items-start gap-4">
                      <div className="w-10 h-10 rounded-full bg-violet-500/10 flex items-center justify-center flex-shrink-0">
                        <Sparkles className="w-5 h-5 text-violet-400" />
                      </div>
                      <div>
                        <h4 className="font-semibold text-white mb-1">Smart Recommendations</h4>
                        <p className="text-zinc-400">Discover music based on your listening habits</p>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </FadeIn>
          </div>
        </div>
      </section>

      {/* Lyrics & Metadata */}
      <section className="py-32 bg-gradient-to-b from-zinc-950 to-black relative overflow-hidden">
        <div className="absolute inset-0 opacity-20">
          <div className="absolute top-1/2 left-1/3 w-96 h-96 bg-emerald-500/20 rounded-full blur-3xl" />
        </div>

        <div className="container mx-auto px-6 relative z-10">
          <div className="max-w-6xl mx-auto">
            <FadeIn>
              <div className="grid lg:grid-cols-2 gap-16 items-center">
                {/* Left: Content */}
                <div>
                  <div className="inline-flex items-center gap-2 px-4 py-2 bg-amber-500/10 border border-amber-500/20 rounded-full text-sm text-amber-400 mb-6">
                    <span className="text-xs">●</span>
                    <span>Planned</span>
                  </div>

                  <h3 className="text-4xl md:text-5xl font-serif font-bold mb-6">
                    Lyrics & Metadata
                  </h3>

                  <p className="text-xl text-zinc-300 mb-8 leading-relaxed">
                    Automatic enrichment of your music library with lyrics, album art, and detailed metadata
                  </p>

                  <div className="space-y-4">
                    <div className="flex items-start gap-4">
                      <div className="w-10 h-10 rounded-full bg-emerald-500/10 flex items-center justify-center flex-shrink-0">
                        <FileText className="w-5 h-5 text-emerald-400" />
                      </div>
                      <div>
                        <h4 className="font-semibold text-white mb-1">Synchronized Lyrics</h4>
                        <p className="text-zinc-400">Time-synced lyrics that scroll as you listen</p>
                      </div>
                    </div>

                    <div className="flex items-start gap-4">
                      <div className="w-10 h-10 rounded-full bg-teal-500/10 flex items-center justify-center flex-shrink-0">
                        <FileAudio className="w-5 h-5 text-teal-400" />
                      </div>
                      <div>
                        <h4 className="font-semibold text-white mb-1">Metadata Enhancement</h4>
                        <p className="text-zinc-400">Automatically fill in missing tags and album information</p>
                      </div>
                    </div>

                    <div className="flex items-start gap-4">
                      <div className="w-10 h-10 rounded-full bg-cyan-500/10 flex items-center justify-center flex-shrink-0">
                        <Music className="w-5 h-5 text-cyan-400" />
                      </div>
                      <div>
                        <h4 className="font-semibold text-white mb-1">High-Quality Artwork</h4>
                        <p className="text-zinc-400">Beautiful album covers from multiple sources</p>
                      </div>
                    </div>
                  </div>
                </div>

                {/* Right: Visual */}
                <FadeIn delay={0.3} direction="left">
                  <div className="relative">
                    <div className="bg-gradient-to-br from-emerald-950/30 to-teal-950/30 rounded-3xl p-12 border border-emerald-500/20">
                      <div className="bg-zinc-900/50 rounded-xl p-6 border border-zinc-800 space-y-4">
                        <div className="flex items-center gap-3">
                          <div className="w-16 h-16 bg-gradient-to-br from-emerald-500/20 to-teal-500/20 rounded-lg" />
                          <div className="flex-1">
                            <p className="text-sm font-medium text-zinc-300">Track Title</p>
                            <p className="text-xs text-zinc-500">Artist Name</p>
                          </div>
                        </div>
                        <div className="space-y-2 text-sm text-zinc-400 font-mono">
                          <p className="text-emerald-400">♪ Verse 1</p>
                          <p>First line of lyrics...</p>
                          <p>Second line of lyrics...</p>
                          <p className="text-emerald-400/50">♪ Chorus</p>
                          <p className="text-zinc-600">Upcoming lyrics...</p>
                        </div>
                      </div>
                    </div>
                  </div>
                </FadeIn>
              </div>
            </FadeIn>
          </div>
        </div>
      </section>

      {/* AcoustID Fingerprinting */}
      <section className="py-32 bg-gradient-to-b from-black to-zinc-950 relative overflow-hidden">
        <div className="absolute inset-0 opacity-20">
          <div className="absolute bottom-1/3 right-1/3 w-96 h-96 bg-pink-500/20 rounded-full blur-3xl" />
        </div>

        <div className="container mx-auto px-6 relative z-10">
          <div className="max-w-6xl mx-auto">
            <FadeIn>
              <div className="grid lg:grid-cols-2 gap-16 items-center">
                {/* Left: Visual */}
                <div className="relative order-2 lg:order-1">
                  <div className="bg-gradient-to-br from-pink-950/30 to-rose-950/30 rounded-3xl p-12 border border-pink-500/20">
                    <div className="flex items-center justify-center mb-8">
                      <div className="relative">
                        <Fingerprint className="w-32 h-32 text-pink-400" />
                        <div className="absolute inset-0 bg-pink-500/20 blur-xl rounded-full" />
                      </div>
                    </div>
                    <div className="space-y-3">
                      <div className="bg-zinc-900/50 rounded-lg p-4 border border-zinc-800 flex items-center gap-3">
                        <div className="w-2 h-2 bg-green-400 rounded-full animate-pulse" />
                        <span className="text-sm text-zinc-300">Analyzing audio...</span>
                      </div>
                      <div className="bg-zinc-900/50 rounded-lg p-4 border border-zinc-800 flex items-center gap-3">
                        <div className="w-2 h-2 bg-green-400 rounded-full animate-pulse" />
                        <span className="text-sm text-zinc-300">Matching fingerprint...</span>
                      </div>
                      <div className="bg-zinc-900/50 rounded-lg p-4 border border-zinc-800 flex items-center gap-3">
                        <div className="w-2 h-2 bg-green-400 rounded-full" />
                        <span className="text-sm text-emerald-300 font-medium">Track identified!</span>
                      </div>
                    </div>
                  </div>
                </div>

                {/* Right: Content */}
                <div className="order-1 lg:order-2">
                  <div className="inline-flex items-center gap-2 px-4 py-2 bg-amber-500/10 border border-amber-500/20 rounded-full text-sm text-amber-400 mb-6">
                    <span className="text-xs">●</span>
                    <span>Planned</span>
                  </div>

                  <h3 className="text-4xl md:text-5xl font-serif font-bold mb-6">
                    AcoustID Fingerprinting
                  </h3>

                  <p className="text-xl text-zinc-300 mb-8 leading-relaxed">
                    Advanced audio fingerprinting for automatic track recognition and identification
                  </p>

                  <div className="space-y-4">
                    <div className="flex items-start gap-4">
                      <div className="w-10 h-10 rounded-full bg-pink-500/10 flex items-center justify-center flex-shrink-0">
                        <Fingerprint className="w-5 h-5 text-pink-400" />
                      </div>
                      <div>
                        <h4 className="font-semibold text-white mb-1">Audio Recognition</h4>
                        <p className="text-zinc-400">Identify tracks even with poor or missing metadata</p>
                      </div>
                    </div>

                    <div className="flex items-start gap-4">
                      <div className="w-10 h-10 rounded-full bg-rose-500/10 flex items-center justify-center flex-shrink-0">
                        <Search className="w-5 h-5 text-rose-400" />
                      </div>
                      <div>
                        <h4 className="font-semibold text-white mb-1">Duplicate Detection</h4>
                        <p className="text-zinc-400">Find and merge duplicate tracks in your library</p>
                      </div>
                    </div>

                    <div className="flex items-start gap-4">
                      <div className="w-10 h-10 rounded-full bg-purple-500/10 flex items-center justify-center flex-shrink-0">
                        <Sparkles className="w-5 h-5 text-purple-400" />
                      </div>
                      <div>
                        <h4 className="font-semibold text-white mb-1">Automatic Tagging</h4>
                        <p className="text-zinc-400">Populate metadata from recognized fingerprints</p>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </FadeIn>
          </div>
        </div>
      </section>

      {/* Mobile Apps */}
      <section className="py-32 bg-gradient-to-b from-zinc-950 to-black relative overflow-hidden">
        <div className="absolute inset-0 opacity-20">
          <div className="absolute top-1/2 left-1/2 w-[800px] h-[800px] bg-violet-500/20 rounded-full blur-3xl -translate-x-1/2 -translate-y-1/2" />
        </div>

        <div className="container mx-auto px-6 relative z-10">
          <div className="max-w-6xl mx-auto">
            <FadeIn>
              <div className="text-center mb-16">
                <div className="inline-flex items-center gap-2 px-4 py-2 bg-amber-500/10 border border-amber-500/20 rounded-full text-sm text-amber-400 mb-6">
                  <span className="text-xs">●</span>
                  <span>Planned</span>
                </div>

                <h3 className="text-4xl md:text-5xl font-serif font-bold mb-6">
                  Mobile Apps
                </h3>

                <p className="text-xl text-zinc-300 max-w-2xl mx-auto leading-relaxed">
                  Native iOS and Android apps with offline sync and streaming from your server
                </p>
              </div>

              <div className="grid lg:grid-cols-2 gap-16 items-start">
                {/* Left: Phone Mockup */}
                <div className="flex justify-center">
                  <FadeIn delay={0.2} direction="up">
                    <PhoneMockup />
                  </FadeIn>
                </div>

                {/* Right: Features */}
                <div className="space-y-8">
                  <div className="space-y-6">
                    <div className="flex items-start gap-4">
                      <div className="w-12 h-12 rounded-xl bg-violet-500/10 flex items-center justify-center flex-shrink-0">
                        <Smartphone className="w-6 h-6 text-violet-400" />
                      </div>
                      <div>
                        <h4 className="text-lg font-semibold text-white mb-2">Native Performance</h4>
                        <p className="text-zinc-400">Built with native technologies for smooth, responsive experience</p>
                      </div>
                    </div>

                    <div className="flex items-start gap-4">
                      <div className="w-12 h-12 rounded-xl bg-purple-500/10 flex items-center justify-center flex-shrink-0">
                        <Music className="w-6 h-6 text-purple-400" />
                      </div>
                      <div>
                        <h4 className="text-lg font-semibold text-white mb-2">Offline Playback</h4>
                        <p className="text-zinc-400">Download tracks for offline listening anywhere</p>
                      </div>
                    </div>

                    <div className="flex items-start gap-4">
                      <div className="w-12 h-12 rounded-xl bg-pink-500/10 flex items-center justify-center flex-shrink-0">
                        <Headphones className="w-6 h-6 text-pink-400" />
                      </div>
                      <div>
                        <h4 className="text-lg font-semibold text-white mb-2">Server Streaming</h4>
                        <p className="text-zinc-400">Stream your entire library from your self-hosted server</p>
                      </div>
                    </div>
                  </div>

                  {/* Cost Explanation */}
                  <div className="bg-zinc-900/50 border border-zinc-800 rounded-2xl p-6">
                    <h4 className="text-sm font-bold text-zinc-300 mb-3 flex items-center gap-2">
                      <span>💰</span> Why Support Helps
                    </h4>
                    <p className="text-sm text-zinc-400 leading-relaxed">
                      Mobile app development requires significant resources: maintaining multiple device form factors,
                      OS versions, app store subscriptions (Apple Developer: $99/year, Google Play: $25 one-time),
                      and ongoing testing infrastructure. Your support helps us deliver high-quality mobile experiences.
                    </p>
                  </div>
                </div>
              </div>
            </FadeIn>
          </div>
        </div>
      </section>

      {/* Physical DAP */}
      <section id="physical-dap" className="py-32 bg-gradient-to-b from-black to-zinc-950 relative overflow-hidden">
        <div className="absolute inset-0 opacity-20">
          <div className="absolute top-1/3 left-1/4 w-96 h-96 bg-amber-500/20 rounded-full blur-3xl" />
        </div>

        <div className="container mx-auto px-6 relative z-10">
          <div className="max-w-6xl mx-auto">
            <FadeIn>
              <div className="text-center mb-16">
                <div className="inline-flex items-center gap-2 px-4 py-2 bg-amber-500/10 border border-amber-500/20 rounded-full text-sm text-amber-400 mb-6">
                  <span className="text-xs">●</span>
                  <span>Planned</span>
                </div>

                <h3 className="text-4xl md:text-5xl font-serif font-bold mb-6">
                  Physical DAP
                </h3>

                <p className="text-xl text-zinc-300 max-w-2xl mx-auto leading-relaxed mb-12">
                  Dedicated digital audio player hardware with audiophile-grade components
                </p>

                {/* Feature Grid */}
                <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 max-w-4xl mx-auto">
                  <FadeIn delay={0.1}>
                    <div className="bg-zinc-900/50 border border-zinc-800 rounded-2xl p-8 hover:border-white/10 transition-all group">
                      <div className="w-16 h-16 mx-auto mb-4 bg-zinc-800/50 rounded-xl flex items-center justify-center group-hover:scale-110 transition-transform">
                        <FileText className="w-8 h-8 text-zinc-500" />
                      </div>
                      <h4 className="text-lg font-bold text-zinc-400 mb-2">E-ink Display</h4>
                      <p className="text-sm text-zinc-600">Low power, outdoor readable</p>
                    </div>
                  </FadeIn>

                  <FadeIn delay={0.2}>
                    <div className="bg-zinc-900/50 border border-zinc-800 rounded-2xl p-8 hover:border-amber/10 transition-all group">
                      <div className="w-16 h-16 mx-auto mb-4 bg-zinc-800/50 rounded-xl flex items-center justify-center group-hover:scale-110 transition-transform">
                        <Music className="w-8 h-8 text-zinc-500" />
                      </div>
                      <h4 className="text-lg font-bold text-zinc-400 mb-2">Hi-Fi DAC</h4>
                      <p className="text-sm text-zinc-600">Audiophile-grade sound</p>
                    </div>
                  </FadeIn>

                  <FadeIn delay={0.3}>
                    <div className="bg-zinc-900/50 border border-zinc-800 rounded-2xl p-8 hover:border-violet/10 transition-all group">
                      <div className="w-16 h-16 mx-auto mb-4 bg-zinc-800/50 rounded-xl flex items-center justify-center group-hover:scale-110 transition-transform">
                        <Disc className="w-8 h-8 text-zinc-500" />
                      </div>
                      <h4 className="text-lg font-bold text-zinc-400 mb-2">SD Card</h4>
                      <p className="text-sm text-zinc-600">Expandable storage</p>
                    </div>
                  </FadeIn>

                  <FadeIn delay={0.4}>
                    <div className="bg-zinc-900/50 border border-zinc-800 rounded-2xl p-8 hover:border-green/10 transition-all group">
                      <div className="w-16 h-16 mx-auto mb-4 bg-zinc-800/50 rounded-xl flex items-center justify-center group-hover:scale-110 transition-transform">
                        <div className="w-6 h-6 rounded-full bg-zinc-600 border-4 border-zinc-700" />
                      </div>
                      <h4 className="text-lg font-bold text-zinc-400 mb-2">3.5mm Jack</h4>
                      <p className="text-sm text-zinc-600">Universal compatibility</p>
                    </div>
                  </FadeIn>
                </div>

                <div className="mt-12 bg-zinc-900/30 border border-zinc-800 rounded-xl p-6 max-w-2xl mx-auto">
                  <p className="text-sm text-zinc-500">
                    Hardware specifications and design are in early planning stages. Final features may vary.
                  </p>
                </div>
              </div>
            </FadeIn>
          </div>
        </div>
      </section>

      {/* Newsletter CTA */}
      <section className="py-32 bg-gradient-to-b from-zinc-950 to-black">
        <div className="container mx-auto px-6">
          <FadeIn>
            <div className="max-w-3xl mx-auto text-center">
              <div className="relative">
                <div className="absolute inset-0 bg-gradient-to-r from-violet-500/10 to-purple-500/10 rounded-3xl blur-2xl" />
                <div className="relative bg-zinc-900/50 backdrop-blur-xl border border-zinc-800 rounded-3xl p-12">
                  <h3 className="text-3xl md:text-4xl font-serif font-bold mb-4">
                    Stay Updated
                  </h3>
                  <p className="text-lg text-zinc-400 mb-8">
                    Get notified about mobile apps, physical DAP development, and new features
                  </p>
                  <Link
                    href="#newsletter"
                    className="inline-flex items-center gap-2 px-8 py-4 bg-zinc-800 hover:bg-zinc-700 text-white rounded-full font-semibold transition-all"
                  >
                    Join Newsletter
                  </Link>
                </div>
              </div>
            </div>
          </FadeIn>
        </div>
      </section>
    </div>
  )
}
