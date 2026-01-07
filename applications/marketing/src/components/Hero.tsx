'use client'

import { GrainGradient } from './GrainGradient'
import { DownloadButton } from './DownloadButton'
import Link from 'next/link'

export function Hero() {
  return (
    <GrainGradient
      from="#18181b"
      via="#3730a3"
      to="#1e1b4b"
      className="min-h-screen flex items-center justify-center"
    >
      <div className="container mx-auto px-4 sm:px-6 py-12 sm:py-16 md:py-24 text-center">
        <h1 className="font-serif font-extrabold mb-4 sm:mb-6 tracking-tight whitespace-nowrap" style={{ fontSize: 'clamp(1.5rem, 8vw, 6rem)' }}>
          Your Music,
          <br />
          <span className="text-transparent bg-clip-text bg-gradient-to-r from-violet-200 to-violet-400">
            Your Way
          </span>
        </h1>

        <p className="text-base sm:text-lg md:text-xl lg:text-2xl text-violet-200/80 max-w-3xl mx-auto mb-8 sm:mb-12 leading-relaxed px-4">
          Local-first music player,
          <br />
          optional self-hosted multi-user streaming server,
          <br />
          optional paid* discovery
        </p>

        <div className="flex flex-col items-center gap-4 sm:gap-6 mb-6 sm:mb-8">
          <DownloadButton />

          <Link
            href="#demo"
            className="text-violet-200/80 hover:text-violet-200 transition-colors text-xs sm:text-sm"
          >
            See it in action ↓
          </Link>
        </div>

        <div className="mt-12 sm:mt-16 md:mt-24 grid grid-cols-1 sm:grid-cols-3 gap-6 sm:gap-8 max-w-3xl mx-auto text-sm px-2">
          <div>
            <div className="text-2xl sm:text-3xl font-bold text-violet-200 mb-2">Cross-Platform</div>
            <div className="text-violet-200/60 text-xs sm:text-sm">Desktop, Server, Hardware</div>
          </div>
          <div>
            <div className="text-2xl sm:text-3xl font-bold text-violet-200 mb-2">Privacy-First</div>
            <div className="text-violet-200/60 text-xs sm:text-sm">Self-hosted & secure</div>
          </div>
          <div>
            <div className="text-2xl sm:text-3xl font-bold text-violet-200 mb-2">Open Source</div>
            <div className="text-violet-200/60 text-xs sm:text-sm">Community-driven</div>
          </div>
        </div>
      </div>
    </GrainGradient>
  )
}
