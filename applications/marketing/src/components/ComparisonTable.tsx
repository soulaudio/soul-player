'use client'

import { Check, X, Minus } from 'lucide-react'
import React, { useState, useRef } from 'react'

type FeatureValue = {
  value: boolean | 'partial'
  tooltip: string
}

type Feature = {
  name: string
  description: string
  soulPlayer: FeatureValue
  foobar2000: FeatureValue
  musicbee: FeatureValue
  strawberry: FeatureValue
  roon: FeatureValue
  navidrome: FeatureValue
  jellyfin: FeatureValue
}

const FEATURES: Feature[] = [
  {
    name: 'Cross-platform',
    description: 'Native apps for Windows, macOS, and Linux',
    soulPlayer: {
      value: true,
      tooltip: 'Native desktop app for Windows, macOS, and Linux',
    },
    foobar2000: {
      value: false,
      tooltip: 'Windows only. Can run on Linux/macOS via Wine but not officially supported',
    },
    musicbee: {
      value: false,
      tooltip: 'Windows only. No official macOS or Linux support',
    },
    strawberry: {
      value: true,
      tooltip: 'Native builds available for Windows, macOS, and Linux',
    },
    roon: {
      value: true,
      tooltip: 'Native apps for Windows, macOS, Linux, iOS, and Android',
    },
    navidrome: {
      value: true,
      tooltip: 'Server runs anywhere. Web UI works in any browser. Subsonic-compatible clients available',
    },
    jellyfin: {
      value: true,
      tooltip: 'Server runs on Windows, macOS, Linux. Native clients for most platforms',
    },
  },
  {
    name: 'Open source',
    description: 'Source code freely available under open license',
    soulPlayer: {
      value: true,
      tooltip: 'AGPL-3.0 licensed. Full source available on GitHub',
    },
    foobar2000: {
      value: false,
      tooltip: 'Proprietary freeware. SDK available for plugins but core is closed source',
    },
    musicbee: {
      value: false,
      tooltip: 'Proprietary freeware. Closed source with plugin API',
    },
    strawberry: {
      value: true,
      tooltip: 'GPL v3 licensed. Fork of Clementine with active development',
    },
    roon: {
      value: false,
      tooltip: 'Proprietary software. Requires subscription ($15/month or $830 lifetime)',
    },
    navidrome: {
      value: true,
      tooltip: 'GPL v3 licensed. Active community development',
    },
    jellyfin: {
      value: true,
      tooltip: 'GPL v2 licensed. Community-driven fork of Emby',
    },
  },
  {
    name: 'Self-hosted streaming',
    description: 'Run your own server to stream music anywhere',
    soulPlayer: {
      value: 'partial',
      tooltip: 'Coming soon. Server component for multi-user streaming is in active development',
    },
    foobar2000: {
      value: false,
      tooltip: 'Local playback only. No server component',
    },
    musicbee: {
      value: false,
      tooltip: 'Local playback only. Has DLNA/UPnP but no remote streaming',
    },
    strawberry: {
      value: false,
      tooltip: 'Local playback only. Can connect to Subsonic/Tidal but cannot serve',
    },
    roon: {
      value: true,
      tooltip: 'Roon Server streams to clients. Roon ARC available for remote access',
    },
    navidrome: {
      value: true,
      tooltip: 'Purpose-built streaming server. Supports transcoding and multiple users',
    },
    jellyfin: {
      value: true,
      tooltip: 'Full media server with music streaming, transcoding, and multi-user support',
    },
  },
  {
    name: 'Bit-perfect output',
    description: 'Unaltered audio data sent to DAC',
    soulPlayer: {
      value: 'partial',
      tooltip: 'Coming soon. Exclusive mode output for bit-perfect playback in development',
    },
    foobar2000: {
      value: true,
      tooltip: 'WASAPI exclusive and ASIO output available. Industry standard for bit-perfect',
    },
    musicbee: {
      value: true,
      tooltip: 'WASAPI exclusive mode and ASIO support',
    },
    strawberry: {
      value: true,
      tooltip: 'Direct output on Linux, WASAPI on Windows. Supports exclusive mode',
    },
    roon: {
      value: true,
      tooltip: 'Signal path display shows all processing. Bit-perfect streaming to endpoints',
    },
    navidrome: {
      value: 'partial',
      tooltip: 'Can serve original files, but bit-perfect depends on client app',
    },
    jellyfin: {
      value: 'partial',
      tooltip: 'Can direct stream without transcoding. Depends on client implementation',
    },
  },
  {
    name: 'Parametric EQ',
    description: 'Multi-band equalizer with precise frequency control',
    soulPlayer: {
      value: 'partial',
      tooltip: 'Basic implementation available. Advanced parametric EQ features coming soon',
    },
    foobar2000: {
      value: true,
      tooltip: 'Multiple EQ plugins available including parametric options',
    },
    musicbee: {
      value: true,
      tooltip: '15-band graphic EQ built-in. VST plugins supported for parametric',
    },
    strawberry: {
      value: true,
      tooltip: '10-band equalizer built-in',
    },
    roon: {
      value: true,
      tooltip: 'Powerful parametric EQ with unlimited bands, filters, and convolution',
    },
    navidrome: {
      value: false,
      tooltip: 'No server-side EQ. Must be handled by client',
    },
    jellyfin: {
      value: false,
      tooltip: 'No audio processing. EQ must be handled by client',
    },
  },
  {
    name: 'DSP effects chain',
    description: 'Compressor, limiter, and other audio effects',
    soulPlayer: {
      value: true,
      tooltip: 'Built-in compressor, limiter, and expandable effects slots',
    },
    foobar2000: {
      value: true,
      tooltip: 'Extensive DSP manager with built-in and third-party components. VST support',
    },
    musicbee: {
      value: true,
      tooltip: 'DSP effects panel with built-in effects. Full VST/VST3 support',
    },
    strawberry: {
      value: 'partial',
      tooltip: 'Basic effects available. Limited compared to dedicated audio players',
    },
    roon: {
      value: true,
      tooltip: 'DSP engine with headroom management, crossfeed, speaker setup, convolution',
    },
    navidrome: {
      value: false,
      tooltip: 'No audio processing on server',
    },
    jellyfin: {
      value: false,
      tooltip: 'Transcoding only, no DSP effects',
    },
  },
  {
    name: 'ReplayGain',
    description: 'Volume normalization across tracks and albums',
    soulPlayer: {
      value: 'partial',
      tooltip: 'Coming soon. ReplayGain tag reading and EBU R128 normalization in development',
    },
    foobar2000: {
      value: true,
      tooltip: 'Full ReplayGain scanner and playback. Can write tags to files',
    },
    musicbee: {
      value: true,
      tooltip: 'ReplayGain scanning and playback. Track and album modes supported',
    },
    strawberry: {
      value: true,
      tooltip: 'ReplayGain tag reading and volume adjustment',
    },
    roon: {
      value: true,
      tooltip: 'Volume leveling with R128 analysis. Track or album modes',
    },
    navidrome: {
      value: 'partial',
      tooltip: 'Reads ReplayGain tags for compatible clients. No server-side normalization',
    },
    jellyfin: {
      value: 'partial',
      tooltip: 'Can read ReplayGain tags. Implementation varies by client',
    },
  },
  {
    name: 'Gapless playback',
    description: 'Seamless transitions between tracks',
    soulPlayer: {
      value: 'partial',
      tooltip: 'Coming soon. Gapless playback and crossfade in development',
    },
    foobar2000: {
      value: true,
      tooltip: 'Excellent gapless support. Industry reference implementation',
    },
    musicbee: {
      value: true,
      tooltip: 'Gapless playback with optional crossfade',
    },
    strawberry: {
      value: true,
      tooltip: 'Gapless playback supported',
    },
    roon: {
      value: true,
      tooltip: 'Seamless gapless playback with signal path optimization',
    },
    navidrome: {
      value: 'partial',
      tooltip: 'Depends on client. Some Subsonic clients support gapless, others don\'t',
    },
    jellyfin: {
      value: 'partial',
      tooltip: 'Client-dependent. Web player has gaps, native clients vary',
    },
  },
  {
    name: 'ASIO support',
    description: 'Low-latency audio driver (Windows)',
    soulPlayer: {
      value: 'partial',
      tooltip: 'Coming soon. ASIO backend for lowest latency output planned',
    },
    foobar2000: {
      value: true,
      tooltip: 'ASIO output available. Widely used by audiophiles',
    },
    musicbee: {
      value: true,
      tooltip: 'ASIO output driver support built-in',
    },
    strawberry: {
      value: 'partial',
      tooltip: 'ASIO support on Windows available',
    },
    roon: {
      value: true,
      tooltip: 'Full ASIO support with device clocking options',
    },
    navidrome: {
      value: false,
      tooltip: 'Web-based. Audio output handled by browser or client app',
    },
    jellyfin: {
      value: false,
      tooltip: 'Server-side only. Audio output depends on client',
    },
  },
  {
    name: 'Multi-library support',
    description: 'Manage music from multiple folders/sources',
    soulPlayer: {
      value: true,
      tooltip: 'Multiple source folders supported. Designed for multi-source libraries',
    },
    foobar2000: {
      value: true,
      tooltip: 'Media Library can monitor multiple folders',
    },
    musicbee: {
      value: true,
      tooltip: 'Multiple watched folders with inbox support',
    },
    strawberry: {
      value: 'partial',
      tooltip: 'Can add multiple collection folders',
    },
    roon: {
      value: true,
      tooltip: 'Multiple storage locations, NAS support, streaming service integration',
    },
    navidrome: {
      value: false,
      tooltip: 'Single music folder per instance. Multiple folders require workarounds',
    },
    jellyfin: {
      value: true,
      tooltip: 'Multiple music libraries with separate folder configurations',
    },
  },
  {
    name: 'Hardware player',
    description: 'Dedicated hardware device support',
    soulPlayer: {
      value: 'partial',
      tooltip: 'Dedicated hardware player in development. Will sync with desktop library',
    },
    foobar2000: {
      value: false,
      tooltip: 'Desktop software only',
    },
    musicbee: {
      value: false,
      tooltip: 'Desktop software only. Can sync to portable devices',
    },
    strawberry: {
      value: false,
      tooltip: 'Desktop software only',
    },
    roon: {
      value: true,
      tooltip: 'Roon Ready certification for streamers and DACs. Large hardware ecosystem',
    },
    navidrome: {
      value: false,
      tooltip: 'Server software only. Limited Subsonic-compatible hardware exists',
    },
    jellyfin: {
      value: false,
      tooltip: 'Software only. No dedicated hardware players',
    },
  },
  {
    name: 'Active development',
    description: 'Regular updates and new features',
    soulPlayer: {
      value: true,
      tooltip: 'Actively developed with regular updates and new features planned',
    },
    foobar2000: {
      value: 'partial',
      tooltip: 'Mature and stable. Updates less frequent but still maintained',
    },
    musicbee: {
      value: true,
      tooltip: 'Actively maintained with regular updates',
    },
    strawberry: {
      value: true,
      tooltip: 'Active open source development',
    },
    roon: {
      value: true,
      tooltip: 'Actively developed with regular feature updates',
    },
    navidrome: {
      value: true,
      tooltip: 'Very active development with frequent releases',
    },
    jellyfin: {
      value: true,
      tooltip: 'Active community development with regular updates',
    },
  },
  {
    name: 'Free to use',
    description: 'No payment required for core features',
    soulPlayer: {
      value: true,
      tooltip: 'Completely free and open source. Optional paid discovery service planned',
    },
    foobar2000: {
      value: true,
      tooltip: 'Freeware. Mobile version is paid ($7)',
    },
    musicbee: {
      value: true,
      tooltip: 'Freeware with optional donation',
    },
    strawberry: {
      value: true,
      tooltip: 'Free and open source',
    },
    roon: {
      value: false,
      tooltip: 'Subscription required: $15/month or $830 lifetime',
    },
    navidrome: {
      value: true,
      tooltip: 'Free and open source. Self-hosted, no fees',
    },
    jellyfin: {
      value: true,
      tooltip: 'Free and open source. No premium tier',
    },
  },
]

const PLAYERS = [
  { key: 'soulPlayer', name: 'Soul Player', highlight: true, url: null },
  { key: 'foobar2000', name: 'foobar2000', highlight: false, url: 'https://www.foobar2000.org/' },
  { key: 'musicbee', name: 'MusicBee', highlight: false, url: 'https://getmusicbee.com/' },
  { key: 'strawberry', name: 'Strawberry', highlight: false, url: 'https://www.strawberrymusicplayer.org/' },
  { key: 'roon', name: 'Roon', highlight: false, url: 'https://roon.app/' },
  { key: 'navidrome', name: 'Navidrome', highlight: false, url: 'https://www.navidrome.org/' },
  { key: 'jellyfin', name: 'Jellyfin', highlight: false, url: 'https://jellyfin.org/' },
] as const

function Tooltip({ content, children }: { content: string; children: React.ReactNode }) {
  const [show, setShow] = useState(false)
  const [position, setPosition] = useState({ top: 0, left: 0 })
  const triggerRef = useRef<HTMLDivElement>(null)

  const handleMouseEnter = () => {
    if (triggerRef.current) {
      const rect = triggerRef.current.getBoundingClientRect()
      const tooltipWidth = 256 // w-64 = 16rem = 256px

      // Calculate left position, keeping tooltip within viewport
      let left = rect.left + rect.width / 2 - tooltipWidth / 2
      left = Math.max(8, Math.min(left, window.innerWidth - tooltipWidth - 8))

      setPosition({
        top: rect.top - 8, // 8px gap above the trigger
        left,
      })
    }
    setShow(true)
  }

  return (
    <div
      ref={triggerRef}
      className="inline-flex justify-center"
      onMouseEnter={handleMouseEnter}
      onMouseLeave={() => setShow(false)}
    >
      {children}
      {show && (
        <div
          className="fixed px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg shadow-xl text-xs text-zinc-200 w-64 z-[100] pointer-events-none transform -translate-y-full"
          style={{ top: position.top, left: position.left }}
        >
          {content}
        </div>
      )}
    </div>
  )
}

function FeatureIcon({ data }: { data: FeatureValue }) {
  const icon = data.value === true ? (
    <Check className="w-5 h-5 text-green-500" />
  ) : data.value === 'partial' ? (
    <Minus className="w-5 h-5 text-yellow-500" />
  ) : (
    <X className="w-5 h-5 text-zinc-600" />
  )

  return (
    <Tooltip content={data.tooltip}>
      <button className="p-1 rounded hover:bg-zinc-800/50 transition-colors cursor-help">
        {icon}
      </button>
    </Tooltip>
  )
}

export function ComparisonTable() {
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-sm">
        <thead>
          <tr className="border-b border-zinc-800">
            <th className="text-left py-4 px-3 font-semibold text-zinc-400 min-w-[160px]">
              Feature
            </th>
            {PLAYERS.map((player) => (
              <th
                key={player.key}
                className={`py-4 px-2 text-center ${
                  player.highlight ? 'font-bold text-violet-400' : 'font-semibold text-zinc-400'
                }`}
              >
                {player.url ? (
                  <a
                    href={player.url}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="hover:underline"
                  >
                    {player.name}
                  </a>
                ) : (
                  player.name
                )}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {FEATURES.map((feature, i) => (
            <tr
              key={i}
              className="border-b border-zinc-800/50 hover:bg-zinc-900/30 transition-colors"
            >
              <td className="py-3 px-3">
                <Tooltip content={feature.description}>
                  <span className="cursor-help border-b border-dotted border-zinc-600">
                    {feature.name}
                  </span>
                </Tooltip>
              </td>
              {PLAYERS.map((player) => (
                <td key={player.key} className="py-3 px-2 text-center">
                  <div className="flex justify-center">
                    <FeatureIcon data={feature[player.key]} />
                  </div>
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>

      <div className="mt-6 flex flex-wrap gap-6 text-xs text-zinc-500">
        <div className="flex items-center gap-1.5">
          <Check className="w-4 h-4 text-green-500" />
          <span>Full support</span>
        </div>
        <div className="flex items-center gap-1.5">
          <Minus className="w-4 h-4 text-yellow-500" />
          <span>Partial or limited</span>
        </div>
        <div className="flex items-center gap-1.5">
          <X className="w-4 h-4 text-zinc-600" />
          <span>Not available</span>
        </div>
        <div className="flex items-center gap-1.5 ml-auto text-zinc-600">
          <span>Hover icons for details</span>
        </div>
      </div>

      <p className="mt-4 text-xs text-zinc-600">
        foobar2000 and MusicBee are Windows-only desktop players. Strawberry is cross-platform (Clementine fork).
        Roon is a premium audiophile solution. Navidrome and Jellyfin are self-hosted media servers.
      </p>
    </div>
  )
}
