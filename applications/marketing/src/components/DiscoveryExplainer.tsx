import Link from 'next/link'

export function DiscoveryExplainer() {
  return (
    <section id="discovery" className="py-24 bg-zinc-950">
      <div className="container mx-auto px-6">
        <div className="max-w-4xl mx-auto">
          {/* Header */}
          <div className="text-center mb-16">
            <h2 className="text-4xl md:text-5xl font-serif font-bold mb-4">
              Why Is Discovery Paid?
            </h2>
            <p className="text-xl text-zinc-400">
              Supporting sustainable open-source development
            </p>
          </div>

          {/* Explanation */}
          <div className="space-y-8 mb-16">
            <div className="bg-zinc-900/50 border border-zinc-800 rounded-xl p-8">
              <h3 className="text-xl font-bold mb-4 flex items-center gap-2">
                <span className="text-violet-400">$</span>
                External Services Cost Money
              </h3>
              <p className="text-zinc-400 leading-relaxed">
                Our discovery features integrate with services like Bandcamp, Discogs, AcoustID, and lyrics providers.
                These services charge for API access based on usage. While we optimize every request to minimize costs,
                serving thousands of users requires substantial API expenses.
              </p>
            </div>

            <div className="bg-zinc-900/50 border border-zinc-800 rounded-xl p-8">
              <h3 className="text-xl font-bold mb-4 flex items-center gap-2">
                <span className="text-violet-400">⚙️</span>
                Server Infrastructure Isn't Free
              </h3>
              <p className="text-zinc-400 leading-relaxed">
                Even though we've engineered Soul Player to be incredibly efficient, running servers for metadata
                enhancement, lyrics fetching, and fingerprinting requires infrastructure. Database storage, compute
                resources, and bandwidth all add up when serving a growing community.
              </p>
            </div>

            <div className="bg-zinc-900/50 border border-zinc-800 rounded-xl p-8">
              <h3 className="text-xl font-bold mb-4 flex items-center gap-2">
                <span className="text-violet-400">❤️</span>
                Sustainable Open Source
              </h3>
              <p className="text-zinc-400 leading-relaxed">
                Soul Player is and will always remain open source. The core player, self-hosted server, and all
                essential features are completely free. Optional discovery services allow those who want enhanced
                features to support ongoing development while keeping the project sustainable long-term.
              </p>
            </div>
          </div>

          {/* CTA Section */}
          <div className="relative">
            {/* Gradient background */}
            <div className="absolute inset-0 bg-gradient-to-br from-violet-950/30 to-purple-950/30 rounded-2xl" />
            <div className="grain absolute inset-0 rounded-2xl opacity-10" />

            <div className="relative border border-violet-500/20 rounded-2xl p-12 text-center">
              <h3 className="text-3xl font-serif font-bold mb-4">
                Join the Community
              </h3>
              <p className="text-lg text-zinc-300 mb-8 max-w-2xl mx-auto">
                Support Soul Player's development and unlock premium features
              </p>

              {/* Benefits Grid */}
              <div className="grid md:grid-cols-2 gap-4 mb-10 text-left max-w-2xl mx-auto">
                <div className="flex items-start gap-3">
                  <div className="w-5 h-5 rounded-full bg-violet-500/20 flex items-center justify-center flex-shrink-0 mt-0.5">
                    <span className="text-violet-400 text-xs">✓</span>
                  </div>
                  <div>
                    <p className="font-medium text-white">Discovery Features</p>
                    <p className="text-sm text-zinc-400">Bandcamp & Discogs integration</p>
                  </div>
                </div>

                <div className="flex items-start gap-3">
                  <div className="w-5 h-5 rounded-full bg-violet-500/20 flex items-center justify-center flex-shrink-0 mt-0.5">
                    <span className="text-violet-400 text-xs">✓</span>
                  </div>
                  <div>
                    <p className="font-medium text-white">Lyrics & Metadata</p>
                    <p className="text-sm text-zinc-400">Automatic enhancement</p>
                  </div>
                </div>

                <div className="flex items-start gap-3">
                  <div className="w-5 h-5 rounded-full bg-violet-500/20 flex items-center justify-center flex-shrink-0 mt-0.5">
                    <span className="text-violet-400 text-xs">✓</span>
                  </div>
                  <div>
                    <p className="font-medium text-white">Discord Community</p>
                    <p className="text-sm text-zinc-400">Supporter role & channels</p>
                  </div>
                </div>

                <div className="flex items-start gap-3">
                  <div className="w-5 h-5 rounded-full bg-violet-500/20 flex items-center justify-center flex-shrink-0 mt-0.5">
                    <span className="text-violet-400 text-xs">✓</span>
                  </div>
                  <div>
                    <p className="font-medium text-white">AcoustID Fingerprinting</p>
                    <p className="text-sm text-zinc-400">Advanced track recognition</p>
                  </div>
                </div>

                <div className="flex items-start gap-3">
                  <div className="w-5 h-5 rounded-full bg-violet-500/20 flex items-center justify-center flex-shrink-0 mt-0.5">
                    <span className="text-violet-400 text-xs">✓</span>
                  </div>
                  <div>
                    <p className="font-medium text-white">Early Access</p>
                    <p className="text-sm text-zinc-400">New features first</p>
                  </div>
                </div>

                <div className="flex items-start gap-3">
                  <div className="w-5 h-5 rounded-full bg-violet-500/20 flex items-center justify-center flex-shrink-0 mt-0.5">
                    <span className="text-violet-400 text-xs">✓</span>
                  </div>
                  <div>
                    <p className="font-medium text-white">Support Development</p>
                    <p className="text-sm text-zinc-400">Help us stay sustainable</p>
                  </div>
                </div>
              </div>

              {/* CTA Button */}
              <Link
                href="#subscribe"
                className="inline-flex items-center gap-2 px-8 py-4 bg-violet-600 hover:bg-violet-500 text-white rounded-full font-semibold transition-all shadow-lg hover:shadow-violet-500/25"
              >
                Become a Supporter
                <span className="text-sm opacity-75">from $5/month</span>
              </Link>

              <p className="text-sm text-zinc-500 mt-6">
                Cancel anytime. All core features remain free forever.
              </p>
            </div>
          </div>
        </div>
      </div>
    </section>
  )
}
