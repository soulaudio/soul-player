'use client'

import { Check, X, Minus } from 'lucide-react'

type Feature = {
  name: string
  soulPlayer: boolean | 'partial'
  spotify: boolean | 'partial'
  navidrome: boolean | 'partial'
  plex: boolean | 'partial'
}

const FEATURES: Feature[] = [
  {
    name: 'Self-hosted / Local-first',
    soulPlayer: true,
    spotify: false,
    navidrome: true,
    plex: true,
  },
  {
    name: 'No subscription required',
    soulPlayer: true,
    spotify: false,
    navidrome: true,
    plex: true,
  },
  {
    name: 'Multi-user support',
    soulPlayer: true,
    spotify: true,
    navidrome: true,
    plex: true,
  },
  {
    name: 'Desktop app (native)',
    soulPlayer: true,
    spotify: true,
    navidrome: false,
    plex: true,
  },
  {
    name: 'Audio effects & EQ',
    soulPlayer: true,
    spotify: 'partial',
    navidrome: false,
    plex: 'partial',
  },
  {
    name: 'Multi-source library',
    soulPlayer: true,
    spotify: false,
    navidrome: false,
    plex: 'partial',
  },
  {
    name: 'Hardware player support',
    soulPlayer: 'partial',
    spotify: false,
    navidrome: false,
    plex: false,
  },
  {
    name: 'Open source',
    soulPlayer: true,
    spotify: false,
    navidrome: true,
    plex: false,
  },
  {
    name: 'Privacy-focused',
    soulPlayer: true,
    spotify: false,
    navidrome: true,
    plex: true,
  },
]

function FeatureIcon({ value }: { value: boolean | 'partial' }) {
  if (value === true) {
    return <Check className="w-5 h-5 text-green-500" />
  }
  if (value === 'partial') {
    return <Minus className="w-5 h-5 text-yellow-500" />
  }
  return <X className="w-5 h-5 text-zinc-600" />
}

export function ComparisonTable() {
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse">
        <thead>
          <tr className="border-b border-zinc-800">
            <th className="text-left py-4 px-4 font-semibold text-zinc-400 text-sm">Feature</th>
            <th className="py-4 px-4 font-bold text-violet-400">Soul Player</th>
            <th className="py-4 px-4 font-semibold text-zinc-400 text-sm">Spotify</th>
            <th className="py-4 px-4 font-semibold text-zinc-400 text-sm">Navidrome</th>
            <th className="py-4 px-4 font-semibold text-zinc-400 text-sm">Plex</th>
          </tr>
        </thead>
        <tbody>
          {FEATURES.map((feature, i) => (
            <tr key={i} className="border-b border-zinc-800/50 hover:bg-zinc-900/30 transition-colors">
              <td className="py-3 px-4 text-sm">{feature.name}</td>
              <td className="py-3 px-4 text-center">
                <div className="flex justify-center">
                  <FeatureIcon value={feature.soulPlayer} />
                </div>
              </td>
              <td className="py-3 px-4 text-center">
                <div className="flex justify-center">
                  <FeatureIcon value={feature.spotify} />
                </div>
              </td>
              <td className="py-3 px-4 text-center">
                <div className="flex justify-center">
                  <FeatureIcon value={feature.navidrome} />
                </div>
              </td>
              <td className="py-3 px-4 text-center">
                <div className="flex justify-center">
                  <FeatureIcon value={feature.plex} />
                </div>
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      <div className="mt-4 text-xs text-zinc-500 space-y-1">
        <p><Check className="w-3 h-3 inline text-green-500" /> Full support</p>
        <p><Minus className="w-3 h-3 inline text-yellow-500" /> Partial or limited support</p>
        <p><X className="w-3 h-3 inline text-zinc-600" /> Not available</p>
      </div>
    </div>
  )
}
