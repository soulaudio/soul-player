'use client'

import { Music2, Play, Heart, MoreVertical } from 'lucide-react'

export function PhoneMockup() {
  return (
    <div className="relative mx-auto w-[280px] h-[570px]">
      {/* Phone frame */}
      <div className="absolute inset-0 bg-gradient-to-b from-zinc-800 to-zinc-900 rounded-[3rem] shadow-2xl border-8 border-zinc-950">
        {/* Notch */}
        <div className="absolute top-0 left-1/2 -translate-x-1/2 w-32 h-6 bg-zinc-950 rounded-b-3xl" />

        {/* Screen content */}
        <div className="absolute inset-3 bg-gradient-to-b from-zinc-900 to-black rounded-[2.5rem] overflow-hidden">
          {/* Status bar */}
          <div className="h-12 px-6 pt-3 flex items-start justify-between text-white text-xs">
            <span>9:41</span>
            <div className="flex gap-1">
              <div className="w-4 h-3 bg-white/30 rounded-sm" />
              <div className="w-4 h-3 bg-white/30 rounded-sm" />
              <div className="w-4 h-3 bg-white/30 rounded-sm" />
            </div>
          </div>

          {/* Album art */}
          <div className="mt-8 px-6">
            <div className="aspect-square bg-gradient-to-br from-violet-600/20 via-purple-600/20 to-pink-600/20 rounded-2xl backdrop-blur-xl border border-white/10 flex items-center justify-center">
              <Music2 className="w-20 h-20 text-violet-300/50" />
            </div>
          </div>

          {/* Song info */}
          <div className="mt-6 px-6 text-center">
            <h3 className="text-white font-bold text-base mb-1">Now Playing</h3>
            <p className="text-zinc-400 text-sm">Soul Player Mobile</p>
          </div>

          {/* Progress bar */}
          <div className="mt-6 px-6">
            <div className="h-1 bg-zinc-800 rounded-full overflow-hidden">
              <div className="h-full w-1/3 bg-gradient-to-r from-violet-500 to-purple-500 rounded-full" />
            </div>
            <div className="flex justify-between text-xs text-zinc-500 mt-2">
              <span>1:23</span>
              <span>3:45</span>
            </div>
          </div>

          {/* Controls */}
          <div className="mt-6 px-6 flex items-center justify-center gap-6">
            <Heart className="w-6 h-6 text-zinc-400" />
            <div className="w-14 h-14 rounded-full bg-gradient-to-br from-violet-600 to-purple-600 flex items-center justify-center shadow-lg shadow-violet-500/50">
              <Play className="w-6 h-6 text-white fill-white ml-1" />
            </div>
            <MoreVertical className="w-6 h-6 text-zinc-400" />
          </div>
        </div>
      </div>

      {/* Glow effect */}
      <div className="absolute inset-0 bg-gradient-to-t from-violet-500/20 to-transparent blur-3xl -z-10" />
    </div>
  )
}
