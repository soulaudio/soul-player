'use client'

import { FadeIn } from './animations/FadeIn'
import Link from 'next/link'

export function WhySoulPlayer() {
  return (
    <section className="py-32 bg-gradient-to-b from-zinc-950 via-black to-zinc-950 relative overflow-hidden">
      {/* Background effects */}
      <div className="absolute inset-0 opacity-20">
        <div className="absolute top-1/4 left-1/3 w-96 h-96 bg-violet-500/30 rounded-full blur-3xl" />
        <div className="absolute bottom-1/3 right-1/3 w-96 h-96 bg-purple-500/30 rounded-full blur-3xl" />
      </div>

      <div className="container mx-auto px-6 relative z-10">
        <FadeIn>
          <div className="text-center mb-20">
            <h2 className="text-5xl md:text-6xl font-serif font-bold mb-6">
              Why Soul Player?
            </h2>
            <p className="text-xl text-zinc-400 max-w-2xl mx-auto font-light">
              For those who cherish their music
            </p>
          </div>
        </FadeIn>

        {/* Bento Grid - 3 columns max, square cells */}
        <div className="max-w-7xl mx-auto">
          <div className="grid grid-cols-3 gap-4 auto-rows-[200px]">

            {/* Privacy - 2×1 */}
            <FadeIn delay={0.1} className="col-span-2 row-span-1">
              <div className="h-full bg-gradient-to-br from-violet-950/30 to-purple-950/30 backdrop-blur-xl border border-violet-500/10 rounded-xl p-8 hover:border-violet-500/30 transition-all relative overflow-hidden group">
                <div className="absolute top-0 right-0 w-40 h-40 bg-violet-500/10 rounded-full blur-3xl group-hover:bg-violet-500/15 transition-all duration-700" />

                <div className="relative h-full flex flex-col justify-center">
                  <h3 className="text-4xl md:text-5xl font-serif font-light mb-4 tracking-tight leading-none">
                    Privacy
                  </h3>
                  <p className="text-zinc-400 text-base md:text-lg font-light leading-relaxed">
                    Your listening habits remain yours alone
                  </p>
                </div>
              </div>
            </FadeIn>

            {/* Local First - 1x1 */}
            <FadeIn delay={0.2} className="col-span-1 row-span-1">
              <div className="h-full bg-gradient-to-br from-purple-950/20 to-fuchsia-950/20 backdrop-blur-xl border border-purple-500/10 rounded-lg p-5 hover:border-purple-500/30 transition-all relative overflow-hidden group">
                <div className="relative h-full flex flex-col justify-center">
                  <h3 className="text-lg md:text-xl font-serif font-light tracking-tight leading-tight">
                    Local<br/>First
                  </h3>
                </div>
              </div>
            </FadeIn>

            {/* Multi-User - 1x1 */}
            <FadeIn delay={0.3} className="col-span-1 row-span-1">
              <div className="h-full bg-gradient-to-br from-emerald-950/20 to-teal-950/20 backdrop-blur-xl border border-emerald-500/10 rounded-lg p-5 hover:border-emerald-500/30 transition-all relative overflow-hidden group">
                <div className="relative h-full flex flex-col justify-center">
                  <h3 className="text-lg md:text-xl font-serif font-light tracking-tight leading-tight">
                    Multi-<br/>User
                  </h3>
                </div>
              </div>
            </FadeIn>

            {/* Free Forever - 1x1 */}
            <FadeIn delay={0.4} className="col-span-1 row-span-1">
              <div className="h-full bg-gradient-to-br from-teal-950/20 to-cyan-950/20 backdrop-blur-xl border border-teal-500/10 rounded-lg p-5 hover:border-teal-500/30 transition-all relative overflow-hidden group">
                <div className="relative h-full flex flex-col justify-center">
                  <h3 className="text-lg md:text-xl font-serif font-light tracking-tight leading-tight">
                    Free<br/>Forever
                  </h3>
                </div>
              </div>
            </FadeIn>

            {/* Open Source - 2×1 */}
            <FadeIn delay={0.5} className="col-span-2 row-span-1">
              <div className="h-full bg-gradient-to-br from-pink-950/30 to-rose-950/30 backdrop-blur-xl border border-pink-500/10 rounded-xl p-8 hover:border-pink-500/30 transition-all relative overflow-hidden group">
                <div className="absolute -bottom-10 -right-10 w-40 h-40 bg-pink-500/10 rounded-full blur-3xl" />

                <div className="relative h-full flex flex-col justify-center">
                  <h3 className="text-3xl md:text-4xl font-serif font-light tracking-tight leading-tight">
                    Open Source
                  </h3>
                  <p className="text-zinc-500 text-sm font-light mt-2">
                    Transparent & auditable
                  </p>
                </div>
              </div>
            </FadeIn>

            {/* Discovery - 2×1 */}
            <FadeIn delay={0.6} className="col-span-2 row-span-1">
              <Link href="#subscribe">
                <div className="h-full bg-gradient-to-br from-blue-950/30 to-indigo-950/30 backdrop-blur-xl border border-blue-500/10 rounded-xl p-8 hover:border-blue-500/40 transition-all relative overflow-hidden group cursor-pointer">
                  <div className="absolute -right-10 -bottom-10 w-40 h-40 bg-blue-500/10 rounded-full blur-3xl group-hover:scale-110 transition-transform duration-700" />

                  <div className="relative h-full flex flex-col justify-between">
                    <div className="flex items-start justify-between">
                      <h3 className="text-3xl md:text-4xl font-serif font-light tracking-tight leading-tight">
                        Discovery
                      </h3>
                      <div className="text-blue-400/50 group-hover:text-blue-400 group-hover:translate-x-2 transition-all">
                        <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M9 5l7 7-7 7" />
                        </svg>
                      </div>
                    </div>
                    <p className="text-xs text-zinc-500 font-light">Optional • Coming Soon</p>
                  </div>
                </div>
              </Link>
            </FadeIn>

            {/* Physical Device - 1x1 */}
            <FadeIn delay={0.7} className="col-span-1 row-span-1">
              <Link href="#physical-dap">
                <div className="h-full bg-gradient-to-br from-amber-950/30 to-orange-950/30 backdrop-blur-xl border border-amber-500/10 rounded-lg p-5 hover:border-amber-500/40 transition-all relative overflow-hidden group cursor-pointer">
                  {/* 3D Device hint */}
                  <div className="absolute right-1 bottom-1 w-12 h-20 opacity-10 group-hover:opacity-20 transition-opacity duration-700"
                       style={{
                         transform: 'perspective(300px) rotateY(-20deg) rotateX(5deg)',
                         transformStyle: 'preserve-3d'
                       }}>
                    <div className="w-full h-full bg-gradient-to-br from-amber-500/40 to-orange-500/40 rounded-md backdrop-blur-sm border border-amber-500/30" />
                  </div>

                  <div className="relative h-full flex flex-col justify-between">
                    <h3 className="text-lg md:text-xl font-serif font-light tracking-tight leading-tight">
                      Physical<br/>Device
                    </h3>
                    <p className="text-[10px] text-zinc-600 font-light">Planned</p>
                  </div>
                </div>
              </Link>
            </FadeIn>

            {/* Cross-Platform - 1x1 */}
            <FadeIn delay={0.8} className="col-span-1 row-span-1">
              <div className="h-full bg-gradient-to-br from-slate-950/20 to-zinc-950/20 backdrop-blur-xl border border-slate-500/10 rounded-lg p-5 hover:border-slate-500/30 transition-all relative overflow-hidden group">
                <div className="relative h-full flex flex-col justify-center">
                  <h3 className="text-lg md:text-xl font-serif font-light tracking-tight leading-tight">
                    Cross-<br/>Platform
                  </h3>
                </div>
              </div>
            </FadeIn>

            {/* Advanced Audio - 3-wide, 1-tall */}
            <FadeIn delay={0.9} className="col-span-3 row-span-1">
              <div className="h-full bg-gradient-to-br from-indigo-950/20 to-blue-950/20 backdrop-blur-xl border border-indigo-500/10 rounded-lg p-6 hover:border-indigo-500/30 transition-all relative overflow-hidden group">
                <div className="relative h-full flex items-center justify-between">
                  <h3 className="text-2xl md:text-3xl font-serif font-light tracking-tight">
                    Advanced Audio
                  </h3>
                  <p className="text-xs md:text-sm text-zinc-600 font-light">Coming Soon</p>
                </div>
              </div>
            </FadeIn>

          </div>
        </div>

        {/* Bottom Message */}
        <FadeIn delay={1.0}>
          <div className="mt-24 text-center">
            <p className="text-2xl text-zinc-400 font-serif font-light italic">
              Built with care for music lovers
            </p>
          </div>
        </FadeIn>
      </div>
    </section>
  )
}
